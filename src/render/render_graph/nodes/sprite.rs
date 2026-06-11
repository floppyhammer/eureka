use crate::render::camera::CameraUniform;
use crate::render::create_render_pipeline;
use crate::render::render_graph::{standard_resources, FrameContext, Node, TextureKey};
use crate::render::vertex::{Vertex2d, VertexBuffer};
use crate::render::Texture;
use std::any::Any;

pub struct SpriteNode {
    pipeline: Option<wgpu::RenderPipeline>,
}

impl Default for SpriteNode {
    fn default() -> Self {
        Self { pipeline: None }
    }
}

impl Node for SpriteNode {
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn node_resources(&self) -> crate::render::render_graph::resource::NodeResources {
        use crate::render::render_graph::resource::{ResourceSpec, TextureKey};
        use crate::render::render_graph::standard_resources;
        use crate::render::Texture;

        crate::render::render_graph::resource::NodeResources::new()
            .input(
                standard_resources::camera_buffer(),
                ResourceSpec::buffer(0, wgpu::BufferUsages::UNIFORM),
            )
            .output(
                standard_resources::main_depth(),
                ResourceSpec::Texture(TextureKey {
                    width: 0,
                    height: 0,
                    format: Texture::DEPTH_FORMAT,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                    layers: 1,
                }),
            )
            .output(
                standard_resources::hdr_resolved(),
                ResourceSpec::Texture(TextureKey {
                    width: 0,
                    height: 0,
                    format: wgpu::TextureFormat::Bgra8UnormSrgb,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
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

        let camera_bind_group_layout = context
            .pool
            .get_bind_group_layout("camera_bind_group_layout")
            .unwrap()
            .clone();

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("sprite bindless pipeline layout"),
            bind_group_layouts: &[
                &camera_bind_group_layout,
                &world.mesh_render_resources.bindless_bind_group_layout,
            ],
            push_constant_ranges: &[],
        });

        let shader = wgpu::ShaderModuleDescriptor {
            label: Some("sprite shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../../../shaders/sprite.wgsl").into()),
        };

        self.pipeline = Some(create_render_pipeline(
            device,
            &pipeline_layout,
            Some(context.render_context.surface_config.format),
            Some(Texture::DEPTH_FORMAT),
            &[Vertex2d::desc()],
            shader,
            "sprite bindless",
            true,
            None,
        ));
    }

    fn run(&mut self, context: &mut FrameContext) {
        let main_depth_key = TextureKey {
            width: context.render_context.surface_config.width,
            height: context.render_context.surface_config.height,
            format: Texture::DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            layers: 1,
        };
        let main_depth =
            context.get_texture_by_id(&standard_resources::main_depth(), main_depth_key);

        if context.render_world.sprite_batches.is_empty() {
            return;
        }

        let camera_bind_group_layout = context
            .pool
            .get_bind_group_layout("camera_bind_group_layout")
            .unwrap()
            .clone();

        let buffer_key = context
            .render_world
            .extracted
            .cameras
            .get_buffer_key()
            .clone();
        let camera_buffer = context
            .get_buffer_by_id(&standard_resources::camera_buffer(), buffer_key)
            .clone();
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

        let world = &*context.render_world;
        let batches = &context.render_world.sprite_batches;

        if let Some(bindless_bind_group) = &world.mesh_render_resources.bindless_bind_group {
            let mut render_pass = context
                .encoder
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("sprite render pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: context.final_output_view, // 直接绘制到 Surface
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load, // 在 3D 映射后的画面上叠加 UI
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                        view: &main_depth.view,
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Clear(1.0), // 关键：清除 3D 场景深度，开始 UI 深度测试
                            store: wgpu::StoreOp::Store,
                        }),
                        stencil_ops: None,
                    }),
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });

            render_pass.set_pipeline(self.pipeline.as_ref().unwrap());

            render_pass.set_vertex_buffer(
                0,
                world
                    .sprite_render_resources
                    .vertex_buffer
                    .as_ref()
                    .unwrap()
                    .slice(..),
            );

            render_pass.set_index_buffer(
                world
                    .sprite_render_resources
                    .index_buffer
                    .as_ref()
                    .unwrap()
                    .slice(..),
                wgpu::IndexFormat::Uint32,
            );

            render_pass.set_bind_group(1, bindless_bind_group, &[]);

            for b in batches {
                let camera_offset = CameraUniform::get_uniform_offset_unit() * b.camera_index;

                render_pass.set_bind_group(
                    0,
                    &camera_bind_group,
                    &[camera_offset],
                );

                render_pass.draw_indexed(b.index_range.clone(), 0, 0..1);
            }
        }
    }
}
