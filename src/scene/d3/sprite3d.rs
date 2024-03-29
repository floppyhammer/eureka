// use crate::pbr::Material2d;
// use crate::render::{Mesh, Texture, TextureId};
// use crate::scene::{AsNode, CameraInfo, CameraUniform, NodeType};
// use crate::{Camera2d, RenderServer, SamplerBindingType, Singletons, Zero};
// use cgmath::{InnerSpace, Rotation3, Vector3, Quaternion, Deg, Matrix4};
// use std::any::Any;
// use std::mem;
// use wgpu::util::DeviceExt;
// use wgpu::BufferAddress;
//
// #[repr(C)]
// #[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
// pub struct SpriteParamsUniform {
//     model_matrix: [[f32; 4]; 4],
//     billboard_mode: f32,
//     pad0: f32,
//     pad1: f32,
//     pad2: f32,
// }
//
// impl SpriteParamsUniform {
//     fn default() -> Self {
//         Self {
//             model_matrix: [[0.0; 4]; 4],
//             billboard_mode: 0.0,
//             pad0: 0.0,
//             pad1: 0.0,
//             pad2: 0.0,
//         }
//     }
// }
//
// pub struct Sprite3d {
//     pub name: String,
//
//     pub position: Vector3<f32>,
//     pub rotation: Quaternion<f32>,
//     pub scale: Vector3<f32>,
//
//     pub billboard_mode: BillboardMode,
//
//     pub texture: Option<TextureId>,
//     // pub bind_group: wgpu::BindGroup,
//     //
//     pub params_uniform: SpriteParamsUniform,
//     // pub params_buffer: wgpu::Buffer,
//     // pub params_bind_group: wgpu::BindGroup,
//
//     pub mesh: Mesh,
// }
//
// impl Sprite3d {
//     pub(crate) fn new(render_server: &RenderServer, texture_id: TextureId) -> Sprite3d {
//         let device = &render_server.device;
//
//         let position = Vector3::new(0.0f32, 0.0, 0.0);
//         let rotation = if position.is_zero() {
//             // This is needed so an object at (0, 0, 0) won't get scaled to zero
//             // as Quaternions can affect scale if they're not created correctly.
//             Quaternion::from_axis_angle(Vector3::unit_z(), Deg(0.0))
//         } else {
//             Quaternion::from_axis_angle(position.normalize(), Deg(45.0))
//         };
//         let scale = Vector3::new(1.0f32, 1.0, 1.0);
//
//         let mesh = Mesh::default_3d(device);
//
//         let texture = render_server.texture_cache.get(texture_id).unwrap();
//
//         let texture_bind_group = texture.create_bind_group(device, render_server);
//
//         let billboard_mode = BillboardMode::Spherical;
//
//         // Create a buffer for the parameters.
//         // ------------------------------------------
//         let params_buffer = device.create_buffer(&wgpu::BufferDescriptor {
//             label: Some("sprite params buffer"),
//             size: mem::size_of::<SpriteParamsUniform>() as BufferAddress,
//             usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
//             mapped_at_creation: false,
//         });
//
//         let params_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
//             layout: render_server
//                 .get_bind_group_layout("sprite3d params bind group layout")
//                 .unwrap(),
//             entries: &[wgpu::BindGroupEntry {
//                 binding: 0,
//                 resource: params_buffer.as_entire_binding(),
//             }],
//             label: Some("sprite params bind group"),
//         });
//         // ------------------------------------------
//
//         Self {
//             name: "".to_string(),
//             position,
//             rotation,
//             scale,
//             billboard_mode,
//             texture: Some(texture_id),
//             // texture_bind_group,
//             params_uniform: SpriteParamsUniform::default(),
//             // params_buffer,
//             // params_bind_group,
//             mesh,
//         }
//     }
// }
//
// #[derive(PartialEq)]
// pub enum BillboardMode {
//     None,
//     Spherical,
// }
//
// impl AsNode for Sprite3d {
//     fn node_type(&self) -> NodeType {
//         NodeType::Sprite3d
//     }
//
//     fn as_any(&self) -> &dyn Any {
//         self
//     }
//
//     fn as_any_mut(&mut self) -> &mut dyn Any {
//         self
//     }
//
//     fn update(&mut self, dt: f32, camera_info: &CameraInfo, singletons: &mut Singletons) {
//         self.params_uniform = SpriteParamsUniform {
//             model_matrix: Matrix4::from_translation(self.position).into(),
//             billboard_mode: if self.billboard_mode == BillboardMode::Spherical {
//                 1.0
//             } else {
//                 0.0
//             },
//             pad0: 0.0,
//             pad1: 0.0,
//             pad2: 0.0,
//         };
//     }
//
//     fn draw<'a, 'b: 'a>(
//         &'b self,
//         render_pass: &mut wgpu::RenderPass<'a>,
//         camera_info: &'b CameraInfo,
//         singletons: &'b mut Singletons,
//     ) {
//
//         // queue_sprite_3d();
//         // // Update buffer.
//         // singletons.render_server.queue.write_buffer(
//         //     &self.params_buffer,
//         //     0,
//         //     bytemuck::cast_slice(&[self.params_uniform]),
//         // );
//         //
//         // match &camera_info.bind_group {
//         //     Some(b) => {
//         //         render_pass.draw_sprite(
//         //             singletons
//         //                 .render_server
//         //                 .get_render_pipeline("sprite3d pipeline")
//         //                 .unwrap(),
//         //             &self.mesh,
//         //             &self.texture_bind_group,
//         //             b,
//         //             &self.params_bind_group,
//         //         );
//         //     }
//         //     None => {}
//         // }
//     }
// }
