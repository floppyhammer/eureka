use crate::render::camera::{CameraType, CameraUniform};
use crate::render::render_graph::resource::BufferKey;
use crate::render::render_graph::standard_resources;
use crate::render::render_graph::{FrameContext, Node, ResourceId, TextureKey};
use crate::render::vertex::{Vertex3d, VertexBuffer};
use crate::render::{create_render_pipeline, InstanceRaw, Texture};
use std::any::Any;

pub struct MeshNode {
    pipeline: Option<wgpu::RenderPipeline>,
}

impl Default for MeshNode {
    fn default() -> Self {
        Self { pipeline: None }
    }
}

impl Node for MeshNode {
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn node_resources(&self) -> crate::render::render_graph::resource::NodeResources {
        use crate::render::render_graph::resource::{BufferKey, ResourceSpec, TextureKey, ResourceId};
        use crate::render::Texture;
        use crate::render::render_graph::standard_resources;

        crate::render::render_graph::resource::NodeResources::new()
            .input(
                standard_resources::camera_buffer(),
                ResourceSpec::buffer(0, wgpu::BufferUsages::UNIFORM),
            )
            .optional_input(
                standard_resources::directional_shadow_map().erase(),
                ResourceSpec::Texture(TextureKey {
                    width: 2048,
                    height: 2048,
                    format: Texture::DEPTH_FORMAT,
                    usage: wgpu::TextureUsages::TEXTURE_BINDING,
                    layers: crate::render::light::NUM_CASCADES as u32,
                }),
            )
            .optional_input(
                standard_resources::ssao_blur().erase(),
                ResourceSpec::Texture(TextureKey {
                    width: 0,
                    height: 0,
                    format: wgpu::TextureFormat::R8Unorm,
                    usage: wgpu::TextureUsages::TEXTURE_BINDING,
                    layers: 1,
                }),
            )
            .output(
                standard_resources::main_color(),
                ResourceSpec::Texture(TextureKey {
                    width: 0, // 0 表示继承
                    height: 0,
                    format: wgpu::TextureFormat::Bgra8UnormSrgb,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
                    layers: 1,
                }),
            )
            .output(
                standard_resources::main_depth(),
                ResourceSpec::Texture(TextureKey {
                    width: 0,
                    height: 0,
                    format: Texture::DEPTH_FORMAT,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
                    layers: 1,
                }),
            )
    }

    fn prepare(&mut self, context: &mut FrameContext) {
        if self.pipeline.is_some() {
            return;
        }
        let device = &context.render_context.device;
        let world = &*context.render_world;
        let resources = &world.mesh_render_resources;

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("mesh layout"),
            bind_group_layouts: &[
                &world.camera_render_resources.bind_group_layout,
                &resources.light_bind_group_layout,
                &resources.bindless_bind_group_layout,
            ],
            push_constant_ranges: &[],
        });

        let shader = wgpu::ShaderModuleDescriptor {
            label: Some("mesh shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../../../shaders/mesh.wgsl").into()),
        };

        self.pipeline = Some(create_render_pipeline(
            device,
            &pipeline_layout,
            Some(context.render_context.surface_config.format),
            Some(Texture::DEPTH_FORMAT),
            &[Vertex3d::desc(), InstanceRaw::desc()],
            shader,
            "standard bindless",
            false,
            Some(wgpu::Face::Back),
        ));
    }

    fn run(&mut self, context: &mut FrameContext) {
        let width = context.render_context.surface_config.width;
        let height = context.render_context.surface_config.height;
        let format = context.render_context.surface_config.format;

        let main_color_key = TextureKey {
            width,
            height,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            layers: 1,
        };
        let main_depth_key = TextureKey {
            width,
            height,
            format: Texture::DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            layers: 1,
        };

        // 使用类型化资源ID获取瞬时资源
        let main_color =
            context.get_texture_by_id(&standard_resources::main_color(), main_color_key);
        let main_depth =
            context.get_texture_by_id(&standard_resources::main_depth(), main_depth_key);

        let r8_key = TextureKey {
            width,
            height,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING,
            layers: 1,
        };
        let ssao_blur = context.get_texture_by_id(&standard_resources::ssao_blur(), r8_key);

        let shadow_key = TextureKey {
            width: 2048,
            height: 2048,
            format: Texture::DEPTH_FORMAT,
            usage: wgpu::TextureUsages::TEXTURE_BINDING,
            layers: crate::render::light::NUM_CASCADES as u32,
        };
        let shadow_map = context.get_texture_by_id(&standard_resources::directional_shadow_map(), shadow_key);

        // 获取相机 Buffer (自动参与 FIF 同步)
        let camera_buffer_key = BufferKey {
            size: (CameraUniform::get_uniform_offset_unit()
                * context.render_world.extracted.cameras.uniforms.len() as u32)
                as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        };
        let camera_buffer =
            context.get_buffer_by_id(&standard_resources::camera_buffer(), camera_buffer_key);

        if context.render_world.extracted.meshes.is_empty() {
            return;
        }

        // --- 动态更新 Light Bind Group ---
        let device = &context.render_context.device;

        let shadow_sampler = device.create_sampler(&wgpu::SamplerDescriptor { label: Some("shadow sampler"), address_mode_u: wgpu::AddressMode::ClampToEdge, address_mode_v: wgpu::AddressMode::ClampToEdge, address_mode_w: wgpu::AddressMode::ClampToEdge, mag_filter: wgpu::FilterMode::Linear, min_filter: wgpu::FilterMode::Linear, mipmap_filter: wgpu::FilterMode::Nearest, compare: Some(wgpu::CompareFunction::LessEqual), ..Default::default() });
        let skybox_sampler = device.create_sampler(&wgpu::SamplerDescriptor { address_mode_u: wgpu::AddressMode::ClampToEdge, address_mode_v: wgpu::AddressMode::ClampToEdge, address_mode_w: wgpu::AddressMode::ClampToEdge, mag_filter: wgpu::FilterMode::Linear, min_filter: wgpu::FilterMode::Linear, mipmap_filter: wgpu::FilterMode::Linear, ..Default::default() });

        // CSM 视图
        let cascade_view = shadow_map.texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some("shadow cascade view"),
            format: Some(Texture::DEPTH_FORMAT),
            dimension: Some(wgpu::TextureViewDimension::D2Array),
            usage: Some(wgpu::TextureUsages::TEXTURE_BINDING),
            aspect: wgpu::TextureAspect::DepthOnly,
            array_layer_count: Some(crate::render::light::NUM_CASCADES as u32),
            ..Default::default()
        });

        // 提取所需句柄以断开借用链
        let light_resources_cascade_buffer = context.render_world.light_render_resources.cascade_uniform_buffer.as_ref().unwrap().clone();
        let mesh_resources_light_layout = context.render_world.mesh_render_resources.light_bind_group_layout.clone();
        let mesh_resources_light_buffer = context.render_world.mesh_render_resources.light_uniform_buffer.as_ref().unwrap().clone();

        let psv = if let Some(psm_id) = context.render_world.light_render_resources.point_shadow_map {
            let psm = context.render_world.texture_cache.get(psm_id).unwrap();
            psm.texture.create_view(&wgpu::TextureViewDescriptor { label: Some("psv"), format: Some(Texture::DEPTH_FORMAT), dimension: Some(wgpu::TextureViewDimension::CubeArray), aspect: wgpu::TextureAspect::DepthOnly, array_layer_count: Some(crate::render::light::MAX_POINT_LIGHTS as u32 * 6), ..Default::default() })
        } else {
            context.render_world.mesh_render_resources.dummy_cube_view.clone()
        };

        let sky_view = if let Some(id) = context.render_world.mesh_render_resources.current_skybox {
            context.render_world.texture_cache.get(id).unwrap().view.clone()
        } else {
            context.render_world.mesh_render_resources.dummy_cube_view.clone()
        };

        let light_bg = context.create_bind_group(
            &mesh_resources_light_layout,
            vec![ssao_blur.id, shadow_map.id],
            |ctx| {
                ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    layout: &mesh_resources_light_layout,
                    entries: &[
                        wgpu::BindGroupEntry { binding: 0, resource: mesh_resources_light_buffer.as_entire_binding() },
                        wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&cascade_view) },
                        wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(&shadow_sampler) },
                        wgpu::BindGroupEntry { binding: 3, resource: light_resources_cascade_buffer.as_entire_binding() },
                        wgpu::BindGroupEntry { binding: 4, resource: wgpu::BindingResource::TextureView(&psv) },
                        wgpu::BindGroupEntry { binding: 5, resource: wgpu::BindingResource::TextureView(&ssao_blur.view) },
                        wgpu::BindGroupEntry { binding: 6, resource: wgpu::BindingResource::TextureView(&sky_view) },
                        wgpu::BindGroupEntry { binding: 7, resource: wgpu::BindingResource::Sampler(&skybox_sampler) },
                    ],
                    label: Some("light bind group (dynamic)"),
                })
            }
        );
        context.render_world.mesh_render_resources.light_bind_group = Some(light_bg);

        let world = &*context.render_world;
        let mut render_pass = context
            .encoder
            .begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("mesh render pass"),
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

        for i in 0..world.extracted.cameras.uniforms.len() {
            if world.extracted.cameras.types[i] == CameraType::D3 {
                crate::render::render_meshes(
                    &world.extracted.meshes,
                    &world.mesh_cache,
                    &world.mesh_render_resources,
                    &world.camera_render_resources,
                    i,
                    &world.extracted.cameras.uniforms[i],
                    &world.gizmo_render_resources,
                    &mut render_pass,
                    &world.extracted.bvh,
                    self.pipeline.as_ref().unwrap(),
                );
            }
        }
    }
}
