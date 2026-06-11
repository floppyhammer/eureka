use crate::render::camera::CameraUniform;
use crate::render::render_graph::{standard_resources, FrameContext, Node};
use std::any::Any;
use crate::render::render_world::RenderWorld;

/// 准备视角数据（相机矩阵等），并上传到池化缓冲区。
/// 它是所有 3D 渲染节点的共同输入源。
#[derive(Default)]
pub struct PrepareViewNode;

pub(crate) const MAX_CAMERAS: u32 = 16;

impl Node for PrepareViewNode {
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn node_resources(&self, world: &RenderWorld) -> crate::render::render_graph::resource::NodeResources {
        use crate::render::camera::CameraUniform;
        use crate::render::render_graph::resource::ResourceSpec;

        let buffer_size = CameraUniform::get_uniform_offset_unit() * MAX_CAMERAS;

        // 输出为一个包含所有相机uniform的大缓冲区
        crate::render::render_graph::resource::NodeResources::new().output(
            standard_resources::camera_buffer(),
            ResourceSpec::buffer(
                buffer_size as u64,
                wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            ),
        )
    }

    fn run(&mut self, context: &mut FrameContext) {
        if context
            .pool
            .get_bind_group_layout("camera_bind_group_layout")
            .is_none()
        {
            let camera_bind_group_layout = context.render_context.device.create_bind_group_layout(
                &wgpu::BindGroupLayoutDescriptor {
                    entries: &[wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: true,
                            min_binding_size: None,
                        },
                        count: None,
                    }],
                    label: Some("Camera Bind Group Layout"),
                },
            );

            context
                .pool
                .add_bind_group_layout("camera_bind_group_layout", camera_bind_group_layout);
        }
        
        let extracted_cameras = context.render_world.extracted.cameras.clone();
        let uniforms = extracted_cameras.uniforms.clone();

        // No cameras.
        if uniforms.is_empty() {
            return;
        }

        let camera_count = uniforms.len();
        let offset_unit = CameraUniform::get_uniform_offset_unit();

        let camera_buffer = context.buffer(&standard_resources::camera_buffer());

        // 2. 准备数据并写入
        let mut aligned_up_data = vec![0u8; offset_unit as usize * camera_count];
        for i in 0..camera_count {
            let slice = bytemuck::cast_slice(&uniforms[i..i + 1]);
            let offset = i * offset_unit as usize;
            aligned_up_data[offset..offset + slice.len()].copy_from_slice(slice);
        }

        context
            .render_context
            .queue
            .write_buffer(&camera_buffer.buffer, 0, &aligned_up_data);

        let camera_bind_group_layout = context
            .pool
            .get_bind_group_layout("camera_bind_group_layout")
            .unwrap()
            .clone();

        let _ =
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
    }
}
