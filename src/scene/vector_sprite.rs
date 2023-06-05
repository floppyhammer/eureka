extern crate lyon;

use std::any::Any;

use crate::math::transform::Transform2d;
use crate::scene::{AsNode, CameraUniform, CameraInfo, NodeType};
use crate::vector_image::{VectorMesh, VectorTexture};
use crate::{Camera2d, RenderServer, Singletons};
use cgmath::Vector3;
use lyon::math::point;
use lyon::path::Path;
use lyon::tessellation::*;
use wgpu::util::DeviceExt;

pub struct VectorSprite {
    pub transform: Transform2d,
    pub size: cgmath::Vector2<f32>,

    texture: Option<VectorTexture>,

    camera_uniform: CameraUniform,
    pub camera_buffer: wgpu::Buffer,
    pub camera_bind_group: wgpu::BindGroup,

    need_to_rebuild: bool,
}

impl VectorSprite {
    pub fn new(render_server: &RenderServer) -> VectorSprite {
        let device = &render_server.device;

        let (camera_buffer, camera_bind_group) = render_server.create_camera2d_resources(device);

        let size = cgmath::Vector2::new(128.0 as f32, 128.0);

        Self {
            transform: Transform2d::default(),
            size,
            texture: None,
            camera_uniform: CameraUniform::default(),
            camera_buffer,
            camera_bind_group,
            need_to_rebuild: false,
        }
    }

    pub fn set_texture(&mut self, texture: VectorTexture) {
        self.texture = Some(texture);
    }
}

impl AsNode for VectorSprite {
    fn node_type(&self) -> NodeType {
        NodeType::SpriteV
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn ready(&mut self) {}

    fn update(&mut self, dt: f32, camera_info: &CameraInfo, singletons: &mut Singletons) {
        let translation = cgmath::Matrix4::from_translation(Vector3::new(-1.0, 1.0, 0.0));

        let scale = cgmath::Matrix4::from_nonuniform_scale(
            1.0 / camera_info.view_size.x as f32 * 2.0,
            -1.0 / camera_info.view_size.y as f32 * 2.0,
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
        if let Some(tex) = &self.texture {
            // Update camera buffer.
            singletons.render_server.queue.write_buffer(
                &self.camera_buffer,
                0,
                bytemuck::cast_slice(&[self.camera_uniform]),
            );

            render_pass.draw_path(
                singletons.render_server.get_render_pipeline("sprite v pipeline").unwrap(),
                &tex.mesh.as_ref().unwrap(),
                &self.camera_bind_group,
            );
        }
    }
}

pub trait DrawVector<'a> {
    fn draw_path(
        &mut self,
        pipeline: &'a wgpu::RenderPipeline,
        mesh: &'a VectorMesh,
        camera_bind_group: &'a wgpu::BindGroup,
    );
}

impl<'a, 'b> DrawVector<'b> for wgpu::RenderPass<'a>
    where
        'b: 'a,
{
    fn draw_path(
        &mut self,
        pipeline: &'b wgpu::RenderPipeline,
        mesh: &'b VectorMesh,
        camera_bind_group: &'b wgpu::BindGroup,
    ) {
        self.set_pipeline(&pipeline);

        // Set vertex buffer for VertexInput.
        self.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));

        self.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);

        // Bind camera at 0.
        self.set_bind_group(0, camera_bind_group, &[]);

        self.draw_indexed(0..mesh.index_count, 0, 0..1);
    }
}
