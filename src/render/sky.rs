use crate::render::vertex::VertexSky;
use crate::render::{RenderContext, TextureId};

#[derive(Copy, Clone)]
pub struct ExtractedSky {
    pub texture: TextureId,
}

#[derive(Clone)]
pub(crate) struct SkyImportedResources {
    // Static vertex & index data
    pub sky_vertex_buffer: Option<wgpu::Buffer>,
    pub sky_index_buffer: Option<wgpu::Buffer>,
    pub index_count: u32,
    pub texture: Option<TextureId>,
}

impl SkyImportedResources {
    pub(crate) fn new() -> Self {
        Self {
            sky_vertex_buffer: None,
            sky_index_buffer: None,
            index_count: 0,
            texture: None,
        }
    }
}

pub(crate) fn prepare_sky(
    imported_resources: &mut SkyImportedResources,
    render_context: &RenderContext,
    texture_id: &TextureId,
) {
    // 一次写入天空盒顶点数据
    if imported_resources.sky_vertex_buffer.is_none() {
        // Dedicated buffers for skybox (small and unique layout)
        let sky_vertex_buffer = render_context
            .device
            .create_buffer(&wgpu::BufferDescriptor {
                label: Some("Skybox Vertex Buffer"),
                size: 1024,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });

        let sky_index_buffer = render_context
            .device
            .create_buffer(&wgpu::BufferDescriptor {
                label: Some("Skybox Index Buffer"),
                size: 1024,
                usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });

        let vertices = [
            VertexSky {
                position: [-1.0, -1.0, -1.0],
            },
            VertexSky {
                position: [1.0, -1.0, -1.0],
            },
            VertexSky {
                position: [1.0, 1.0, -1.0],
            },
            VertexSky {
                position: [-1.0, 1.0, -1.0],
            },
            VertexSky {
                position: [-1.0, -1.0, 1.0],
            },
            VertexSky {
                position: [1.0, -1.0, 1.0],
            },
            VertexSky {
                position: [1.0, 1.0, 1.0],
            },
            VertexSky {
                position: [-1.0, 1.0, 1.0],
            },
        ];
        let indices = [
            0, 1, 2, 2, 3, 0, 4, 6, 5, 6, 4, 7, 2, 6, 7, 2, 7, 3, 1, 5, 6, 1, 6, 2, 3, 7, 0, 4, 0,
            7, 5, 1, 4, 4, 1, 0,
        ];

        render_context
            .queue
            .write_buffer(&sky_vertex_buffer, 0, bytemuck::cast_slice(&vertices));
        render_context
            .queue
            .write_buffer(&sky_index_buffer, 0, bytemuck::cast_slice(&indices));

        imported_resources.sky_vertex_buffer = Some(sky_vertex_buffer);
        imported_resources.sky_index_buffer = Some(sky_index_buffer);
        imported_resources.index_count = indices.len() as u32;
    }

    if imported_resources.texture.is_none() || imported_resources.texture.unwrap() != *texture_id {
        imported_resources.texture = Some(*texture_id);
    }
}
