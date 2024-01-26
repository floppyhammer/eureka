use crate::render::vertex::{VertexBuffer, VertexSky};
use crate::render::{create_render_pipeline, Mesh, RenderServer, Texture, TextureCache, TextureId};
use wgpu::RenderPass;

#[derive(Copy, Clone)]
pub struct ExtractedSky {
    pub texture: TextureId,
}

pub(crate) struct SkyRenderResources {
    pub texture_bind_group_layout: wgpu::BindGroupLayout,
    pub texture: Option<TextureId>,
    pub texture_bind_group: Option<wgpu::BindGroup>,
    pub mesh: Mesh,
    pub pipeline: Option<wgpu::RenderPipeline>,
}

impl SkyRenderResources {
    pub(crate) fn new(render_server: &RenderServer) -> Self {
        let device = &render_server.device;

        let skybox_texture_bind_group_layout = {
            let label = "skybox texture bind group layout";

            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::Cube,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler {
                            0: wgpu::SamplerBindingType::Filtering,
                        },
                        count: None,
                    },
                ],
                label: Some(label),
            })
        };

        let mesh = Mesh::default_skybox(&render_server.device);

        Self {
            texture_bind_group_layout: skybox_texture_bind_group_layout,
            texture_bind_group: None,
            texture: None,
            pipeline: None,
            mesh,
        }
    }
}

pub(crate) fn prepare_sky(
    render_resources: &mut SkyRenderResources,
    render_server: &RenderServer,
    texture_cache: &TextureCache,
    texture_id: &TextureId,
    camera_bind_group_layout: &wgpu::BindGroupLayout,
) {
    let device = &render_server.device;

    if (render_resources.texture.is_none() || render_resources.texture.unwrap() != *texture_id) {
        render_resources.texture = Some(*texture_id);

        let texture = texture_cache.get(*texture_id).unwrap();

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &render_resources.texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&texture.sampler),
                },
            ],
            label: None,
        });

        render_resources.texture_bind_group = Some(bind_group);
    }

    if render_resources.pipeline.is_none() {
        let pipeline_label = "skybox pipeline";

        let pipeline = {
            let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("skybox pipeline layout"),
                bind_group_layouts: &[
                    camera_bind_group_layout,
                    &render_resources.texture_bind_group_layout,
                ],
                push_constant_ranges: &[],
            });

            let shader = wgpu::ShaderModuleDescriptor {
                label: Some("skybox shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/skybox.wgsl").into()),
            };

            create_render_pipeline(
                device,
                &pipeline_layout,
                render_server.surface_config.format,
                Some(Texture::DEPTH_FORMAT),
                &[VertexSky::desc()],
                shader,
                pipeline_label,
                false,
                Some(wgpu::Face::Back),
            )
        };

        render_resources.pipeline = Some(pipeline);
    }
}

pub(crate) fn render_sky<'a, 'b: 'a>(
    camera_bind_group: &'b wgpu::BindGroup,
    render_resources: &'b SkyRenderResources,
    render_pass: &mut RenderPass<'a>,
) {
    if render_resources.pipeline.is_none() {
        return;
    }

    render_pass.set_pipeline(render_resources.pipeline.as_ref().unwrap());

    // Set vertex buffer for VertexInput.
    render_pass.set_vertex_buffer(0, render_resources.mesh.vertex_buffer.slice(..));

    render_pass.set_index_buffer(
        render_resources.mesh.index_buffer.slice(..),
        wgpu::IndexFormat::Uint32,
    );

    // FIXME
    // Set camera uniform.
    render_pass.set_bind_group(0, camera_bind_group, &[0]);

    // Set texture.
    render_pass.set_bind_group(
        1,
        render_resources.texture_bind_group.as_ref().unwrap(),
        &[],
    );

    render_pass.draw_indexed(0..render_resources.mesh.index_count, 0, 0..1);
}
