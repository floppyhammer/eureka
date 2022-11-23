use crate::render::vertex::VertexBuffer;
use crate::{RenderServer, Singletons, Texture};
use cgmath::{Point2, Vector2, Vector4};
use wgpu::util::DeviceExt;
use wgpu::Buffer;
use crate::scene::CameraInfo;

/// To draw multiple textures with an instanced draw call.
/// CPU data.
pub struct AtlasInstance {
    pub(crate) position: Vector2<f32>,
    pub(crate) size: Vector2<f32>,
    pub(crate) region: Vector4<f32>,
    pub(crate) color: Vector4<f32>,
}

/// GPU data.
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct AtlasInstanceRaw {
    position: [f32; 2],
    size: [f32; 2],
    region: [f32; 4],
    color: [f32; 4],
}

#[repr(C)]
// This is so we can store this in a buffer.
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct AtlasParamsUniform {
    camera_view_size: [f32; 2],
    texture_size: [f32; 2],
    mode_flag: u32,
    pad0: u32,
    pad1: u32,
    pad2: u32,
}

#[derive(Default, Copy, Clone)]
pub(crate) enum AtlasMode {
    #[default]
    Sprite = 0x1,
    Text = 0x2,
}

impl AtlasParamsUniform {
    pub(crate) fn new(texture_size: Point2<u32>,
                      camera_view_size: Point2<u32>,
                      mode_flag: u32) -> Self {
        use cgmath::SquareMatrix;
        Self {
            camera_view_size: [camera_view_size.x as f32, camera_view_size.y as f32],
            texture_size: [texture_size.x as f32, texture_size.y as f32],
            mode_flag,
            pad0: 0,
            pad1: 0,
            pad2: 0,
        }
    }

    pub(crate) fn default() -> Self {
        Self {
            camera_view_size: [0.0, 0.0],
            texture_size: [0.0, 0.0],
            mode_flag: 0,
            pad0: 0,
            pad1: 0,
            pad2: 0,
        }
    }
}

impl AtlasInstance {
    fn to_raw(&self) -> AtlasInstanceRaw {
        AtlasInstanceRaw {
            position: [self.position.x, self.position.y],
            size: [self.size.x, self.size.y],
            region: self.region.into(),
            color: [1.0, 1.0, 1.0, 1.0],
        }
    }

    fn create_instance_buffer(
        device: &wgpu::Device,
        instances: Vec<AtlasInstance>,
    ) -> wgpu::Buffer {
        let instance_data = instances
            .iter()
            .map(AtlasInstance::to_raw)
            .collect::<Vec<_>>();

        let instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("atlas instance buffer"),
            contents: bytemuck::cast_slice(&instance_data),
            usage: wgpu::BufferUsages::VERTEX,
        });

        instance_buffer
    }
}

impl VertexBuffer for AtlasInstanceRaw {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<AtlasInstance>() as wgpu::BufferAddress,
            // We need to switch from using a step mode of Vertex to Instance.
            // This means that our shaders will only change to use the next
            // instance when the shader starts processing a new instance.
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 8]>() as wgpu::BufferAddress,
                    shader_location: 3,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}

pub(crate) struct Atlas {
    pub(crate) instances: Vec<AtlasInstance>,
    instance_buffer: Option<wgpu::Buffer>,
    texture: Texture,
    texture_bind_group: wgpu::BindGroup,
    atlas_params_buffer: wgpu::Buffer,
    atlas_params_bind_group: wgpu::BindGroup,
    mode: AtlasMode,
}

impl Atlas {
    pub fn new(render_server: &RenderServer) -> Self {
        let texture = Texture::empty(&render_server.device, &render_server.queue, (16, 16)).unwrap();

        let texture_bind_group = render_server.create_sprite2d_bind_group(&texture);

        let (atlas_params_buffer, atlas_params_bind_group) =
            render_server.create_atlas_params_bind_group();

        Self {
            instances: vec![],
            instance_buffer: None,
            texture,
            texture_bind_group,
            atlas_params_buffer,
            atlas_params_bind_group,
            mode: AtlasMode::Sprite,
        }
    }

    pub(crate) fn set_texture(&mut self, texture: Texture, render_server: &RenderServer) {
        self.texture = texture;

        self.texture_bind_group = render_server.create_sprite2d_bind_group(&self.texture);
    }

    pub(crate) fn set_mode(&mut self, mode: AtlasMode) {
        self.mode = mode;
    }

    pub(crate) fn set_instances(&mut self, instances: Vec<AtlasInstance>, render_server: &RenderServer) {
        self.instances = instances;
        if self.instances.len() == 0 {
            return;
        }

        let instance_data = self.instances.iter().map(AtlasInstance::to_raw).collect::<Vec<_>>();

        match &self.instance_buffer {
            // Allocate a new buffer.
            None => {
                self.instance_buffer = Some(render_server
                    .device
                    .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("atlas instance buffer"),
                        contents: bytemuck::cast_slice(&instance_data),
                        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                    }));
            }
            // Update data.
            // FIXME: Should handle instance buffer size change.
            Some(buffer) => {
                render_server
                    .queue
                    .write_buffer(buffer, 0, bytemuck::cast_slice(&instance_data));
            }
        }
    }

    pub(crate) fn draw<'a, 'b: 'a>(
        &'b self,
        render_pass: &mut wgpu::RenderPass<'a>,
        camera_info: &'b CameraInfo,
        singletons: &'b Singletons,
    ) {
        let render_server = &singletons.render_server;

        let atlas_params = AtlasParamsUniform::new(self.texture.size,
                                                   camera_info.view_size,
                                                   self.mode as u32);
        render_server
            .queue
            .write_buffer(&self.atlas_params_buffer, 0, bytemuck::cast_slice(&[atlas_params]));

        let instance_count = self.instances.len();
        if instance_count == 0 {
            return;
        }

        match &self.instance_buffer {
            None => {}
            Some(buffer) => {
                render_pass.draw_atlas(
                    &render_server.atlas_pipeline,
                    buffer,
                    self.instances.len() as u32,
                    &self.texture_bind_group,
                    &self.atlas_params_bind_group,
                );
            }
        }
    }
}

pub trait DrawAtlas<'a> {
    fn draw_atlas(
        &mut self,
        pipeline: &'a wgpu::RenderPipeline,
        instance_buffer: &'a wgpu::Buffer,
        instance_count: u32,
        texture_bind_group: &'a wgpu::BindGroup,
        atlas_params_bind_group: &'a wgpu::BindGroup,
    );
}

impl<'a, 'b> DrawAtlas<'b> for wgpu::RenderPass<'a>
    where
        'b: 'a, // This means 'b must outlive 'a.
{
    fn draw_atlas(
        &mut self,
        pipeline: &'b wgpu::RenderPipeline,
        instance_buffer: &'a wgpu::Buffer,
        instance_count: u32,
        texture_bind_group: &'a wgpu::BindGroup,
        atlas_params_bind_group: &'a wgpu::BindGroup,
    ) {
        self.set_pipeline(&pipeline);

        // Set instance vertex buffer.
        self.set_vertex_buffer(0, instance_buffer.slice(..));

        // Set bind groups.
        self.set_bind_group(0, &atlas_params_bind_group, &[]);
        self.set_bind_group(1, &texture_bind_group, &[]);

        self.draw(0..4, 0..instance_count);
    }
}
