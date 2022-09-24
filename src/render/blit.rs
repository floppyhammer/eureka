use cgmath::{Vector2, Vector4};
use wgpu::util::DeviceExt;

/// To draw multiple textures with an instanced draw call.
/// CPU data.
pub struct BlitInstance {
    position: Vector2<f32>,
    rotation: f32,
    region: Vector4<f32>,
}

impl BlitInstance {
    fn to_raw(&self) -> BlitInstanceRaw {
        BlitInstanceRaw {
            model: [self.position.x, self.position.y, self.rotation],
            region: self.region.into(),
        }
    }

    fn create_instance_buffer(device: &wgpu::Device, instances: Vec<BlitInstance>) -> wgpu::Buffer {
        let instance_data = instances
            .iter()
            .map(BlitInstance::to_raw)
            .collect::<Vec<_>>();
        let instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Blit Instance Buffer"),
            contents: bytemuck::cast_slice(&instance_data),
            usage: wgpu::BufferUsages::VERTEX,
        });

        instance_buffer
    }
}

/// GPU data.
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct BlitInstanceRaw {
    model: [f32; 3],
    region: [f32; 4],
}

impl BlitInstanceRaw {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<BlitInstanceRaw>() as wgpu::BufferAddress,
            // We need to switch from using a step mode of Vertex to Instance
            // This means that our shaders will only change to use the next
            // instance when the shader starts processing a new instance
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[wgpu::VertexAttribute {
                offset: 0,
                shader_location: 1,
                format: wgpu::VertexFormat::Float32x3,
            }],
        }
    }
}
