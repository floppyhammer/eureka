use crate::render::{Mesh, RenderContext, TextureCache, TextureId};
use wgpu::RenderPass;

#[derive(Copy, Clone)]
pub struct ExtractedSky {
    pub texture: TextureId,
}

pub(crate) struct SkyRenderResources {
    pub texture_bind_group_layout: wgpu::BindGroupLayout,
    pub texture: Option<TextureId>,
    pub texture_bind_group: Option<wgpu::BindGroup>,
    pub mesh: Option<Mesh>,
}

impl SkyRenderResources {
    pub(crate) fn new(render_server: &RenderContext) -> Self {
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

        Self {
            texture_bind_group_layout: skybox_texture_bind_group_layout,
            texture_bind_group: None,
            texture: None,
            mesh: None,
        }
    }
}

pub(crate) fn prepare_sky(
    render_resources: &mut SkyRenderResources,
    render_server: &RenderContext,
    texture_cache: &TextureCache,
    texture_id: &TextureId,
    _camera_bind_group_layout: &wgpu::BindGroupLayout,
    mesh_allocator: &mut crate::render::allocator::MeshAllocator,
) {
    let device = &render_server.device;

    if render_resources.mesh.is_none() {
        render_resources.mesh = Some(Mesh::default_skybox(&render_server.queue, mesh_allocator));
    }

    if render_resources.texture.is_none() || render_resources.texture.unwrap() != *texture_id {
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
}

pub(crate) fn render_sky<'a, 'b: 'a>(
    camera_bind_group: &'b wgpu::BindGroup,
    render_resources: &'b SkyRenderResources,
    render_pass: &mut RenderPass<'a>,
    mesh_allocator: &'b crate::render::allocator::MeshAllocator,
    pipeline: &'b wgpu::RenderPipeline,
) {
    if render_resources.mesh.is_none() {
        return;
    }

    let mesh = render_resources.mesh.as_ref().unwrap();
    render_pass.set_pipeline(pipeline);

    render_pass.set_vertex_buffer(0, mesh_allocator.sky_vertex_buffer.slice(..));
    render_pass.set_index_buffer(mesh_allocator.sky_index_buffer.slice(..), wgpu::IndexFormat::Uint32);

    render_pass.set_bind_group(0, camera_bind_group, &[0]);
    render_pass.set_bind_group(1, render_resources.texture_bind_group.as_ref().unwrap(), &[]);

    render_pass.draw_indexed(0..mesh.index_count, 0, 0..1);
}
