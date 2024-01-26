// use crate::render::vertex::{VertexBuffer};
// use crate::render::RenderServer;
// use lyon::geom::Point;
// use lyon::math::point;
// use lyon::path::builder::Build;
// use lyon::path::path::Builder;
// use lyon::path::Path;
// use lyon::tessellation::{
//     BuffersBuilder, FillOptions, FillTessellator, FillVertex, StrokeOptions, StrokeTessellator,
//     StrokeVertex, VertexBuffers,
// };
// use std::fs;
// use usvg::{Paint, TreeParsing};
// use wgpu::util::DeviceExt;
//
// #[repr(C)]
// #[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
// pub(crate) struct VectorVertex {
//     pub(crate) position: [f32; 2],
//     pub(crate) color: [f32; 3],
// }
//
// impl VertexBuffer for VectorVertex {
//     fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
//         wgpu::VertexBufferLayout {
//             array_stride: std::mem::size_of::<VectorVertex>() as wgpu::BufferAddress,
//             step_mode: wgpu::VertexStepMode::Vertex,
//             attributes: &[
//                 // Position.
//                 wgpu::VertexAttribute {
//                     offset: 0,
//                     shader_location: 0,
//                     format: wgpu::VertexFormat::Float32x2,
//                 },
//                 // Color.
//                 wgpu::VertexAttribute {
//                     offset: std::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
//                     shader_location: 1,
//                     format: wgpu::VertexFormat::Float32x3,
//                 },
//             ],
//         }
//     }
// }
//
// pub struct VectorMesh {
//     // For debugging.
//     pub name: String,
//     pub vertex_buffer: wgpu::Buffer,
//     pub index_buffer: wgpu::Buffer,
//     pub index_count: u32,
// }
//
// /// An vector analogy to ImageTexture.
// pub struct VectorTexture {
//     pub size: (f32, f32),
//     // pub paths: Vec<Path>,
//     /// CPU mesh.
//     geometry: VertexBuffers<VectorVertex, u32>,
//     vertices: Vec<VectorVertex>,
//     indices: Vec<u32>,
//     max_index: u32,
//     /// GPU mesh.
//     pub(crate) mesh: Option<VectorMesh>,
//     builder: Builder,
// }
//
// impl VectorTexture {
//     /// Load from a SVG file.
//     pub fn from_tree(tree: &usvg::Tree) -> Self {
//         let mut tex = VectorTexture::new((tree.size.width() as f32, tree.size.height() as f32));
//
//         let root = &tree.root;
//
//         for kid in root.children() {
//             tex.process_node(&kid);
//         }
//
//         // tex.build();
//
//         tex.prepare_gpu_resources(render_server);
//
//         tex
//     }
//
//     pub(crate) fn new(size: (f32, f32)) -> Self {
//         // // Build a Path.
//         let mut builder = Path::builder();
//
//         let mut geometry: VertexBuffers<VectorVertex, u32> = VertexBuffers::new();
//
//         Self {
//             size,
//             geometry,
//             vertices: vec![],
//             indices: vec![],
//             max_index: 0,
//             mesh: None,
//             builder,
//         }
//     }
//
//     pub(crate) fn build(&mut self) {
//         let mut builder = std::mem::replace(&mut self.builder, Path::builder());
//
//         let path = builder.build();
//
//         // Will contain the result of the tessellation.
//         let mut tessellator = FillTessellator::new();
//
//         // Compute the tessellation.
//         tessellator
//             .tessellate_path(
//                 &path,
//                 &FillOptions::default(),
//                 &mut BuffersBuilder::new(&mut self.geometry, |vertex: FillVertex| VectorVertex {
//                     position: vertex.position().to_array(),
//                     color: [1.0, 1.0, 1.0],
//                 }),
//             )
//             .unwrap();
//
//         // The tessellated geometry is ready to be uploaded to the GPU.
//         log::info!(
//             "Vector sprite info: {} vertices, {} indices",
//             self.geometry.vertices.len(),
//             self.geometry.indices.len()
//         );
//     }
//
//     pub(crate) fn prepare_gpu_resources(&mut self, render_server: &RenderServer) {
//         let device = &render_server.device;
//
//         let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
//             label: Some(&format!("vertex buffer for vector sprite")),
//             contents: bytemuck::cast_slice(&self.vertices),
//             usage: wgpu::BufferUsages::VERTEX,
//         });
//
//         let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
//             label: Some(&format!("index buffer for vector sprite")),
//             contents: bytemuck::cast_slice(&self.indices),
//             usage: wgpu::BufferUsages::INDEX,
//         });
//
//         self.mesh = Some(VectorMesh {
//             name: "".to_string(),
//             vertex_buffer,
//             index_buffer,
//             index_count: self.indices.len() as u32,
//         });
//     }
//
//     fn process_node(&mut self, node: &usvg::Node) {
//         match *node.borrow() {
//             usvg::NodeKind::Group(_) => {
//                 for kid in node.children() {
//                     self.process_node(&kid);
//                 }
//             }
//             usvg::NodeKind::Path(ref path) => {
//                 let mut builder = Path::builder();
//
//                 let mut subpath_ended = false;
//
//                 for segment in path.data.segments() {
//                     match segment {
//                         usvg::PathSegment::MoveTo { x, y } => {
//                             builder.begin(point(x as f32, y as f32));
//                         }
//                         usvg::PathSegment::LineTo { x, y } => {
//                             builder.line_to(point(x as f32, y as f32));
//                         }
//                         usvg::PathSegment::CurveTo {
//                             x1,
//                             y1,
//                             x2,
//                             y2,
//                             x,
//                             y,
//                         } => {
//                             builder.cubic_bezier_to(
//                                 point(x1 as f32, y1 as f32),
//                                 point(x2 as f32, y2 as f32),
//                                 point(x as f32, y as f32),
//                             );
//                         }
//                         usvg::PathSegment::ClosePath => {
//                             builder.close();
//                             subpath_ended = true;
//                         }
//                     }
//                 }
//
//                 if !subpath_ended {
//                     builder.end(false);
//                 }
//
//                 let lyon_path = builder.build();
//
//                 let mut geometry: VertexBuffers<VectorVertex, u32> = VertexBuffers::new();
//
//                 if let Some(ref fill) = path.fill {
//                     // Will contain the result of the tessellation.
//                     let mut tessellator = FillTessellator::new();
//
//                     match fill.paint {
//                         Paint::Color(color) => {
//                             // Compute the tessellation.
//                             let result = tessellator.tessellate_path(
//                                 &lyon_path,
//                                 &FillOptions::default(),
//                                 &mut BuffersBuilder::new(&mut geometry, |vertex: FillVertex| {
//                                     VectorVertex {
//                                         position: vertex.position().to_array(),
//                                         color: [
//                                             color.red as f32 / 255.0,
//                                             color.green as f32 / 255.0,
//                                             color.blue as f32 / 255.0,
//                                         ],
//                                     }
//                                 }),
//                             );
//                             assert!(result.is_ok());
//                         }
//                         Paint::LinearGradient(_) => {}
//                         Paint::RadialGradient(_) => {}
//                         Paint::Pattern(_) => {}
//                     }
//                 }
//
//                 if let Some(ref stroke) = path.stroke {
//                     // Create the tessellator.
//                     let mut tessellator = StrokeTessellator::new();
//
//                     match stroke.paint {
//                         Paint::Color(color) => {
//                             let mut options = StrokeOptions::default()
//                                 .with_line_width(stroke.width.get() as f32)
//                                 .with_line_join(convert_line_join(stroke.linejoin))
//                                 .with_line_cap(convert_line_cap(stroke.linecap));
//                             if stroke.linejoin == usvg::LineJoin::Miter {
//                                 options = options.with_miter_limit(stroke.miterlimit.get() as f32);
//                             }
//
//                             // Compute the tessellation.
//                             let result = tessellator.tessellate_path(
//                                 &lyon_path,
//                                 &options,
//                                 &mut BuffersBuilder::new(&mut geometry, |vertex: StrokeVertex| {
//                                     VectorVertex {
//                                         position: vertex.position().to_array(),
//                                         color: [
//                                             color.red as f32 / 255.0,
//                                             color.green as f32 / 255.0,
//                                             color.blue as f32 / 255.0,
//                                         ],
//                                     }
//                                 }),
//                             );
//                             assert!(result.is_ok());
//                         }
//                         Paint::LinearGradient(_) => {}
//                         Paint::RadialGradient(_) => {}
//                         Paint::Pattern(_) => {}
//                     }
//                 }
//
//                 let vertex_count = geometry.vertices.len();
//
//                 self.vertices.extend(geometry.vertices);
//
//                 for index in geometry.indices {
//                     self.indices.push(index + self.max_index);
//                 }
//
//                 self.max_index += vertex_count as u32;
//             }
//             _ => {}
//         }
//     }
// }
//
// fn convert_line_join(usvg_line_join: usvg::LineJoin) -> lyon::path::LineJoin {
//     match usvg_line_join {
//         usvg::LineJoin::Bevel => lyon::path::LineJoin::Bevel,
//         usvg::LineJoin::Miter => lyon::path::LineJoin::Miter,
//         usvg::LineJoin::Round => lyon::path::LineJoin::Round,
//     }
// }
//
// fn convert_line_cap(usvg_line_cap: usvg::LineCap) -> lyon::path::LineCap {
//     match usvg_line_cap {
//         usvg::LineCap::Round => lyon::path::LineCap::Round,
//         usvg::LineCap::Butt => lyon::path::LineCap::Butt,
//         usvg::LineCap::Square => lyon::path::LineCap::Square,
//     }
// }
