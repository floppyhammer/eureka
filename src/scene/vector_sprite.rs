extern crate lyon;

use crate::scene::{AsNode, Camera2dUniform, NodeType};
use crate::{Camera2d, InputEvent, RenderServer, Singletons, Vertex};
use cgmath::Vector3;
use lyon::math::point;
use lyon::path::Path;
use lyon::tessellation::*;
use wgpu::util::DeviceExt;

pub struct VectorSprite {
    pub path: Path,
    geometry: VertexBuffers<MyVertex, u16>,

    pub position: cgmath::Vector2<f32>,
    pub size: cgmath::Vector2<f32>,
    pub scale: cgmath::Vector2<f32>,

    pub camera_buffer: wgpu::Buffer,
    pub camera_bind_group: wgpu::BindGroup,

    pub(crate) mesh: VectorMesh,
}

// Let's use our own custom vertex type instead of the default one.
#[derive(Copy, Clone, Debug)]
struct MyVertex {
    position: [f32; 2],
}

impl VectorSprite {
    pub fn new(render_server: &RenderServer) -> VectorSprite {
        let device = &render_server.device;

        // Build a Path.
        let mut builder = Path::builder();
        builder.begin(point(256.0, 256.0));
        builder.line_to(point(128.0, 256.0));
        builder.line_to(point(256.0, 128.0));
        //builder.quadratic_bezier_to(point(200.0, 0.0), point(200.0, 100.0));
        //builder.cubic_bezier_to(point(100.0, 100.0), point(0.0, 100.0), point(0.0, 0.0));
        builder.end(true);
        let path = builder.build();

        // Will contain the result of the tessellation.
        let mut geometry: VertexBuffers<MyVertex, u16> = VertexBuffers::new();
        let mut tessellator = FillTessellator::new();
        {
            // Compute the tessellation.
            tessellator
                .tessellate_path(
                    &path,
                    &FillOptions::default(),
                    &mut BuffersBuilder::new(&mut geometry, |vertex: FillVertex| MyVertex {
                        position: vertex.position().to_array(),
                    }),
                )
                .unwrap();
        }
        // The tessellated geometry is ready to be uploaded to the GPU.
        log::info!(
            "Vector sprite info: {} vertices, {} indices",
            geometry.vertices.len(),
            geometry.indices.len()
        );

        let mut vertices = Vec::new();
        for v in &geometry.vertices {
            vertices.push(VectorVertex {
                position: [v.position[0], v.position[1]],
                color: [1.0, 1.0, 1.0],
            });
        }

        let mut indices = Vec::new();
        for i in &geometry.indices {
            indices.push(*i as i32);
        }

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("vertex buffer for vector sprite")),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("index buffer for vector sprite")),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        let (camera_buffer, camera_bind_group) = render_server.create_camera2d_resources(device);

        let position = cgmath::Vector2::new(0.0 as f32, 0.0);
        let size = cgmath::Vector2::new(128.0 as f32, 128.0);
        let scale = cgmath::Vector2::new(1.0 as f32, 1.0);

        let mesh = VectorMesh {
            name: "".to_string(),
            vertex_buffer,
            index_buffer,
            index_count: indices.len() as u32,
        };

        Self {
            path,
            geometry,
            position,
            size,
            scale,
            camera_buffer,
            camera_bind_group,
            mesh,
        }
    }
}

impl AsNode for VectorSprite {
    fn node_type(&self) -> NodeType {
        NodeType::SpriteVector
    }

    fn input(&mut self, input: &InputEvent) {}

    fn update(
        &mut self,
        dt: f32,
        render_server: &RenderServer,
        singletons: Option<&Singletons>,
    ) {
        let camera = singletons.unwrap().camera2d.as_ref().unwrap();

        let translation = cgmath::Matrix4::from_translation(Vector3::new(-1.0, -1.0, 0.0));

        let scale = cgmath::Matrix4::from_nonuniform_scale(
            1.0 / camera.view_size.x as f32,
            1.0 / camera.view_size.y as f32,
            1.0,
        );

        let mut uniform = Camera2dUniform::new();

        // Note the multiplication direction (left multiplication).
        // So, scale first, translation second.
        uniform.proj = (translation * scale).into();

        // Update camera buffer.
        render_server.queue.write_buffer(&self.camera_buffer, 0, bytemuck::cast_slice(&[uniform]));
    }

    fn draw<'a, 'b: 'a>(
        &'b self,
        render_pass: &mut wgpu::RenderPass<'a>,
        render_server: &'b RenderServer,
        singletons: &'b Singletons,
    ) {
        render_pass.draw_path(
            &render_server.vector_sprite_pipeline,
            &self.mesh,
            &self.camera_bind_group,
        );
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct VectorVertex {
    pub(crate) position: [f32; 2],
    pub(crate) color: [f32; 3],
}

impl Vertex for VectorVertex {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<VectorVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    // Position.
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    // Color.
                    offset: std::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        }
    }
}

pub struct VectorMesh {
    // Mesh name for debugging reason.
    pub name: String,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub index_count: u32,
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
