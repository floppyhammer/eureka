use crate::math::transform::Transform2d;
use crate::resources::{Material2d, Mesh, Texture};
use crate::scene::{AsNode, Camera2dUniform, CameraInfo, NodeType};
use crate::{Camera2d, InputEvent, RenderServer, SamplerBindingType, Singletons};
use cgmath::{Vector2, Vector3, Vector4};
use std::any::Any;
use wgpu::util::DeviceExt;

pub struct SpriteSheet {
    h_frames: u32,
    v_frames: u32,
    frame: u32,
}

pub struct Sprite2d {
    pub name: String,

    transform: Transform2d,
    pub size: Vector2<f32>,

    // A portion of the texture to draw.
    pub region: Vector4<f32>,

    pub sprite_sheet: SpriteSheet,

    pub texture: Option<Texture>,
    pub texture_bind_group: wgpu::BindGroup,

    camera_uniform: Camera2dUniform,
    pub camera_buffer: wgpu::Buffer,
    pub camera_bind_group: wgpu::BindGroup,

    pub centered: bool,

    pub mesh: Mesh,
}

impl Sprite2d {
    pub(crate) fn new(render_server: &RenderServer, texture: Texture) -> Sprite2d {
        let device = &render_server.device;

        let size = Vector2::new(128.0_f32, 128.0);

        let region = Vector4::new(0.0_f32, 0.0, 1.0, 1.0);

        let mesh = Mesh::default_2d(device);

        let texture_bind_group = render_server.create_sprite2d_bind_group(&texture);

        let (camera_buffer, camera_bind_group) = render_server.create_camera2d_resources(device);

        Self {
            name: "".to_string(),
            transform: Transform2d::default(),
            size,
            region,
            sprite_sheet: SpriteSheet {
                h_frames: 0,
                v_frames: 0,
                frame: 0,
            },
            texture: Some(texture),
            texture_bind_group,
            camera_uniform: Camera2dUniform::default(),
            camera_buffer,
            camera_bind_group,
            centered: false,
            mesh,
        }
    }

    pub fn set_texture(
        &mut self,
        device: &wgpu::Device,
        render_server: &RenderServer,
        texture: Texture,
    ) {
        self.texture_bind_group = render_server.create_sprite2d_bind_group(&texture);
        self.texture = Some(texture);
    }
}

impl AsNode for Sprite2d {
    fn node_type(&self) -> NodeType {
        NodeType::Label
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn ready(&mut self) {}

    fn update(&mut self, dt: f32, camera_info: &CameraInfo, singletons: &mut Singletons) {
        let scaled_width = self.transform.scale.x * self.size.x;
        let scaled_height = self.transform.scale.y * self.size.y;

        let view_size = camera_info.view_size;

        let translation = if self.centered {
            cgmath::Matrix4::from_translation(Vector3::new(
                (self.transform.position.x / view_size.x as f32 - scaled_width * 0.5)
                    / view_size.x as f32
                    * 2.0
                    - 1.0,
                (self.transform.position.y / view_size.y as f32 - scaled_height * 0.5)
                    / view_size.y as f32
                    * 2.0
                    - 1.0,
                0.0,
            ))
        } else {
            cgmath::Matrix4::from_translation(Vector3::new(
                (self.transform.position.x / view_size.x as f32) / view_size.x as f32 * 2.0 - 1.0,
                (self.transform.position.y / view_size.y as f32) / view_size.y as f32 * 2.0 - 1.0,
                0.0,
            ))
        };

        let scale = cgmath::Matrix4::from_nonuniform_scale(
            scaled_width / camera_info.view_size.x as f32,
            scaled_height / camera_info.view_size.y as f32,
            1.0,
        );

        // Note the multiplication direction (left multiplication).
        // So, scale first, translation second.
        self.camera_uniform.proj = (translation * scale).into();
    }

    fn draw<'a, 'b: 'a>(
        &'b self,
        render_pass: &mut wgpu::RenderPass<'a>,
        camera_info: &'b CameraInfo,
        singletons: &'b Singletons,
    ) {
        // Update camera buffer.
        singletons.render_server.queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(&[self.camera_uniform]),
        );

        render_pass.draw_sprite(
            &singletons.render_server.sprite_pipeline,
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
