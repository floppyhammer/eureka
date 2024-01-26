use crate::math::alignup_u32;
use crate::math::transform::Transform2d;
use crate::render::bind_group::{BindGroupCache, BindGroupId};
use crate::render::camera::CameraUniform;
use crate::render::vertex::{Vertex2d, VertexBuffer};
use crate::render::{create_render_pipeline, Mesh, RenderServer, Texture, TextureCache, TextureId};
use cgmath::{ElementWise, Vector2};
use naga::TypeInner::Vector;
use std::collections::HashMap;
use std::mem;
use std::ops::Range;
use wgpu::{BufferAddress, Device, DynamicOffset, SamplerBindingType};

/// Minimal data for rendering a sprite.
#[derive(Debug, Copy, Clone)]
pub struct ExtractedSprite2d {
    pub(crate) transform: Transform2d,
    pub(crate) size: (f32, f32),
    pub(crate) texture_id: TextureId,
    pub(crate) centered: bool,
    pub(crate) flip_x: bool,
    pub(crate) flip_y: bool,
}

#[derive(Debug, Clone)]
pub struct SpriteBatch {
    pub(crate) texture_id: Option<TextureId>,
    pub(crate) index_range: Range<u32>,
    pub(crate) camera_index: u32,
}

/// All sprite related resources.
pub struct SpriteRenderResources {
    pub(crate) texture_bind_group_layout: wgpu::BindGroupLayout,
    pub(crate) texture_bind_group_cache: HashMap<TextureId, wgpu::BindGroup>,

    pub(crate) pipeline: Option<wgpu::RenderPipeline>,

    // A big buffer for all sprites. Use index range to use different parts of the data.
    pub(crate) vertex_buffer: Option<wgpu::Buffer>,
    pub(crate) vertex_buffer_capacity: usize,
    pub(crate) index_buffer: Option<wgpu::Buffer>,
    pub(crate) index_buffer_capacity: usize,
}

impl SpriteRenderResources {
    pub(crate) fn new(render_server: &RenderServer) -> Self {
        let camera_bind_group_layout =
            render_server
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    entries: &[wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: true,
                            min_binding_size: None,
                        },
                        count: None,
                    }],
                    label: Some("sprite2d camera bind group layout"),
                });

        let texture_bind_group_layout =
            render_server
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Texture {
                                multisampled: false,
                                view_dimension: wgpu::TextureViewDimension::D2,
                                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Sampler {
                                0: SamplerBindingType::Filtering,
                            },
                            count: None,
                        },
                    ],
                    label: Some("sprite2d texture bind group layout"),
                });

        let pipeline_label = "sprite2d pipeline";

        let pipeline = {
            // Set up resource pipeline layout using bind group layouts.
            let pipeline_layout =
                render_server
                    .device
                    .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                        label: Some("sprite2d pipeline layout"),
                        bind_group_layouts: &[
                            &camera_bind_group_layout,
                            &texture_bind_group_layout,
                        ],
                        push_constant_ranges: &[],
                    });

            // Shader descriptor, not a shader module yet.
            let shader = wgpu::ShaderModuleDescriptor {
                label: Some("sprite2d shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/sprite.wgsl").into()),
            };

            create_render_pipeline(
                &render_server.device,
                &pipeline_layout,
                render_server.surface_config.format,
                Some(Texture::DEPTH_FORMAT),
                &[Vertex2d::desc()],
                shader,
                pipeline_label,
                true,
                Some(wgpu::Face::Back),
            )
        };

        Self {
            texture_bind_group_layout,
            texture_bind_group_cache: HashMap::new(),
            pipeline: Some(pipeline),
            vertex_buffer: None,
            vertex_buffer_capacity: 0,
            index_buffer: None,
            index_buffer_capacity: 0,
        }
    }

    pub fn add_texture_bind_group(
        &mut self,
        device: &Device,
        texture_cache: &TextureCache,
        texture_id: TextureId,
    ) {
        let bind_group = self.texture_bind_group_cache.get(&texture_id);
        if bind_group.is_some() {
            return;
        }

        let texture = texture_cache.get(texture_id).unwrap();

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &self.texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&(texture.view)),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&texture.sampler),
                },
            ],
            label: None,
        });

        self.texture_bind_group_cache.insert(texture_id, bind_group);
    }

    pub fn get_texture_bind_group(&self, texture_id: TextureId) -> &wgpu::BindGroup {
        return self.texture_bind_group_cache.get(&texture_id).unwrap();
    }

    pub fn remove_texture_bind_group(&mut self, texture_id: TextureId) {
        self.texture_bind_group_cache.remove(&texture_id);
    }
}

pub trait DrawSprite2d<'a> {
    fn draw_sprite2d(
        &mut self,
        pipeline: &'a wgpu::RenderPipeline,
        mesh: &'a Mesh,
        texture_bind_group: &'a wgpu::BindGroup,
        camera_bind_group: &'a wgpu::BindGroup,
    );
}

impl<'a, 'b> DrawSprite2d<'b> for wgpu::RenderPass<'a>
where
    'b: 'a, // This means 'b must outlive 'a.
{
    fn draw_sprite2d(
        &mut self,
        pipeline: &'b wgpu::RenderPipeline,
        mesh: &'b Mesh,
        texture_bind_group: &'b wgpu::BindGroup,
        camera_bind_group: &'b wgpu::BindGroup,
    ) {
        self.set_pipeline(&pipeline);

        // Set vertex buffer for VertexInput.
        self.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));

        self.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);

        // Set camera group.
        self.set_bind_group(0, &camera_bind_group, &[]);

        // Set texture group.
        self.set_bind_group(1, &texture_bind_group, &[]);

        self.draw_indexed(0..mesh.index_count, 0, 0..1);
    }
}

pub trait DrawSprite3d<'a> {
    fn draw_sprite(
        &mut self,
        pipeline: &'a wgpu::RenderPipeline,
        mesh: &'a Mesh,
        texture_bind_group: &'a wgpu::BindGroup,
        camera_bind_group: &'a wgpu::BindGroup,
        params_bind_group: &'a wgpu::BindGroup,
    );
}

impl<'a, 'b> DrawSprite3d<'b> for wgpu::RenderPass<'a>
where
    'b: 'a, // This means 'b must outlive 'a.
{
    fn draw_sprite(
        &mut self,
        pipeline: &'b wgpu::RenderPipeline,
        mesh: &'b Mesh,
        texture_bind_group: &'b wgpu::BindGroup,
        camera_bind_group: &'b wgpu::BindGroup,
        params_bind_group: &'b wgpu::BindGroup,
    ) {
        self.set_pipeline(&pipeline);

        // Set vertex buffer for VertexInput.
        self.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));

        self.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);

        // Set camera group.
        self.set_bind_group(0, &camera_bind_group, &[]);

        // Set texture group.
        self.set_bind_group(1, &texture_bind_group, &[]);

        // Set params group.
        self.set_bind_group(2, &params_bind_group, &[]);

        self.draw_indexed(0..mesh.index_count, 0, 0..1);
    }
}

/// CCW is front.
const QUAD_INDICES: [u32; 6] = [0, 2, 3, 0, 1, 2];

const QUAD_VERTEX_POSITIONS: [Vector2<f32>; 4] = [
    Vector2::new(-0.5, 0.5),
    Vector2::new(0.5, 0.5),
    Vector2::new(0.5, -0.5),
    Vector2::new(-0.5, -0.5),
];

const QUAD_UVS: [Vector2<f32>; 4] = [
    Vector2::new(0., 1.),
    Vector2::new(1., 1.),
    Vector2::new(1., 0.),
    Vector2::new(0., 0.),
];

pub(crate) fn prepare_sprite(
    sprites: &Vec<ExtractedSprite2d>,
    render_resources: &mut SpriteRenderResources,
    texture_cache: &TextureCache,
    render_server: &RenderServer,
    camera_bind_group_layout: &wgpu::BindGroupLayout,
) -> Vec<SpriteBatch> {
    if sprites.is_empty() {
        return vec![];
    }

    let sprite_count = sprites.len();

    // Reallocate the vertex buffer.
    if render_resources.vertex_buffer_capacity < sprite_count {
        let buffer = render_server.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("sprite vertex buffer (unique)"),
            size: (mem::size_of::<Vertex2d>() * 4 * sprite_count) as BufferAddress,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        render_resources.vertex_buffer_capacity = sprite_count;
        render_resources.vertex_buffer = Some(buffer);
    }

    for sprite in sprites {
        render_resources.add_texture_bind_group(
            &render_server.device,
            &texture_cache,
            sprite.texture_id,
        );
    }

    // Prepare data for the vertex buffer.
    let mut all_vertices = vec![];
    let mut all_indices = vec![];

    let mut batches = vec![];
    let mut current_batch = SpriteBatch {
        texture_id: None,
        index_range: Default::default(),
        camera_index: 0,
    };

    for e in sprites {
        let transform = e.transform;
        let size = e.size;

        // Calculate vertex data for this item.

        // Default UVs.
        let mut uvs = QUAD_UVS;

        // Consider flip.
        if (e.flip_x) {
            uvs = [uvs[1], uvs[0], uvs[3], uvs[2]];
        }
        if (e.flip_y) {
            uvs = [uvs[3], uvs[2], uvs[1], uvs[0]];
        }

        // By default, the size of the quad is the size of the texture.
        let quad_size = Vector2::new(size.0, size.1);

        let mut vertices = vec![];
        vertices.reserve(4);

        // Apply size and global transform.
        for i in 0..QUAD_VERTEX_POSITIONS.len() {
            let mut quad_pos = QUAD_VERTEX_POSITIONS[i];
            if !e.centered {
                quad_pos += Vector2::new(0.5, 0.5);
            }
            let new_pos = transform.transform_point(&quad_pos.mul_element_wise(quad_size));

            vertices.push(Vertex2d {
                position: new_pos.into(),
                uv: uvs[i].into(),
                color: [1., 1., 1.],
            });
        }

        let mut need_new_batch = false;
        if current_batch.texture_id.is_some() {
            if current_batch.texture_id.unwrap() != e.texture_id {
                need_new_batch = true
            }
        } else {
            need_new_batch = true
        }

        if (need_new_batch) {
            if current_batch.texture_id.is_some() {
                current_batch.index_range.end = all_indices.len() as u32;
                batches.push(current_batch.clone());
            }

            current_batch.texture_id = Some(e.texture_id);
            current_batch.index_range.start = all_indices.len() as u32;
        }

        for i in QUAD_INDICES {
            all_indices.push(all_vertices.len() as u32 + i);
        }

        for v in vertices {
            all_vertices.push(v);
        }
    }

    // Add the last batch.
    current_batch.index_range.end = all_indices.len() as u32;
    batches.push(current_batch.clone());

    let index_count = all_indices.len();

    if render_resources.index_buffer_capacity < index_count {
        let buffer = render_server.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("sprite index buffer (unique)"),
            size: (mem::size_of::<u32>() * index_count) as BufferAddress,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        render_resources.index_buffer = Some(buffer);
        render_resources.index_buffer_capacity = index_count;
    }

    // Write the vertex buffer.
    if render_resources.vertex_buffer.is_some() {
        render_server.queue.write_buffer(
            render_resources.vertex_buffer.as_ref().unwrap(),
            0,
            bytemuck::cast_slice(all_vertices.as_slice()),
        );
    }

    if render_resources.index_buffer.is_some() {
        render_server.queue.write_buffer(
            render_resources.index_buffer.as_ref().unwrap(),
            0,
            bytemuck::cast_slice(all_indices.as_slice()),
        );
    }

    batches
}

pub(crate) fn render_sprite<'a, 'b: 'a>(
    batches: &'b Vec<SpriteBatch>,
    render_resources: &'b SpriteRenderResources,
    render_pass: &mut wgpu::RenderPass<'a>,
    camera_bind_group: &'b wgpu::BindGroup,
) {
    if batches.is_empty() {
        return;
    }

    let offset_unit = CameraUniform::get_uniform_offset_unit();

    // Draw sprites batch by batch.
    for b in batches {
        let uniform_offset = offset_unit * b.camera_index;

        let texture_bind_group = render_resources.get_texture_bind_group(b.texture_id.unwrap());

        let pipeline = render_resources.pipeline.as_ref().unwrap();

        render_pass.set_pipeline(pipeline);

        // Set vertex buffer for VertexInput.
        render_pass.set_vertex_buffer(
            0,
            render_resources.vertex_buffer.as_ref().unwrap().slice(..),
        );

        render_pass.set_index_buffer(
            render_resources.index_buffer.as_ref().unwrap().slice(..),
            wgpu::IndexFormat::Uint32,
        );

        // Set camera group.
        render_pass.set_bind_group(0, camera_bind_group, &[uniform_offset]);

        // Set texture group.
        render_pass.set_bind_group(1, texture_bind_group, &[]);

        render_pass.draw_indexed(b.index_range.clone(), 0, 0..1);
    }
}
