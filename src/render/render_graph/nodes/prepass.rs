use crate::render::camera::{CameraType, CameraUniform};
use crate::render::render_backend::PreparedFrame;
use crate::render::render_graph::{
    standard_resources, FrameContext, Node,
};
use crate::render::vertex::{Vertex3d, VertexBuffer};
use crate::render::{create_render_pipeline_with_entry, InstanceRaw, Texture};
use std::any::Any;
use wgpu::BufferAddress;

pub struct PrePassNode {
    pipeline: Option<wgpu::RenderPipeline>,
}

impl Default for PrePassNode {
    fn default() -> Self {
        Self { pipeline: None }
    }
}

impl Node for PrePassNode {
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn node_resources(
        &self,
        prepared: &PreparedFrame,
    ) -> crate::render::render_graph::resource::NodeResources {
        use crate::render::render_graph::resource::{NodeResources, ResourceSpec, TextureKey};
        use crate::render::render_graph::standard_resources;

        let camera_buffer_size = CameraUniform::get_uniform_offset_unit()
            * crate::render::render_graph::nodes::prepare_view::MAX_CAMERAS;

        NodeResources::new()
            .input(
                standard_resources::camera_buffer(),
                ResourceSpec::buffer(camera_buffer_size as u64, wgpu::BufferUsages::UNIFORM),
            )
            .input(
                standard_resources::cull_visible_instance_buffer(),
                ResourceSpec::buffer(
                    prepared.instance_buffer_size as BufferAddress,
                    wgpu::BufferUsages::STORAGE
                        | wgpu::BufferUsages::VERTEX
                        | wgpu::BufferUsages::COPY_DST,
                ),
            )
            .input(
                standard_resources::cull_indirect_buffer(),
                ResourceSpec::buffer(
                    prepared.indirect_buffer_size as BufferAddress,
                    wgpu::BufferUsages::STORAGE
                        | wgpu::BufferUsages::INDIRECT
                        | wgpu::BufferUsages::COPY_DST,
                ),
            )
            .input(
                standard_resources::material_storage_buffer(),
                ResourceSpec::buffer(0, wgpu::BufferUsages::STORAGE),
            )
            .output(
                standard_resources::main_depth(),
                ResourceSpec::Texture(TextureKey {
                    width: 0,
                    height: 0,
                    format: Some(Texture::DEPTH_FORMAT),
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                        | wgpu::TextureUsages::TEXTURE_BINDING,
                    layers: 1,
                    mip_levels: 1,
                    dimension: wgpu::TextureDimension::D2,
                }),
            )
            .output(
                standard_resources::prepass_normal(),
                ResourceSpec::Texture(TextureKey {
                    width: 0,
                    height: 0,
                    format: Some(wgpu::TextureFormat::Rgba16Float),
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                        | wgpu::TextureUsages::TEXTURE_BINDING,
                    layers: 1,
                    mip_levels: 1,
                    dimension: wgpu::TextureDimension::D2,
                }),
            )
            .output(
                standard_resources::prepass_velocity(),
                ResourceSpec::Texture(TextureKey {
                    width: 0,
                    height: 0,
                    format: Some(wgpu::TextureFormat::Rg16Float),
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                        | wgpu::TextureUsages::TEXTURE_BINDING,
                    layers: 1,
                    mip_levels: 1,
                    dimension: wgpu::TextureDimension::D2,
                }),
            )
    }

    fn run(&mut self, context: &mut FrameContext) {
        if context.prepared.instance_buffer_size == 0 {
            return;
        }

        let device = &context.render_context.device;

        if self.pipeline.is_none() {
            let camera_layout = context.backend.get_bind_group_layout("camera_bind_group_layout").unwrap().clone();
            let bindless_layout = context.backend.get_bind_group_layout("bindless_bind_group_layout").unwrap().clone();

            let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("PrePass Pipeline Layout"),
                bind_group_layouts: &[Some(&camera_layout), Some(&bindless_layout)],
                immediate_size: 0,
            });

            let source = include_str!("../../../shaders/prepass.wgsl")
                .replace("#import eureka::camera::Camera", crate::render::camera::CAMERA_STRUCT_WGSL);

            let shader = wgpu::ShaderModuleDescriptor {
                label: Some("PrePass Shader"),
                source: wgpu::ShaderSource::Wgsl(source.into()),
            };

            // MRT Pipeline: Location 0 is Normal, Location 1 is Velocity
            self.pipeline = Some(device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("PrePass Pipeline"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &device.create_shader_module(shader),
                    entry_point: Some("vs_main"),
                    compilation_options: Default::default(),
                    buffers: &[Vertex3d::desc(), InstanceRaw::desc()],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &device.create_shader_module(wgpu::ShaderModuleDescriptor {
                        label: None,
                        source: wgpu::ShaderSource::Wgsl(include_str!("../../../shaders/prepass.wgsl").replace("#import eureka::camera::Camera", crate::render::camera::CAMERA_STRUCT_WGSL).into()),
                    }),
                    entry_point: Some("fs_main"),
                    compilation_options: Default::default(),
                    targets: &[
                        Some(wgpu::ColorTargetState {
                            format: wgpu::TextureFormat::Rgba16Float,
                            blend: None,
                            write_mask: wgpu::ColorWrites::ALL,
                        }),
                        Some(wgpu::ColorTargetState {
                            format: wgpu::TextureFormat::Rg16Float,
                            blend: None,
                            write_mask: wgpu::ColorWrites::ALL,
                        }),
                    ],
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: Some(wgpu::Face::Back),
                    polygon_mode: wgpu::PolygonMode::Fill,
                    unclipped_depth: false,
                    conservative: false,
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: Texture::DEPTH_FORMAT,
                    depth_write_enabled: Some(true),
                    depth_compare: Some(wgpu::CompareFunction::LessEqual),
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
                multisample: wgpu::MultisampleState::default(),
                cache: None,
                multiview_mask: None,
            }));
        }

        let main_depth = context.texture(&standard_resources::main_depth());
        let normal_tex = context.texture(&standard_resources::prepass_normal());
        let velocity_tex = context.texture(&standard_resources::prepass_velocity());

        let camera_buffer = context.buffer(&standard_resources::camera_buffer());
        let camera_layout = context.backend.get_bind_group_layout("camera_bind_group_layout").unwrap().clone();
        let camera_bg = context.create_bind_group("camera_bind_group_layout", vec![camera_buffer.id], |ctx| {
            ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &camera_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera_buffer.buffer.as_entire_binding(),
                }],
                label: None,
            })
        });

        let materials_storage_buffer = context.buffer(&standard_resources::material_storage_buffer());
        let bindless_bg = context.get_bind_group("bindless_bind_group_layout", vec![materials_storage_buffer.id]).clone();

        let mesh_allocator = context.backend.imported_mesh_allocator.read().unwrap();
        let visible_instances = context.buffer(&standard_resources::cull_visible_instance_buffer());
        let indirect_buffer = context.buffer(&standard_resources::cull_indirect_buffer());

        {
            let mut render_pass = context.encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("PrePass Render Pass"),
                color_attachments: &[
                    Some(wgpu::RenderPassColorAttachment {
                        view: &normal_tex.view,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                            store: wgpu::StoreOp::Store,
                        },
                    }),
                    Some(wgpu::RenderPassColorAttachment {
                        view: &velocity_tex.view,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                            store: wgpu::StoreOp::Store,
                        },
                    }),
                ],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &main_depth.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            // 目前只针对 3D 相机
            for (camera_idx, cam_type) in context.extracted.cameras.types.iter().enumerate() {
                if *cam_type == CameraType::D3 {
                    let offset = camera_idx as u32 * CameraUniform::get_uniform_offset_unit();
                    render_pass.set_pipeline(self.pipeline.as_ref().unwrap());
                    render_pass.set_bind_group(0, &camera_bg, &[offset]);
                    render_pass.set_bind_group(1, &bindless_bg, &[]);
                    render_pass.set_vertex_buffer(0, mesh_allocator.vertex_buffer.slice(..));
                    render_pass.set_vertex_buffer(1, visible_instances.buffer.slice(..));
                    render_pass.set_index_buffer(mesh_allocator.index_buffer.slice(..), wgpu::IndexFormat::Uint32);

                    if !context.prepared.draw_counts.is_empty() {
                        render_pass.multi_draw_indexed_indirect(&indirect_buffer.buffer, 0, context.prepared.draw_counts[0]);
                    }
                }
            }
        }
    }
}
