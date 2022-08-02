use crate::resource::{Material2d, Mesh, Texture};
use crate::scene::{AsNode, Camera2dUniform};
use crate::{Camera2d, InputEvent, RenderServer, SamplerBindingType};
use cgmath::Vector3;
use wgpu::util::DeviceExt;

pub struct Sprite2d {
    pub name: String,

    pub position: cgmath::Vector2<f32>,
    pub size: cgmath::Vector2<f32>,
    pub scale: cgmath::Vector2<f32>,

    pub texture: Option<Texture>,
    pub texture_bind_group: wgpu::BindGroup,

    pub camera_buffer: wgpu::Buffer,
    pub camera_bind_group: wgpu::BindGroup,

    pub centered: bool,

    pub mesh: Mesh,
}

impl Sprite2d {
    pub(crate) fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        render_server: &RenderServer,
        texture: Texture,
    ) -> Sprite2d {
        let position = cgmath::Vector2::new(0.0 as f32, 0.0);
        let size = cgmath::Vector2::new(128.0 as f32, 128.0);
        let scale = cgmath::Vector2::new(1.0 as f32, 1.0);

        let mesh = Mesh::default_2d(device);

        let texture_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &render_server.sprite_texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&(texture.view)),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&texture.sampler),
                },
            ],
            label: None,
        });

        let (camera_buffer, camera_bind_group) = render_server.create_camera2d_resources(device);

        Self {
            name: "".to_string(),
            position,
            size,
            scale,
            texture: Some(texture),
            texture_bind_group,
            camera_buffer,
            camera_bind_group,
            centered: false,
            mesh,
        }
    }
}

impl AsNode for Sprite2d {
    fn input(&mut self, input: InputEvent) {}

    fn update(&mut self, queue: &wgpu::Queue, dt: f32, render_server: &RenderServer) {
        let camera = render_server.camera2d.as_ref().unwrap();

        let scaled_width = self.scale.x * self.size.x;
        let scaled_height = self.scale.y * self.size.y;

        let translation = if self.centered {
            cgmath::Matrix4::from_translation(Vector3::new(
                (self.position.x / camera.view_size.x as f32 - scaled_width * 0.5)
                    / camera.view_size.x as f32
                    * 2.0
                    - 1.0,
                (self.position.y / camera.view_size.y as f32 - scaled_height * 0.5)
                    / camera.view_size.x as f32
                    * 2.0
                    - 1.0,
                0.0,
            ))
        } else {
            cgmath::Matrix4::from_translation(Vector3::new(
                (self.position.x / camera.view_size.x as f32) / camera.view_size.x as f32 * 2.0
                    - 1.0,
                (self.position.y / camera.view_size.y as f32) / camera.view_size.x as f32 * 2.0
                    - 1.0,
                0.0,
            ))
        };

        let scale = cgmath::Matrix4::from_nonuniform_scale(
            scaled_width / camera.view_size.x as f32,
            scaled_height / camera.view_size.y as f32,
            1.0,
        );

        let mut uniform = Camera2dUniform::new();

        // Note the multiplication direction (left multiplication).
        // So, scale first, translation second.
        uniform.proj = (translation * scale).into();

        // Update camera buffer.
        queue.write_buffer(&self.camera_buffer, 0, bytemuck::cast_slice(&[uniform]));
    }

    fn draw<'a, 'b: 'a>(
        &'b self,
        render_pass: &mut wgpu::RenderPass<'a>,
        render_server: &'b RenderServer,
    ) {
        render_pass.draw_sprite(
            &render_server.sprite_pipeline,
            &self.mesh,
            &self.texture_bind_group,
            &self.camera_bind_group,
        );
    }
}

pub trait DrawSprite2d<'a> {
    fn draw_sprite(
        &mut self,
        pipeline: &'a wgpu::RenderPipeline,
        mesh: &'a Mesh,
        texture_bind_group: &'a wgpu::BindGroup,
        camera_bind_group: &'a wgpu::BindGroup,
    );
}

impl<'a, 'b> DrawSprite2d<'b> for wgpu::RenderPass<'a>
where
    'b: 'a, // This means 'b must outlive 'a.
{
    fn draw_sprite(
        &mut self,
        pipeline: &'b wgpu::RenderPipeline,
        mesh: &'b Mesh,
        texture_bind_group: &'b wgpu::BindGroup,
        camera_bind_group: &'b wgpu::BindGroup,
    ) {
        self.set_pipeline(&pipeline);

        // Set vertex buffer for VertexInput.
        self.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));

        self.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);

        // Set camera group.
        self.set_bind_group(0, &camera_bind_group, &[]);

        // Set texture group.
        self.set_bind_group(1, &texture_bind_group, &[]);

        self.draw_indexed(0..mesh.index_count, 0, 0..1);
    }
}
