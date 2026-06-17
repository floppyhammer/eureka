use crate::render::camera::{CameraType, CameraUniform};
use crate::render::light::{CascadeUniform, LightUniform, MAX_SHADOWED_POINT_LIGHTS};
use crate::render::render_backend::PreparedFrame;
use crate::render::render_graph::standard_resources;
use crate::render::render_graph::{FrameContext, Node, TextureKey};
use crate::render::vertex::{Vertex3d, VertexBuffer};
use crate::render::{create_render_pipeline, InstanceRaw, Texture};
use std::any::Any;
use wgpu::BufferAddress;

pub struct TransparentMeshNode {
    pipeline: Option<wgpu::RenderPipeline>,
}

impl Default for TransparentMeshNode {
    fn default() -> Self {
        Self { pipeline: None }
    }
}

impl Node for TransparentMeshNode {
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn node_resources(
        &self,
        prepared: &PreparedFrame,
    ) -> crate::render::render_graph::resource::NodeResources {
        use crate::render::render_graph::resource::{ResourceSpec, TextureKey};
        use crate::render::render_graph::standard_resources;
        use crate::render::Texture;

        let camera_buffer_size = CameraUniform::get_uniform_offset_unit()
            * crate::render::render_graph::nodes::prepare_view::MAX_CAMERAS;

        let material_buffer_size = prepared.material_uniforms.len()
            * size_of::<crate::render::material::MaterialUniform>();

        // 计算透明实例缓冲区大小
        let transparent_instance_buffer_size =
            (prepared.sorted_transparent_instances.len() * size_of::<InstanceRaw>()) as u64;

        crate::render::render_graph::resource::NodeResources::new()
            .input(
                standard_resources::camera_buffer(),
                ResourceSpec::buffer(camera_buffer_size as u64, wgpu::BufferUsages::UNIFORM),
            )
            .input(
                standard_resources::shadow_cascade_buffer(),
                ResourceSpec::buffer(
                    size_of::<CascadeUniform>() as BufferAddress,
                    wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                ),
            )
            .internal(
                standard_resources::transparent_instance_buffer(),
                ResourceSpec::buffer(
                    transparent_instance_buffer_size.max(64), // 至少分配一点空间
                    wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                ),
            )
            .input(
                standard_resources::material_storage_buffer(),
                ResourceSpec::buffer(
                    material_buffer_size as u64,
                    wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                ),
            )
            .optional_input(
                standard_resources::point_shadow_map(),
                ResourceSpec::Texture(TextureKey {
                    width: 512,
                    height: 512,
                    format: Some(Texture::DEPTH_FORMAT),
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                        | wgpu::TextureUsages::TEXTURE_BINDING,
                    layers: (MAX_SHADOWED_POINT_LIGHTS * 6) as u32,
                    dimension: wgpu::TextureDimension::D2,
                }),
            )
            .optional_input(
                standard_resources::directional_shadow_map(),
                ResourceSpec::Texture(TextureKey {
                    width: 2048,
                    height: 2048,
                    format: Some(Texture::DEPTH_FORMAT),
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                        | wgpu::TextureUsages::TEXTURE_BINDING,
                    layers: crate::render::light::NUM_CASCADES as u32,
                    dimension: wgpu::TextureDimension::D2,
                }),
            )
            .optional_input(
                standard_resources::ssao_blur(),
                ResourceSpec::Texture(TextureKey {
                    width: 0,
                    height: 0,
                    format: Some(wgpu::TextureFormat::R8Unorm),
                    usage: wgpu::TextureUsages::TEXTURE_BINDING,
                    layers: 1,
                    dimension: wgpu::TextureDimension::D2,
                }),
            )
            .output(
                standard_resources::main_color(),
                ResourceSpec::Texture(TextureKey {
                    width: 0,
                    height: 0,
                    format: Some(wgpu::TextureFormat::Rgba16Float),
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                        | wgpu::TextureUsages::TEXTURE_BINDING,
                    layers: 1,
                    dimension: wgpu::TextureDimension::D2,
                }),
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
                    dimension: wgpu::TextureDimension::D2,
                }),
            )
            .input(
                standard_resources::light_uniform_buffer(),
                ResourceSpec::buffer(
                    size_of::<LightUniform>() as u64,
                    wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                ),
            )
            .input(
                standard_resources::point_light_storage_buffer(),
                ResourceSpec::buffer(
                    (size_of::<crate::render::light::PointLightUniform>() * 1024) as u64,
                    wgpu::BufferUsages::STORAGE,
                ),
            )
            .input(
                standard_resources::light_grid_buffer(),
                ResourceSpec::buffer(0, wgpu::BufferUsages::STORAGE),
            )
            .input(
                standard_resources::light_index_list_buffer(),
                ResourceSpec::buffer(0, wgpu::BufferUsages::STORAGE),
            )
            .input(
                standard_resources::cluster_config_buffer(),
                ResourceSpec::buffer(0, wgpu::BufferUsages::UNIFORM),
            )
            .input(
                standard_resources::volumetric_lighting_texture(),
                ResourceSpec::Texture(TextureKey {
                    width: crate::render::light::CLUSTER_GRID_SIZE[0],
                    height: crate::render::light::CLUSTER_GRID_SIZE[1],
                    layers: crate::render::light::CLUSTER_GRID_SIZE[2],
                    format: Some(wgpu::TextureFormat::Rgba16Float),
                    usage: wgpu::TextureUsages::TEXTURE_BINDING,
                    dimension: wgpu::TextureDimension::D3,
                }),
            )
    }

    fn run(&mut self, context: &mut FrameContext) {
        if context.prepared.transparent_draw_batches.is_empty() {
            return;
        }

        let device = &context.render_context.device;
        let queue = &context.render_context.queue;
        let extracted = context.extracted;

        // 1. 上传排好序的实例数据
        let transparent_instance_buffer =
            context.buffer(&standard_resources::transparent_instance_buffer());
        queue.write_buffer(
            &transparent_instance_buffer.buffer,
            0,
            bytemuck::cast_slice(&context.prepared.sorted_transparent_instances),
        );

        let camera_bind_group_layout = context
            .backend
            .get_bind_group_layout("camera_bind_group_layout")
            .unwrap()
            .clone();

        let light_bind_group_layout = context
            .backend
            .get_bind_group_layout("light_bind_group_layout")
            .unwrap()
            .clone();

        if self.pipeline.is_none() {
            let bindless_bind_group_layout = context
                .backend
                .get_bind_group_layout("bindless_bind_group_layout")
                .unwrap();

            let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Transparent Mesh Layout"),
                bind_group_layouts: &[
                    &camera_bind_group_layout,
                    &light_bind_group_layout,
                    &bindless_bind_group_layout,
                ],
                push_constant_ranges: &[],
            });

            let shader = wgpu::ShaderModuleDescriptor {
                label: Some("Transparent Mesh Shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("../../../shaders/mesh.wgsl").into()),
            };

            self.pipeline = Some(create_render_pipeline(
                device,
                &pipeline_layout,
                Some(wgpu::TextureFormat::Rgba16Float),
                Some(Texture::DEPTH_FORMAT),
                &[Vertex3d::desc(), InstanceRaw::desc()],
                shader,
                "Transparent Mesh Bindless",
                true, // 开起 Alpha 混合
                Some(wgpu::Face::Back),
            ));
        }

        let main_color = FrameContext::texture(context, &standard_resources::main_color());
        let main_depth = context.texture(&standard_resources::main_depth());
        let ssao_blur = context.texture(&standard_resources::ssao_blur());
        let directional_shadow_map = context.texture(&standard_resources::directional_shadow_map());
        let point_shadow_map = context.texture(&standard_resources::point_shadow_map());

        let point_shadow_map_view = {
            point_shadow_map.get_view(&wgpu::TextureViewDescriptor {
                label: Some("point shadow map view"),
                format: Some(Texture::DEPTH_FORMAT),
                dimension: Some(wgpu::TextureViewDimension::CubeArray),
                aspect: wgpu::TextureAspect::DepthOnly,
                array_layer_count: Some(MAX_SHADOWED_POINT_LIGHTS as u32 * 6),
                ..Default::default()
            })
        };

        let camera_buffer = context.buffer(&standard_resources::camera_buffer());
        let camera_bind_group = context
            .get_bind_group("camera_bind_group_layout", vec![camera_buffer.id])
            .clone();

        let cascade_view = directional_shadow_map.get_view(&wgpu::TextureViewDescriptor {
            label: Some("shadow cascade view"),
            format: Some(Texture::DEPTH_FORMAT),
            dimension: Some(wgpu::TextureViewDimension::D2Array),
            usage: Some(wgpu::TextureUsages::TEXTURE_BINDING),
            aspect: wgpu::TextureAspect::DepthOnly,
            array_layer_count: Some(crate::render::light::NUM_CASCADES as u32),
            ..Default::default()
        });

        let (sky_view, _) = if let Some(id) = context.backend.sky_imported_resources.texture {
            let texture_cache = context.backend.imported_texture_cache.read().unwrap();
            let tex = texture_cache.get(id).unwrap();
            (tex.view.clone(), tex.view_id)
        } else {
            (context.backend.dummy_cube_view.clone(), 0)
        };

        let volumetric_tex = context.texture(&standard_resources::volumetric_lighting_texture());
        let volumetric_view = volumetric_tex.get_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::D3),
            ..Default::default()
        });

        // 获取通用的 Light Bind Group
        let materials_storage_buffer =
            context.buffer(&standard_resources::material_storage_buffer());
        let light_bind_group = context
            .get_bind_group(
                "light_bind_group_layout",
                vec![
                    context
                        .buffer(&standard_resources::light_uniform_buffer())
                        .id,
                    ssao_blur.view_id,
                    cascade_view.1,
                    context
                        .buffer(&standard_resources::shadow_cascade_buffer())
                        .id,
                    point_shadow_map_view.1,
                    ssao_blur.view_id, // skybox placeholder or actual
                    volumetric_view.1, // skybox sampler or actual
                    volumetric_view.1, // sampler
                    context
                        .buffer(&standard_resources::point_light_storage_buffer())
                        .id,
                    context.buffer(&standard_resources::light_grid_buffer()).id,
                    context
                        .buffer(&standard_resources::light_index_list_buffer())
                        .id,
                    context
                        .buffer(&standard_resources::cluster_config_buffer())
                        .id,
                    volumetric_view.1,
                ],
            )
            .clone();

        let bindless_bind_group = context
            .get_bind_group(
                "bindless_bind_group_layout",
                vec![materials_storage_buffer.id],
            )
            .clone();

        let mut render_pass = context
            .encoder
            .begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("transparent mesh render pass"),
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

        let mesh_cache = context.backend.imported_mesh_cache.read().unwrap();
        let allocator = context.backend.imported_mesh_allocator.read().unwrap();

        for camera_idx in 0..extracted.cameras.uniforms.len() {
            let camera_offset = camera_idx as u32 * CameraUniform::get_uniform_offset_unit();

            if extracted.cameras.types[camera_idx] == CameraType::D3 {
                render_pass.set_pipeline(self.pipeline.as_ref().unwrap());
                render_pass.set_bind_group(0, &camera_bind_group, &[camera_offset]);
                render_pass.set_bind_group(1, &light_bind_group, &[]);
                render_pass.set_bind_group(2, &bindless_bind_group, &[]);

                render_pass.set_vertex_buffer(0, allocator.vertex_buffer.slice(..));
                render_pass
                    .set_index_buffer(allocator.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                render_pass.set_vertex_buffer(1, transparent_instance_buffer.buffer.slice(..));

                for batch in &context.prepared.transparent_draw_batches {
                    if let Some(mesh) = mesh_cache.get(batch.mesh_id) {
                        render_pass.draw_indexed(
                            mesh.index_offset..mesh.index_offset + mesh.index_count,
                            mesh.vertex_offset as i32,
                            batch.instance_range.clone(),
                        );
                    }
                }
            }
        }
    }
}
