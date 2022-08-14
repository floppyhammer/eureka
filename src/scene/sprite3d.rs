use crate::resource::{Material2d, Mesh, Texture};
use crate::scene::{AsNode, Camera2dUniform};
use crate::{Camera2d, InputEvent, RenderServer, SamplerBindingType, Singletons, Zero};
use cgmath::{InnerSpace, Rotation3, Vector3};
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SpriteParamsUniform {
    model_matrix: [[f32; 4]; 4],
    billboard_mode: f32,
    pad0: f32,
    pad1: f32,
    pad2: f32,
}

pub struct Sprite3d {
    pub name: String,

    pub position: cgmath::Vector3<f32>,
    pub rotation: cgmath::Quaternion<f32>,
    pub scale: cgmath::Vector3<f32>,

    pub billboard_mode: BillboardMode,

    pub texture: Option<Texture>,
    pub texture_bind_group: wgpu::BindGroup,

    pub params_uniform: SpriteParamsUniform,
    pub params_buffer: wgpu::Buffer,
    pub params_bind_group: wgpu::BindGroup,

    pub mesh: Mesh,
}

impl Sprite3d {
    pub(crate) fn new(render_server: &RenderServer, texture: Texture) -> Sprite3d {
        let device = &render_server.device;
        let queue = &render_server.queue;

        let position = Vector3::new(0.0 as f32, 0.0, 0.0);
        let rotation = if position.is_zero() {
            // This is needed so an object at (0, 0, 0) won't get scaled to zero
            // as Quaternions can effect scale if they're not created correctly.
            cgmath::Quaternion::from_axis_angle(cgmath::Vector3::unit_z(), cgmath::Deg(0.0))
        } else {
            cgmath::Quaternion::from_axis_angle(position.normalize(), cgmath::Deg(45.0))
        };
        let scale = Vector3::new(1.0 as f32, 1.0, 1.0);

        let mesh = Mesh::default_3d(device);

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

        let billboard_mode = BillboardMode::Spherical;

        // Create a buffer for the params.
        // ------------------------------------------
        let params_uniform = SpriteParamsUniform {
            model_matrix: cgmath::Matrix4::from_translation(position).into(),
            billboard_mode: if billboard_mode == BillboardMode::Spherical {
                1.0
            } else {
                0.0
            },
            pad0: 0.0,
            pad1: 0.0,
            pad2: 0.0,
        };

        let params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("sprite params buffer"),
            contents: bytemuck::cast_slice(&[params_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let params_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &render_server.sprite_params_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: params_buffer.as_entire_binding(),
            }],
            label: Some("sprite params bind group"),
        });

        // Update buffer.
        queue.write_buffer(&params_buffer, 0, bytemuck::cast_slice(&[params_uniform]));
        // ------------------------------------------

        Self {
            name: "".to_string(),
            position,
            rotation,
            scale,
            billboard_mode,
            texture: Some(texture),
            texture_bind_group,
            params_uniform,
            params_buffer,
            params_bind_group,
            mesh,
        }
    }
}

#[derive(PartialEq)]
pub enum BillboardMode {
    None,
    Spherical,
}

impl AsNode for Sprite3d {
    fn input(&mut self, input: &InputEvent) {}

    fn update(
        &mut self,
        queue: &wgpu::Queue,
        dt: f32,
        render_server: &RenderServer,
        singletons: Option<&Singletons>,
    ) {
        let params_uniform = SpriteParamsUniform {
            model_matrix: cgmath::Matrix4::from_translation(self.position).into(),
            billboard_mode: if self.billboard_mode == BillboardMode::Spherical {
                1.0
            } else {
                0.0
            },
            pad0: 0.0,
            pad1: 0.0,
            pad2: 0.0,
        };

        // Update buffer.
        queue.write_buffer(
            &self.params_buffer,
            0,
            bytemuck::cast_slice(&[params_uniform]),
        );
    }

    fn draw<'a, 'b: 'a>(
        &'b self,
        render_pass: &mut wgpu::RenderPass<'a>,
        render_server: &'b RenderServer,
        singletons: &'b Singletons,
    ) {
        render_pass.draw_sprite(
            &render_server.sprite3d_pipeline,
            &self.mesh,
            &self.texture_bind_group,
            &singletons.camera3d.as_ref().unwrap().bind_group,
            &self.params_bind_group,
        );
    }
}

pub trait DrawSprite3d<'a> {
    fn draw_sprite(
        &mut self,
        pipeline: &'a wgpu::RenderPipeline,
        mesh: &'a Mesh,
        texture_bind_group: &'a wgpu::BindGroup,
        camera_bind_group: &'a wgpu::BindGroup,
        params_bind_group: &'a wgpu::BindGroup,
    );
}

impl<'a, 'b> DrawSprite3d<'b> for wgpu::RenderPass<'a>
where
    'b: 'a, // This means 'b must outlive 'a.
{
    fn draw_sprite(
        &mut self,
        pipeline: &'b wgpu::RenderPipeline,
        mesh: &'b Mesh,
        texture_bind_group: &'b wgpu::BindGroup,
        camera_bind_group: &'b wgpu::BindGroup,
        params_bind_group: &'b wgpu::BindGroup,
    ) {
        self.set_pipeline(&pipeline);

        // Set vertex buffer for VertexInput.
        self.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));

        self.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);

        // Set camera group.
        self.set_bind_group(0, &camera_bind_group, &[]);

        // Set texture group.
        self.set_bind_group(1, &texture_bind_group, &[]);

        // Set params group.
        self.set_bind_group(2, &params_bind_group, &[]);

        self.draw_indexed(0..mesh.index_count, 0, 0..1);
    }
}
