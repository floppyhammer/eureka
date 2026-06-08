use crate::render::camera::{CameraType, CameraUniform};
use crate::render::render_graph::{FrameContext, Node, ResourceId, TextureKey};
use crate::render::render_graph::standard_resources;
use crate::render::vertex::{Vertex3d, VertexBuffer};
use crate::render::{create_render_pipeline, InstanceRaw, Texture};
use std::any::Any;
use crate::render::render_graph::resource::BufferKey;

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

    fn input_resources(&self) -> Vec<ResourceId<()>> {
        vec![standard_resources::camera_buffer()]
    }

    fn output_resources(&self) -> Vec<ResourceId<()>> {
        vec![standard_resources::main_color(), standard_resources::main_depth()]
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
        };
        let main_depth_key = TextureKey {
            width,
            height,
            format: Texture::DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        };

        // 使用类型化资源ID获取瞬时资源
        let main_color = context.get_texture_by_id(&standard_resources::main_color(), main_color_key);
        let main_depth = context.get_texture_by_id(&standard_resources::main_depth(), main_depth_key);

        // 获取相机 Buffer (自动参与 FIF 同步)
        let camera_buffer_key = BufferKey {
            size: (CameraUniform::get_uniform_offset_unit() * context.render_world.extracted.cameras.uniforms.len() as u32) as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        };
        let camera_buffer = context.get_buffer_by_id(&standard_resources::camera_buffer(), camera_buffer_key);

        let world = &*context.render_world;
        if world.extracted.meshes.is_empty() {
            return;
        }

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