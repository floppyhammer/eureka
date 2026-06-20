use crate::render::camera::{CameraType, CameraUniform};
use crate::render::render_backend::PreparedFrame;
use crate::render::render_graph::standard_resources;
use crate::render::render_graph::{FrameContext, Node};
use crate::render::vertex::{Vertex3d, VertexBuffer};
use crate::render::{create_render_pipeline, InstanceRaw, Texture};
use std::any::Any;

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
        use crate::render::render_graph::resource::{NodeResources, ResourceSpec, TextureKey};
        use crate::render::render_graph::standard_resources;
        use crate::render::Texture;
        use super::shared_mesh::common_mesh_resources;

        let transparent_instance_buffer_size = (prepared.sorted_transparent_instances.len() * size_of::<InstanceRaw>()) as u64;

        let resources = NodeResources::new()
            .internal(
                standard_resources::transparent_instance_buffer(),
                ResourceSpec::buffer(transparent_instance_buffer_size.max(64), wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST),
            )
            .output(
                standard_resources::main_color(),
                ResourceSpec::Texture(TextureKey::d2(0, 0, wgpu::TextureFormat::Rgba16Float, wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING)),
            )
            .output(
                standard_resources::main_depth(),
                ResourceSpec::Texture(TextureKey::d2(0, 0, Texture::DEPTH_FORMAT, wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING)),
            );

        common_mesh_resources(resources, prepared)
    }

    fn run(&mut self, context: &mut FrameContext) {
        if context.prepared.transparent_draw_batches.is_empty() {
            return;
        }

        let device = &context.render_context.device;
        let queue = &context.render_context.queue;
        let extracted = context.extracted;

        // 1. 上传排好序的实例数据
        let transparent_instance_buffer = context.buffer(&standard_resources::transparent_instance_buffer());
        queue.write_buffer(
            &transparent_instance_buffer.buffer,
            0,
            bytemuck::cast_slice(&context.prepared.sorted_transparent_instances),
        );

        if self.pipeline.is_none() {
            let light_layout = super::shared_mesh::get_or_create_light_layout(context);
            let camera_layout = context.backend.get_bind_group_layout("camera_bind_group_layout").unwrap();
            let bindless_layout = context.backend.get_bind_group_layout("bindless_bind_group_layout").unwrap();

            let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Transparent Mesh Layout"),
                bind_group_layouts: &[Some(camera_layout), Some(&light_layout), Some(bindless_layout)],
                immediate_size: 0,
            });

            let source = include_str!("../../../shaders/mesh.wgsl")
                .replace("#import eureka::camera::Camera", crate::render::camera::CAMERA_STRUCT_WGSL);

            self.pipeline = Some(create_render_pipeline(
                device, &pipeline_layout, Some(wgpu::TextureFormat::Rgba16Float), Some(Texture::DEPTH_FORMAT),
                &[Vertex3d::desc(), InstanceRaw::desc()],
                wgpu::ShaderModuleDescriptor { label: None, source: wgpu::ShaderSource::Wgsl(source.into()) },
                "Transparent Mesh", true, Some(wgpu::Face::Back),
            ));
        }

        let (camera_bg, light_bg, bindless_bg) = super::shared_mesh::get_mesh_bind_groups(context);
        let main_color = context.texture(&standard_resources::main_color());
        let main_depth = context.texture(&standard_resources::main_depth());

        // 提前获取排序后的 Instance Buffer
        let transparent_instance_buffer = context.buffer(&standard_resources::transparent_instance_buffer());

        let mut render_pass = context.encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("transparent mesh pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &main_color.view, depth_slice: None, resolve_target: None,
                ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &main_depth.view,
                depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store }),
                stencil_ops: None,
            }),
            timestamp_writes: None, occlusion_query_set: None,
            multiview_mask: None,
        });

        let mesh_cache = context.backend.imported_mesh_cache.read().unwrap();
        let allocator = context.backend.imported_mesh_allocator.read().unwrap();

        for (camera_idx, cam_type) in extracted.cameras.types.iter().enumerate() {
            if *cam_type == CameraType::D3 {
                let offset = camera_idx as u32 * CameraUniform::get_uniform_offset_unit();
                render_pass.set_pipeline(self.pipeline.as_ref().unwrap());
                render_pass.set_bind_group(0, &camera_bg, &[offset]);
                render_pass.set_bind_group(1, &light_bg, &[]);
                render_pass.set_bind_group(2, &bindless_bg, &[]);
                render_pass.set_vertex_buffer(0, allocator.vertex_buffer.slice(..));
                render_pass.set_vertex_buffer(1, transparent_instance_buffer.buffer.slice(..));
                render_pass.set_index_buffer(allocator.index_buffer.slice(..), wgpu::IndexFormat::Uint32);

                for batch in &context.prepared.transparent_draw_batches {
                    if let Some(mesh) = mesh_cache.get(batch.mesh_id) {
                        render_pass.draw_indexed(mesh.index_offset..mesh.index_offset + mesh.index_count, mesh.vertex_offset as i32, batch.instance_range.clone());
                    }
                }
            }
        }
    }
}
