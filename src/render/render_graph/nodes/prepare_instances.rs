use crate::render::render_backend::PreparedFrame;
use crate::render::render_graph::{standard_resources, FrameContext, Node, ResourceSpec};
use std::any::Any;
use wgpu::BufferAddress;

#[derive(Default)]
pub struct PrepareInstancesNode;

impl Node for PrepareInstancesNode {
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn node_resources(
        &self,
        prepared: &PreparedFrame,
    ) -> crate::render::render_graph::resource::NodeResources {
        let metadata_buffer_size = (prepared.mesh_metadatas.len()
            * size_of::<crate::render::mesh::MeshMetadata>())
            as BufferAddress;

        crate::render::render_graph::resource::NodeResources::new()
            .output(
                standard_resources::global_instance_buffer(), // 未经过 culling 的 instance 数据
                ResourceSpec::buffer(
                    prepared.instance_buffer_size as u64,
                    wgpu::BufferUsages::STORAGE
                        | wgpu::BufferUsages::VERTEX
                        | wgpu::BufferUsages::COPY_DST,
                ),
            )
            .output(
                standard_resources::mesh_metadata_buffer(),
                ResourceSpec::buffer(
                    metadata_buffer_size as u64,
                    wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                ),
            )
    }

    fn run(&mut self, context: &mut FrameContext) {
        if context.prepared.instance_buffer_size == 0 {
            return;
        }

        let global_instance_buffer = context.buffer(&standard_resources::global_instance_buffer());
        let mesh_metadata_buffer = context.buffer(&standard_resources::mesh_metadata_buffer());

        context.render_context.queue.write_buffer(
            &global_instance_buffer.buffer,
            0,
            bytemuck::cast_slice(&context.prepared.all_instances),
        );

        context.render_context.queue.write_buffer(
            &mesh_metadata_buffer.buffer,
            0,
            bytemuck::cast_slice(&context.prepared.mesh_metadatas),
        );
    }
}
