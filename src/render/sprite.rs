use crate::render::bind_group::{BindGroupCache, BindGroupId};
use crate::render::camera::CameraUniform;
use crate::render::vertex::{Vertex2d, VertexBuffer};
use crate::render::{create_render_pipeline, Mesh, RenderServer, Texture, TextureCache, TextureId};
use std::collections::HashMap;
use wgpu::{Device, SamplerBindingType};

/// Minimal data for rendering a sprite.
#[derive(Debug, Copy, Clone)]
pub struct ExtractedSprite2d {
    pub(crate) render_params: CameraUniform,
    pub(crate) texture_id: TextureId,
}

/// All sprite related resources.
pub struct SpriteRenderResources {
    // TODO: remove camera data.
    pub(crate) camera_bind_group_layout: wgpu::BindGroupLayout,
    pub(crate) camera_bind_group: Option<wgpu::BindGroup>,
    pub(crate) camera_uniform_buffer: Option<wgpu::Buffer>,
    pub(crate) camera_buffer_capacity: usize,

    pub(crate) texture_bind_group_layout: wgpu::BindGroupLayout,
    pub(crate) texture_bind_group_cache: HashMap<TextureId, wgpu::BindGroup>,

    pub(crate) pipeline: Option<wgpu::RenderPipeline>,

    /// Shared 2D rect mesh for sprites.
    pub(crate) mesh: Mesh,
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
            camera_bind_group_layout,
            camera_uniform_buffer: None,
            camera_bind_group: None,
            camera_buffer_capacity: 0,
            texture_bind_group_layout,
            texture_bind_group_cache: HashMap::new(),
            pipeline: Some(pipeline),
            mesh: Mesh::default_2d(&render_server.device),
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
