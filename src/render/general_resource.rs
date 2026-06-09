use crate::render::{MeshRenderResources, RenderContext};

pub struct GeneralRenderResources {
    pub(crate) dummy_2d_view: wgpu::TextureView,
    pub(crate) dummy_cube_view: wgpu::TextureView,
    pub(crate) dummy_sampler: wgpu::Sampler,
    pub(crate) bindless_bind_group_layout: wgpu::BindGroupLayout,
    pub(crate) bindless_bind_group: Option<wgpu::BindGroup>,
}

impl GeneralRenderResources {
    pub(crate) fn new(render_server: &RenderContext) -> Self {
        let bindless_bind_group_layout =
            render_server
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: true },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Texture {
                                multisampled: false,
                                view_dimension: wgpu::TextureViewDimension::D2,
                                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            },
                            count: std::num::NonZeroU32::new(1024),
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 2,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                            count: None,
                        },
                    ],
                    label: Some("bindless bind group layout"),
                });

        let dummy_2d_view = {
            let texture = render_server
                .device
                .create_texture(&wgpu::TextureDescriptor {
                    label: Some("dummy 2d"),
                    size: wgpu::Extent3d {
                        width: 1,
                        height: 1,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: wgpu::TextureFormat::Rgba8UnormSrgb,
                    usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                    view_formats: &[],
                });
            texture.create_view(&wgpu::TextureViewDescriptor {
                label: Some("dummy 2d view"),
                dimension: Some(wgpu::TextureViewDimension::D2),
                ..Default::default()
            })
        };

        let dummy_cube_view = {
            let texture = render_server
                .device
                .create_texture(&wgpu::TextureDescriptor {
                    label: Some("dummy cube"),
                    size: wgpu::Extent3d {
                        width: 1,
                        height: 1,
                        depth_or_array_layers: 6,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: wgpu::TextureFormat::Rgba8UnormSrgb,
                    usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                    view_formats: &[],
                });
            texture.create_view(&wgpu::TextureViewDescriptor {
                label: Some("dummy cube view"),
                dimension: Some(wgpu::TextureViewDimension::Cube),
                ..Default::default()
            })
        };

        let dummy_sampler = render_server
            .device
            .create_sampler(&wgpu::SamplerDescriptor {
                label: Some("mesh bindless sampler"),
                address_mode_u: wgpu::AddressMode::Repeat,
                address_mode_v: wgpu::AddressMode::Repeat,
                address_mode_w: wgpu::AddressMode::Repeat,
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                mipmap_filter: wgpu::FilterMode::Linear,
                ..Default::default()
            });


        Self {
            dummy_2d_view,
            dummy_cube_view,
            dummy_sampler,
            bindless_bind_group_layout,
            bindless_bind_group: None,
        }
    }
}
