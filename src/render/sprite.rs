use crate::math::transform::Transform2d;
use crate::render::camera::CameraUniform;
use crate::render::vertex::{Vertex2d, VertexBuffer};
use crate::render::{Mesh, RenderServer, Texture, TextureCache, TextureId, MeshRenderResources};
use glam::Vec2;
use std::mem;
use std::ops::Range;
use wgpu::{BufferAddress};

#[derive(Debug, Copy, Clone)]
pub struct ExtractedSprite2d {
    pub(crate) transform: Transform2d,
    pub(crate) size: Option<(f32, f32)>,
    pub(crate) texture_id: TextureId,
    pub(crate) centered: bool,
    pub(crate) flip_x: bool,
    pub(crate) flip_y: bool,
}

#[derive(Debug, Clone)]
pub struct SpriteBatch {
    pub(crate) index_range: Range<u32>,
    pub(crate) camera_index: u32,
}

pub struct SpriteRenderResources {
    pub(crate) vertex_buffer: Option<wgpu::Buffer>,
    pub(crate) vertex_buffer_capacity: usize,
    pub(crate) index_buffer: Option<wgpu::Buffer>,
    pub(crate) index_buffer_capacity: usize,
}

impl SpriteRenderResources {
    pub(crate) fn new(_render_server: &RenderServer) -> Self {
        Self {
            vertex_buffer: None,
            vertex_buffer_capacity: 0,
            index_buffer: None,
            index_buffer_capacity: 0,
        }
    }
}

const QUAD_INDICES: [u32; 6] = [0, 2, 3, 0, 1, 2];
const QUAD_VERTEX_POSITIONS: [Vec2; 4] = [Vec2::new(-0.5, 0.5), Vec2::new(0.5, 0.5), Vec2::new(0.5, -0.5), Vec2::new(-0.5, -0.5)];
const QUAD_UVS: [Vec2; 4] = [Vec2::new(0., 1.), Vec2::new(1., 1.), Vec2::new(1., 0.), Vec2::new(0., 0.)];

pub(crate) fn prepare_sprite(
    sprites: &Vec<ExtractedSprite2d>,
    render_resources: &mut SpriteRenderResources,
    texture_cache: &TextureCache,
    render_server: &RenderServer,
    mesh_render_resources: &MeshRenderResources,
) -> Vec<SpriteBatch> {
    if sprites.is_empty() { return vec![]; }

    let sprite_count = sprites.len();
    if render_resources.vertex_buffer_capacity < sprite_count {
        render_resources.vertex_buffer = Some(render_server.device.create_buffer(&wgpu::BufferDescriptor { label: Some("sprite vertex"), size: (mem::size_of::<Vertex2d>() * 4 * sprite_count) as BufferAddress, usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false }));
        render_resources.vertex_buffer_capacity = sprite_count;
    }

    let mut all_vertices = Vec::with_capacity(sprite_count * 4);
    let mut all_indices = Vec::with_capacity(sprite_count * 6);

    for e in sprites {
        let mut uvs = QUAD_UVS;
        if e.flip_x { uvs = [uvs[1], uvs[0], uvs[3], uvs[2]]; }
        if e.flip_y { uvs = [uvs[3], uvs[2], uvs[1], uvs[0]]; }

        let size = e.size.unwrap_or_else(|| { let tex = texture_cache.get(e.texture_id).unwrap(); (tex.size.0 as f32, tex.size.1 as f32) });
        let quad_size = Vec2::new(size.0, size.1);
        let texture_idx = *mesh_render_resources.texture_index_map.get(&e.texture_id).unwrap_or(&0);

        let vertex_start = all_vertices.len() as u32;
        for i in 0..4 {
            let mut quad_pos = QUAD_VERTEX_POSITIONS[i];
            if !e.centered { quad_pos += Vec2::new(0.5, 0.5); }
            let new_pos = e.transform.transform_point(&(quad_pos * quad_size));
            all_vertices.push(Vertex2d { position: new_pos.into(), uv: uvs[i].into(), color: [1., 1., 1.], texture_idx });
        }
        for i in QUAD_INDICES { all_indices.push(vertex_start + i); }
    }

    if render_resources.index_buffer_capacity < all_indices.len() {
        render_resources.index_buffer = Some(render_server.device.create_buffer(&wgpu::BufferDescriptor { label: Some("sprite index"), size: (mem::size_of::<u32>() * all_indices.len()) as BufferAddress, usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false }));
        render_resources.index_buffer_capacity = all_indices.len();
    }

    render_server.queue.write_buffer(render_resources.vertex_buffer.as_ref().unwrap(), 0, bytemuck::cast_slice(&all_vertices));
    render_server.queue.write_buffer(render_resources.index_buffer.as_ref().unwrap(), 0, bytemuck::cast_slice(&all_indices));

    vec![SpriteBatch { index_range: 0..all_indices.len() as u32, camera_index: 0 }]
}

pub(crate) fn render_sprite<'a, 'b: 'a>(
    batches: &'b Vec<SpriteBatch>,
    render_resources: &'b SpriteRenderResources,
    render_pass: &mut wgpu::RenderPass<'a>,
    camera_bind_group: &'b wgpu::BindGroup,
    bindless_bind_group: &'b wgpu::BindGroup,
    pipeline: &'b wgpu::RenderPipeline,
) {
    if batches.is_empty() { return; }
    let offset_unit = CameraUniform::get_uniform_offset_unit();
    render_pass.set_pipeline(pipeline);
    render_pass.set_vertex_buffer(0, render_resources.vertex_buffer.as_ref().unwrap().slice(..));
    render_pass.set_index_buffer(render_resources.index_buffer.as_ref().unwrap().slice(..), wgpu::IndexFormat::Uint32);
    render_pass.set_bind_group(1, bindless_bind_group, &[]);

    for b in batches {
        render_pass.set_bind_group(0, camera_bind_group, &[offset_unit * b.camera_index]);
        render_pass.draw_indexed(b.index_range.clone(), 0, 0..1);
    }
}
