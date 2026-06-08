use crate::render::camera::{CameraUniform};
use crate::render::render_graph::{FrameContext, Node, BufferKey, standard_resources};
use std::any::Any;

/// 准备视角数据（相机矩阵等），并上传到池化缓冲区。
/// 它是所有 3D 渲染节点的共同输入源。
#[derive(Default)]
pub struct PrepareViewNode;

impl Node for PrepareViewNode {
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn output_resources(&self) -> Vec<crate::render::render_graph::ResourceId<()>> {
        vec![standard_resources::camera_buffer()]
    }

    fn run(&mut self, context: &mut FrameContext) {
        // 先提取需要的数据，避免长时间占用 context 的借用
        let uniforms = context.render_world.extracted.cameras.uniforms.clone();
        if uniforms.is_empty() {
            return;
        }

        let camera_count = uniforms.len();
        let offset_unit = CameraUniform::get_uniform_offset_unit();
        let buffer_size = offset_unit * camera_count as u32;

        let buffer_key = BufferKey {
            size: buffer_size as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        };

        // 1. 动态申请池化 Buffer 并注册到 context
        let pooled_buffer = context.get_buffer_by_id(&standard_resources::camera_buffer(), buffer_key);

        // 2. 准备数据并写入
        let mut aligned_up_data = vec![0u8; offset_unit as usize * camera_count];
        for i in 0..camera_count {
            let slice = bytemuck::cast_slice(&uniforms[i..i + 1]);
            let offset = i * offset_unit as usize;
            aligned_up_data[offset..offset + slice.len()].copy_from_slice(slice);
        }

        context.render_context.queue.write_buffer(
            &pooled_buffer.buffer,
            0,
            &aligned_up_data,
        );

        // 3. 为了兼容现有的渲染函数，暂时将结果存回 CameraRenderResources
        context.render_world.camera_render_resources.uniform_buffer = Some(pooled_buffer.buffer.clone());

        // 克隆 Layout 以断开借用链
        let layout = context.render_world.camera_render_resources.bind_group_layout.clone();

        let bind_group = context.create_bind_group(
            &layout,
            vec![pooled_buffer.id],
            |ctx| {
                ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    layout: &layout,
                    entries: &[wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                            buffer: &pooled_buffer.buffer,
                            offset: 0,
                            size: Some(
                                wgpu::BufferSize::new(std::mem::size_of::<CameraUniform>() as u64)
                                    .unwrap(),
                            ),
                        }),
                    }],
                    label: Some("camera dynamic bind group"),
                })
            },
        );
        context.render_world.camera_render_resources.bind_group = Some(bind_group);
    }
}
