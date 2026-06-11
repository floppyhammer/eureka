use crate::render::camera::CameraUniform;
use crate::render::render_graph::{standard_resources, FrameContext, Node, SamplerKey, TextureKey};
use crate::render::vertex::{VertexBuffer, VertexSky};
use crate::render::{create_render_pipeline, Texture};
use std::any::Any;

pub struct SkyboxNode {
    pipeline_layout: Option<wgpu::PipelineLayout>,
    pipeline: Option<wgpu::RenderPipeline>,
}

impl Default for SkyboxNode {
    fn default() -> Self {
        Self {
            pipeline_layout: None,
            pipeline: None,
        }
    }
}

impl Node for SkyboxNode {
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn node_resources(&self) -> crate::render::render_graph::resource::NodeResources {
        use crate::render::render_graph::resource::{ResourceSpec, TextureKey};
        use crate::render::render_graph::standard_resources;
        use crate::render::Texture;

        let color_spec = ResourceSpec::Texture(TextureKey {
            width: 0,
            height: 0,
            format: Some(wgpu::TextureFormat::Rgba16Float),
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            layers: 1,
        });
        let depth_spec = ResourceSpec::Texture(TextureKey {
            width: 0,
            height: 0,
            format: Some(Texture::DEPTH_FORMAT),
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            layers: 1,
        });

        let buffer_size = CameraUniform::get_uniform_offset_unit() * crate::render::render_graph::nodes::prepare_view::MAX_CAMERAS;

        crate::render::render_graph::resource::NodeResources::new()
            .input(
                standard_resources::camera_buffer(),
                ResourceSpec::buffer(buffer_size as u64, wgpu::BufferUsages::UNIFORM),
            )
            .output(standard_resources::main_color(), color_spec)
            .output(standard_resources::main_depth(), depth_spec)
    }

    fn prepare(&mut self, context: &mut FrameContext) {
        if self.pipeline.is_some() {
            return;
        }

        let device = &context.render_context.device;

        let sky_texture_bind_group_layout = {
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::Cube,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler {
                            0: wgpu::SamplerBindingType::Filtering,
                        },
                        count: None,
                    },
                ],
                label: Some("Sky Texture Bind Group Layout"),
            })
        };

        let camera_bind_group_layout = context
            .pool
            .get_bind_group_layout("camera_bind_group_layout")
            .unwrap()
            .clone();

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Skybox Pipeline Layout"),
            bind_group_layouts: &[&camera_bind_group_layout, &sky_texture_bind_group_layout],
            push_constant_ranges: &[],
        });

        let shader = wgpu::ShaderModuleDescriptor {
            label: Some("Skybox Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../../../shaders/skybox.wgsl").into()),
        };

        let pipeline = create_render_pipeline(
            device,
            &pipeline_layout,
            Some(wgpu::TextureFormat::Rgba16Float),
            Some(Texture::DEPTH_FORMAT),
            &[VertexSky::desc()],
            shader,
            "Skybox Pipeline",
            false,
            Some(wgpu::Face::Back),
        );

        self.pipeline_layout = Some(pipeline_layout);
        self.pipeline = Some(pipeline);
        context
            .pool
            .add_bind_group_layout("skybox_bind_group_layout", sky_texture_bind_group_layout);
    }

    fn run(&mut self, context: &mut FrameContext) {
        if context.render_world.extracted.sky.is_none() {
            return;
        }

        let main_color = context.texture(&standard_resources::main_color());
        let main_depth = context.texture(&standard_resources::main_depth());

        let sampler = context.get_sampler(SamplerKey {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        // 1. 克隆所有需要的静态资源句柄，以断开对 context 的借用
        let sky_texture_bind_group_layout = context
            .pool
            .get_bind_group_layout("skybox_bind_group_layout")
            .unwrap()
            .clone();

        let camera_bind_group_layout = context
            .pool
            .get_bind_group_layout("camera_bind_group_layout")
            .unwrap()
            .clone();

        let imported_resources = context.render_world.sky_imported_resources.clone();
        let sky_texture = context
            .render_world
            .imported_texture_cache
            .get(
                imported_resources
                    .texture
                    .expect("Sky texture not found in imported cache"),
            )
            .unwrap()
            .clone();

        // 2. 创建 BindGroups (现在 context 可以被安全地以 mut 方式借用)
        let sky_texture_bind_group = context.create_bind_group(
            &sky_texture_bind_group_layout,
            vec![sky_texture.id],
            |ctx| {
                ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    layout: &sky_texture_bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(&sky_texture.view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::Sampler(&sampler),
                        },
                    ],
                    label: Some("Skybox Texture Bind Group"),
                })
            },
        );

        let buffer_key = context.render_world.extracted.cameras.get_buffer_key();
        let camera_buffer =
            context.get_buffer_by_id(&standard_resources::camera_buffer(), buffer_key);

        let camera_bind_group =
            context.create_bind_group(&camera_bind_group_layout, vec![camera_buffer.id], |ctx| {
                ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    layout: &camera_bind_group_layout,
                    entries: &[wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                            buffer: &camera_buffer.buffer,
                            offset: 0,
                            size: Some(
                                wgpu::BufferSize::new(size_of::<CameraUniform>() as u64).unwrap(),
                            ),
                        }),
                    }],
                    label: Some("Camera Bind Group"),
                })
            });

        // 3. 执行渲染
        if let Some(mesh) = imported_resources.mesh {
            let mut render_pass = context
                .encoder
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Skybox Render Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &main_color.view,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                        view: &main_depth.view,
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        }),
                        stencil_ops: None,
                    }),
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });

            let mesh_allocator = &context.render_world.mesh_render_resources.mesh_allocator;

            render_pass.set_pipeline(self.pipeline.as_ref().unwrap());
            render_pass.set_vertex_buffer(0, mesh_allocator.sky_vertex_buffer.slice(..));
            render_pass.set_index_buffer(
                mesh_allocator.sky_index_buffer.slice(..),
                wgpu::IndexFormat::Uint32,
            );
            render_pass.set_bind_group(0, &camera_bind_group, &[0]);
            render_pass.set_bind_group(1, &sky_texture_bind_group, &[]);
            render_pass.draw_indexed(0..mesh.index_count, 0, 0..1);
        }
    }
}
