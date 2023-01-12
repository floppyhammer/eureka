use crate::render::vertex::{VectorVertex, VertexBuffer};
use crate::resources::RenderServer;
use lyon::math::point;
use lyon::path::Path;
use lyon::tessellation::{BuffersBuilder, FillOptions, FillTessellator, FillVertex, VertexBuffers};
use wgpu::util::DeviceExt;

pub struct VectorMesh {
    // Mesh name for debugging reason.
    pub name: String,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub index_count: u32,
}

/// An vector analogy to ImageTexture.
pub struct VectorTexture {
    pub size: (f32, f32),
    // pub paths: Vec<Path>,
    /// CPU mesh.
    geometry: VertexBuffers<VectorVertex, u32>,
    /// GPU mesh.
    pub(crate) mesh: Option<VectorMesh>,
}

impl VectorTexture {
    /// Load from a SVG file.
    fn load_from_file() {}

    pub(crate) fn default() -> Self {
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
        let mut geometry: VertexBuffers<VectorVertex, u32> = VertexBuffers::new();
        let mut tessellator = FillTessellator::new();
        {
            // Compute the tessellation.
            tessellator
                .tessellate_path(
                    &path,
                    &FillOptions::default(),
                    &mut BuffersBuilder::new(&mut geometry, |vertex: FillVertex| VectorVertex {
                        position: vertex.position().to_array(),
                        color: [1.0, 1.0, 1.0],
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

        Self {
            size: (256.0, 256.0),
            geometry,
            mesh: None,
        }
    }

    pub(crate) fn prepare_gpu_resources(&mut self, render_server: &RenderServer) {
        let device = &render_server.device;

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("vertex buffer for vector sprite")),
            contents: bytemuck::cast_slice(&self.geometry.vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("index buffer for vector sprite")),
            contents: bytemuck::cast_slice(&self.geometry.indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        self.mesh = Some(VectorMesh {
            name: "".to_string(),
            vertex_buffer,
            index_buffer,
            index_count: self.geometry.indices.len() as u32,
        });
    }
}

pub struct VectorServer {}

impl VectorServer {
    /// Draw a vector texture.
    fn draw_texture() {}
}
