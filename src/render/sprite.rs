use crate::math::transform::Transform2d;
use crate::render::camera::CameraUniform;
use crate::render::vertex::Vertex2d;
use crate::render::{MeshRenderResources, RenderContext, TextureCache, TextureId};
use glam::{Vec2, Vec4};
use std::ops::Range;
use wgpu::BufferAddress;

const QUAD_INDICES: [u32; 6] = [0, 2, 3, 0, 1, 2];

const QUAD_VERTEX_POSITIONS: [Vec2; 4] = [
    Vec2::new(-0.5, 0.5),
    Vec2::new(0.5, 0.5),
    Vec2::new(0.5, -0.5),
    Vec2::new(-0.5, -0.5),
];

#[derive(Debug, Copy, Clone)]
pub struct ExtractedSprite2d {
    pub(crate) transform: Transform2d,
    pub(crate) color: [f32; 4],
    pub(crate) rect: Vec4, // [min_u, min_v, max_u, max_v]
    pub(crate) size: Vec2,
    pub(crate) texture_id: TextureId, // Bindless texture ID.
    pub(crate) centered: bool,
    pub(crate) flip_x: bool,
    pub(crate) flip_y: bool,
    pub(crate) mode: u32,
}

#[derive(Debug, Clone)]
pub struct SpriteBatch {
    pub(crate) index_range: Range<u32>,
    pub(crate) camera_index: u32,
}

/// Persistent GPU resources.
pub struct SpriteRenderResources {
    pub(crate) vertex_buffer: Option<wgpu::Buffer>,
    pub(crate) vertex_buffer_capacity: usize,
    pub(crate) index_buffer: Option<wgpu::Buffer>,
    pub(crate) index_buffer_capacity: usize,
}

impl SpriteRenderResources {
    pub(crate) fn new(_render_server: &RenderContext) -> Self {
        Self {
            vertex_buffer: None,
            vertex_buffer_capacity: 0,
            index_buffer: None,
            index_buffer_capacity: 0,
        }
    }
}

/// Prepare the sprite vertex buffer, index buffer, and the sprite batches.
pub(crate) fn prepare_sprite(
    sprites_2d: &Vec<ExtractedSprite2d>,
    render_resources: &mut SpriteRenderResources,
    _imported_texture_cache: &TextureCache,
    render_server: &RenderContext,
    mesh_render_resources: &MeshRenderResources,
    extracted_cameras: &crate::render::camera::ExtractedCameras,
) -> Vec<SpriteBatch> {
    if sprites_2d.is_empty() {
        return vec![];
    }

    // 找到第一个 D2 类型的相机
    let camera_index = extracted_cameras
        .types
        .iter()
        .position(|t| *t == crate::render::camera::CameraType::D2);

    // 如果没有 2D 相机，则不渲染任何 2D 元素
    let camera_index = match camera_index {
        Some(idx) => idx as u32,
        None => return vec![],
    };

    let total_quads = sprites_2d.len();

    // Allocate the sprite vertex buffer.
    if render_resources.vertex_buffer.is_none()
        || render_resources.vertex_buffer_capacity < total_quads
    {
        render_resources.vertex_buffer =
            Some(render_server.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("ui vertex"),
                size: (size_of::<Vertex2d>() * 4 * total_quads) as BufferAddress,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }));
        render_resources.vertex_buffer_capacity = total_quads;
    }

    let mut all_vertices = Vec::with_capacity(total_quads * 4);
    let mut all_indices = Vec::with_capacity(total_quads * 6);

    // 计算 Z 步长。我们希望越后抽取的元素 Z 越小（越靠近相机，在正交投影中，Z 越小越靠前）。
    let z_step = 1.0 / (sprites_2d.len() as f32 + 1.0);
    let mut current_z = 0.0;

    for e in sprites_2d {
        current_z -= z_step;

        let mut uvs = [
            Vec2::new(e.rect.x, e.rect.w), // BL
            Vec2::new(e.rect.z, e.rect.w), // BR
            Vec2::new(e.rect.z, e.rect.y), // TR
            Vec2::new(e.rect.x, e.rect.y), // TL
        ];

        if e.flip_x {
            uvs.swap(0, 1);
            uvs.swap(2, 3);
        }
        if e.flip_y {
            uvs.swap(0, 3);
            uvs.swap(1, 2);
        }

        let texture_idx = *mesh_render_resources
            .texture_index_map
            .get(&e.texture_id)
            .unwrap_or(&0);
        let vertex_start = all_vertices.len() as u32;

        for i in 0..4 {
            let mut quad_pos = QUAD_VERTEX_POSITIONS[i];
            if !e.centered {
                quad_pos += Vec2::new(0.5, 0.5);
            }
            let new_pos = e.transform.transform_point(&(quad_pos * e.size));

            all_vertices.push(Vertex2d {
                position: [new_pos.x, new_pos.y, current_z],
                uv: uvs[i].into(),
                color: e.color,
                texture_idx,
                mode: e.mode,
            });
        }
        for i in QUAD_INDICES {
            all_indices.push(vertex_start + i);
        }
    }

    // Allocate the sprite index buffer.
    if render_resources.index_buffer_capacity < all_indices.len() {
        render_resources.index_buffer =
            Some(render_server.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("sprite index"),
                size: (size_of::<u32>() * all_indices.len()) as BufferAddress,
                usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }));
        render_resources.index_buffer_capacity = all_indices.len();
    }

    // Write to buffers.
    render_server.queue.write_buffer(
        render_resources.vertex_buffer.as_ref().unwrap(),
        0,
        bytemuck::cast_slice(&all_vertices),
    );
    render_server.queue.write_buffer(
        render_resources.index_buffer.as_ref().unwrap(),
        0,
        bytemuck::cast_slice(&all_indices),
    );

    vec![SpriteBatch {
        index_range: 0..all_indices.len() as u32,
        camera_index,
    }]
}

pub(crate) fn render_sprite<'a, 'b: 'a>(
    batches: &'b Vec<SpriteBatch>,
    render_resources: &'b SpriteRenderResources,
    render_pass: &mut wgpu::RenderPass<'a>,
    camera_bind_group: &'b wgpu::BindGroup,
    bindless_bind_group: &'b wgpu::BindGroup,
    pipeline: &'b wgpu::RenderPipeline,
) {
    if batches.is_empty() {
        return;
    }

    let camera_offset_unit = CameraUniform::get_uniform_offset_unit();

    render_pass.set_pipeline(pipeline);

    render_pass.set_vertex_buffer(
        0,
        render_resources.vertex_buffer.as_ref().unwrap().slice(..),
    );

    render_pass.set_index_buffer(
        render_resources.index_buffer.as_ref().unwrap().slice(..),
        wgpu::IndexFormat::Uint32,
    );

    render_pass.set_bind_group(1, bindless_bind_group, &[]);

    for b in batches {
        render_pass.set_bind_group(0, camera_bind_group, &[camera_offset_unit * b.camera_index]);
        render_pass.draw_indexed(b.index_range.clone(), 0, 0..1);
    }
}
