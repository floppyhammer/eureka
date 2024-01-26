// extern crate lyon;
//
// use std::any::Any;
// use std::fs;
//
// use crate::math::transform::Transform2d;
// use crate::scene::{AsNode, NodeType};
// use crate::{Camera2d, RenderServer, Singletons};
// use cgmath::{Vector2, Vector3};
// use lyon::math::point;
// use lyon::path::Path;
// use lyon::tessellation::*;
// use usvg::TreeParsing;
// use wgpu::util::DeviceExt;
// use crate::render::draw_command::DrawCommands;
// use crate::render::vector_texture::VectorTexture;
//
// pub struct VectorSprite {
//     pub transform: Transform2d,
//     pub size: cgmath::Vector2<f32>,
//
//     svg_data: Option<usvg::Tree>,
//
//     need_to_rebuild: bool,
// }
//
// impl VectorSprite {
//     pub fn default() -> Self {
//         Self {
//             transform: Transform2d::default(),
//             size: Vector2::new(0.0, 0.0),
//             svg_data: None,
//             need_to_rebuild: false,
//         }
//     }
//
//     pub fn from_file<P: AsRef<std::path::Path>>(path: P) -> Self {
//         let data = fs::read(path).expect("No SVG file found!");
//
//         let tree: usvg::Tree = usvg::Tree::from_data(&data, &usvg::Options::default()).unwrap();
//
//         Self {
//             transform: Transform2d::default(),
//             size: Vector2::new(tree.size.width() as f32, tree.size.height() as f32),
//             svg_data: Some(tree),
//             need_to_rebuild: true,
//         }
//     }
// }
//
// impl AsNode for VectorSprite {
//     fn node_type(&self) -> NodeType {
//         NodeType::VectorSprite
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
//     fn ready(&mut self) {}
//
//     fn update(&mut self, dt: f32, singletons: &mut Singletons) {
//         // let translation = cgmath::Matrix4::from_translation(Vector3::new(-1.0, 1.0, 0.0));
//         //
//         // let scale = cgmath::Matrix4::from_nonuniform_scale(
//         //     1.0 / camera_info.view_size.x as f32 * 2.0,
//         //     -1.0 / camera_info.view_size.y as f32 * 2.0,
//         //     1.0,
//         // );
//         //
//         // // Note the multiplication direction (left multiplication).
//         // // So, scale first, translation second.
//         // self.camera_uniform.proj = (translation * scale).into();
//     }
//
//     fn draw(&self, draw_commands: &mut DrawCommands,
//     ) {}
// }
//
// pub trait DrawVector<'a> {
//     fn draw_path(
//         &mut self,
//         pipeline: &'a wgpu::RenderPipeline,
//         mesh: &'a VectorMesh,
//         camera_bind_group: &'a wgpu::BindGroup,
//     );
// }
//
// impl<'a, 'b> DrawVector<'b> for wgpu::RenderPass<'a>
//     where
//         'b: 'a,
// {
//     fn draw_path(
//         &mut self,
//         pipeline: &'b wgpu::RenderPipeline,
//         mesh: &'b VectorMesh,
//         camera_bind_group: &'b wgpu::BindGroup,
//     ) {
//         self.set_pipeline(&pipeline);
//
//         // Set vertex buffer for VertexInput.
//         self.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
//
//         self.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
//
//         // Bind camera at 0.
//         self.set_bind_group(0, camera_bind_group, &[]);
//
//         self.draw_indexed(0..mesh.index_count, 0, 0..1);
//     }
// }
