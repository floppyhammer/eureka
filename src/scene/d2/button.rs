// use crate::math::transform::Transform2d;
// use crate::render::atlas::{AtlasMode, DrawAtlas};
// use crate::render::render_server::RenderServer;
// use crate::scene::vector_sprite::VectorSprite;
// use crate::scene::{CameraInfo, Label, NodeType};
// use crate::window::{InputEvent, InputServer};
// use crate::{AsNode, Atlas, AtlasInstance, Singletons, TextServer, Texture};
// use cgmath::{Point2, Vector2, Vector3, Vector4};
// use image::DynamicImage;
// use lyon::geom::Transform;
// use std::any::Any;
//
// pub struct Button {
//     label: Label,
//
//     pub transform: Transform2d,
//
//     pub(crate) size: Vector2<f32>,
//
//     hovered: bool,
//     pressed: bool,
//
//     sprite_v: VectorSprite,
// }
//
// impl Button {
//     pub fn new(render_server: &RenderServer) -> Button {
//         let size = Vector2::new(128.0_f32, 128.0);
//
//         let mut label = Label::new(render_server);
//         label.set_text("button".to_string());
//
//         Self {
//             label,
//             transform: Transform2d::default(),
//             size,
//             hovered: false,
//             pressed: false,
//             sprite_v: VectorSprite::new(render_server),
//         }
//     }
//
//     pub fn set_text(&mut self, text: String) {
//         self.label.set_text(text);
//     }
// }
//
// impl AsNode for Button {
//     fn node_type(&self) -> NodeType {
//         NodeType::Label
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
//     fn input(&mut self, input_event: &mut InputEvent, input_server: &mut InputServer) {
//         match input_event {
//             InputEvent::MouseButton(_) => {}
//             InputEvent::MouseMotion(args) => {
//                 self.hovered = false;
//                 if args.position.0 > self.transform.position.x
//                     && args.position.0 < (self.transform.position.x + self.size.x)
//                 {
//                     if args.position.1 > self.transform.position.y
//                         && args.position.1 < (self.transform.position.y + self.size.y)
//                     {
//                         self.hovered = true;
//                     }
//                 }
//             }
//             InputEvent::MouseScroll(_) => {}
//             InputEvent::Key(_) => {}
//             InputEvent::Invalid => {}
//         }
//     }
//
//     fn update(&mut self, dt: f32, camera_info: &CameraInfo, singletons: &mut Singletons) {
//         self.label.transform.position = self.transform.position;
//         if self.hovered {
//             self.label.set_text("hovered".to_string());
//         } else {
//             self.label.set_text("normal".to_string());
//         }
//         self.label.update(dt, camera_info, singletons);
//     }
//
//     fn draw<'a, 'b: 'a>(
//         &'b self,
//         render_pass: &mut wgpu::RenderPass<'a>,
//         camera_info: &'b CameraInfo,
//         singletons: &'b mut Singletons,
//     ) {
//         self.label.draw(render_pass, camera_info, singletons);
//     }
// }
