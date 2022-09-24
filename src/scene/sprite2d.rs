use crate::resource::{Material2d, Mesh, Texture};
use crate::scene::{AsNode, Camera2dUniform, NodeType};
use crate::{Camera2d, InputEvent, RenderServer, SamplerBindingType, Singletons};
use cgmath::{Vector2, Vector3, Vector4};
use wgpu::util::DeviceExt;

pub struct SpriteSheet {
    h_frames: u32,
    v_frames: u32,
    frame: u32,
}

pub struct Sprite2d {
    pub name: String,

    pub position: Vector2<f32>,
    pub size: Vector2<f32>,
    pub scale: Vector2<f32>,

    // A portion of the texture to draw.
    pub region: Vector4<f32>,

    pub sprite_sheet: SpriteSheet,

    pub texture: Option<Texture>,
    pub texture_bind_group: wgpu::BindGroup,

    pub camera_buffer: wgpu::Buffer,
    pub camera_bind_group: wgpu::BindGroup,

    pub centered: bool,

    pub mesh: Mesh,
}

impl Sprite2d {
    pub(crate) fn new(render_server: &RenderServer, texture: Texture) -> Sprite2d {
        let device = &render_server.device;

        let position = Vector2::new(0.0_f32, 0.0);
        let size = Vector2::new(128.0_f32, 128.0);
        let scale = Vector2::new(1.0_f32, 1.0);

        let region = Vector4::new(0.0_f32, 0.0, 1.0, 1.0);

        let mesh = Mesh::default_2d(device);

        let texture_bind_group = render_server.create_sprite2d_bind_group(&texture);

        let (camera_buffer, camera_bind_group) = render_server.create_camera2d_resources(device);

        Self {
            name: "".to_string(),
            position,
            size,
            scale,
            region,
            sprite_sheet: SpriteSheet {
                h_frames: 0,
                v_frames: 0,
                frame: 0,
            },
            texture: Some(texture),
            texture_bind_group,
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
        NodeType::Sprite2d
    }

    fn input(&mut self, input: &InputEvent) {}

    fn update(&mut self, dt: f32, render_server: &RenderServer, singletons: Option<&Singletons>) {
        let camera = singletons.unwrap().camera2d.as_ref().unwrap();

        let scaled_width = self.scale.x * self.size.x;
        let scaled_height = self.scale.y * self.size.y;

        let translation = if self.centered {
            cgmath::Matrix4::from_translation(Vector3::new(
                (self.position.x / camera.view_size.x as f32 - scaled_width * 0.5)
                    / camera.view_size.x as f32
                    * 2.0
                    - 1.0,
                (self.position.y / camera.view_size.y as f32 - scaled_height * 0.5)
                    / camera.view_size.y as f32
                    * 2.0
                    - 1.0,
                0.0,
            ))
        } else {
            cgmath::Matrix4::from_translation(Vector3::new(
                (self.position.x / camera.view_size.x as f32) / camera.view_size.x as f32 * 2.0
                    - 1.0,
                (self.position.y / camera.view_size.y as f32) / camera.view_size.y as f32 * 2.0
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
        render_server
            .queue
            .write_buffer(&self.camera_buffer, 0, bytemuck::cast_slice(&[uniform]));
    }

    fn draw<'a, 'b: 'a>(
        &'b self,
        render_pass: &mut wgpu::RenderPass<'a>,
        render_server: &'b RenderServer,
        singletons: &'b Singletons,
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
