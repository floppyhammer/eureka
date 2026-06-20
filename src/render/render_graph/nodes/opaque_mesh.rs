use crate::render::camera::{CameraType, CameraUniform};
use crate::render::render_backend::PreparedFrame;
use crate::render::render_graph::standard_resources;
use crate::render::render_graph::{FrameContext, Node};
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

    fn node_resources(
        &self,
        prepared: &PreparedFrame,
    ) -> crate::render::render_graph::resource::NodeResources {
        use crate::render::render_graph::resource::{NodeResources, ResourceSpec, TextureKey};
        use crate::render::render_graph::standard_resources;
        use crate::render::Texture;
        use super::shared_mesh::common_mesh_resources;

        let resources = NodeResources::new()
            .input(
                standard_resources::cull_visible_instance_buffer(),
                ResourceSpec::buffer(
                    prepared.instance_buffer_size as u64,
                    wgpu::BufferUsages::STORAGE
                        | wgpu::BufferUsages::VERTEX
                        | wgpu::BufferUsages::COPY_DST,
                ),
            )
            .input(
                standard_resources::cull_indirect_buffer(),
                ResourceSpec::buffer(
                    prepared.indirect_buffer_size as u64,
                    wgpu::BufferUsages::STORAGE
                        | wgpu::BufferUsages::INDIRECT
                        | wgpu::BufferUsages::COPY_DST,
                ),
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
        if context.prepared.instance_buffer_size == 0 {
            return;
        }

        let device = &context.render_context.device;
        let extracted = context.extracted;

        if self.pipeline.is_none() {
            let camera_layout = context.backend.get_bind_group_layout("camera_bind_group_layout").unwrap().clone();
            let light_layout = super::shared_mesh::get_or_create_light_layout(context);
            let bindless_layout = context.backend.get_bind_group_layout("bindless_bind_group_layout").unwrap();

            let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Opaque Mesh Layout"),
                bind_group_layouts: &[&camera_layout, &light_layout, bindless_layout],
                push_constant_ranges: &[],
            });

            let source = include_str!("../../../shaders/mesh.wgsl")
                .replace("#import eureka::camera::Camera", crate::render::camera::CAMERA_STRUCT_WGSL);

            self.pipeline = Some(create_render_pipeline(
                device, &pipeline_layout, Some(wgpu::TextureFormat::Rgba16Float), Some(Texture::DEPTH_FORMAT),
                &[Vertex3d::desc(), InstanceRaw::desc()],
                wgpu::ShaderModuleDescriptor { label: None, source: wgpu::ShaderSource::Wgsl(source.into()) },
                "Opaque Mesh", false, Some(wgpu::Face::Back),
            ));
        }

        let (camera_bg, light_bg, bindless_bg) = super::shared_mesh::get_mesh_bind_groups(context);
        let main_color = FrameContext::texture(context, &standard_resources::main_color());
        let main_depth = context.texture(&standard_resources::main_depth());

        // 提前获取所有需要的 Buffer，避免在 RenderPass 中借用 context
        let visible_instances = context.buffer(&standard_resources::cull_visible_instance_buffer());
        let indirect_buffer = context.buffer(&standard_resources::cull_indirect_buffer());

        let mut render_pass = context.encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("opaque mesh pass"),
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
        });

        let allocator = context.backend.imported_mesh_allocator.read().unwrap();

        for (camera_idx, cam_type) in extracted.cameras.types.iter().enumerate() {
            if *cam_type == CameraType::D3 {
                let offset = camera_idx as u32 * CameraUniform::get_uniform_offset_unit();
                render_pass.set_pipeline(self.pipeline.as_ref().unwrap());
                render_pass.set_bind_group(0, &camera_bg, &[offset]);
                render_pass.set_bind_group(1, &light_bg, &[]);
                render_pass.set_bind_group(2, &bindless_bg, &[]);
                render_pass.set_vertex_buffer(0, allocator.vertex_buffer.slice(..));
                render_pass.set_vertex_buffer(1, visible_instances.buffer.slice(..));
                render_pass.set_index_buffer(allocator.index_buffer.slice(..), wgpu::IndexFormat::Uint32);

                if !context.prepared.draw_counts.is_empty() {
                    render_pass.multi_draw_indexed_indirect(&indirect_buffer.buffer, 0, context.prepared.draw_counts[0]);
                }
            }
        }
    }
}
