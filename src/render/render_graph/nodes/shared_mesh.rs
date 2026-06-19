use crate::render::camera::CameraUniform;
use crate::render::light::{CascadeUniform, LightUniform, MAX_SHADOWED_POINT_LIGHTS};
use crate::render::render_backend::PreparedFrame;
use crate::render::render_graph::{
    standard_resources, FrameContext, NodeResources, ResourceSpec, SamplerKey, TextureKey,
};
use crate::render::Texture;

/// 声明 Mesh 渲染通用的资源依赖
pub fn common_mesh_resources(resources: NodeResources, prepared: &PreparedFrame) -> NodeResources {
    let camera_buffer_size = CameraUniform::get_uniform_offset_unit() * 16;
    let material_buffer_size =
        prepared.material_uniforms.len() * size_of::<crate::render::material::MaterialUniform>();

    resources
        .input(
            standard_resources::camera_buffer(),
            ResourceSpec::buffer(camera_buffer_size as u64, wgpu::BufferUsages::UNIFORM),
        )
        .input(
            standard_resources::shadow_cascade_buffer(),
            ResourceSpec::buffer(
                size_of::<CascadeUniform>() as u64,
                wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            ),
        )
        .input(
            standard_resources::material_storage_buffer(),
            ResourceSpec::buffer(
                material_buffer_size as u64,
                wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            ),
        )
        .optional_input(
            standard_resources::point_shadow_map(),
            ResourceSpec::Texture(TextureKey {
                width: 512,
                height: 512,
                format: Some(Texture::DEPTH_FORMAT),
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                layers: (MAX_SHADOWED_POINT_LIGHTS * 6) as u32,
                mip_levels: 1,
                dimension: wgpu::TextureDimension::D2,
            }),
        )
        .optional_input(
            standard_resources::directional_shadow_map(),
            ResourceSpec::Texture(TextureKey {
                width: 2048,
                height: 2048,
                format: Some(Texture::DEPTH_FORMAT),
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                layers: 3,
                mip_levels: 1,
                dimension: wgpu::TextureDimension::D2,
            }),
        )
        .optional_input(
            standard_resources::ssao_blur(),
            ResourceSpec::Texture(TextureKey::d2(
                0,
                0,
                wgpu::TextureFormat::R8Unorm,
                wgpu::TextureUsages::TEXTURE_BINDING,
            )),
        )
        .input(
            standard_resources::light_uniform_buffer(),
            ResourceSpec::buffer(
                size_of::<LightUniform>() as u64,
                wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            ),
        )
        .input(
            standard_resources::point_light_storage_buffer(),
            ResourceSpec::buffer(
                (size_of::<crate::render::light::PointLightUniform>() * 1024) as u64,
                wgpu::BufferUsages::STORAGE,
            ),
        )
        .input(
            standard_resources::light_grid_buffer(),
            ResourceSpec::buffer(0, wgpu::BufferUsages::STORAGE),
        )
        .input(
            standard_resources::light_index_list_buffer(),
            ResourceSpec::buffer(0, wgpu::BufferUsages::STORAGE),
        )
        .input(
            standard_resources::cluster_config_buffer(),
            ResourceSpec::buffer(0, wgpu::BufferUsages::UNIFORM),
        )
        .input(
            standard_resources::volumetric_lighting_texture(),
            ResourceSpec::Texture(TextureKey {
                width: crate::render::light::CLUSTER_GRID_SIZE[0],
                height: crate::render::light::CLUSTER_GRID_SIZE[1],
                layers: crate::render::light::CLUSTER_GRID_SIZE[2],
                format: Some(wgpu::TextureFormat::Rgba16Float),
                usage: wgpu::TextureUsages::TEXTURE_BINDING,
                dimension: wgpu::TextureDimension::D3,
                mip_levels: 1,
            }),
        )
        .optional_input(
            standard_resources::irradiance_map(),
            ResourceSpec::Texture(TextureKey {
                width: 64,
                height: 64,
                layers: 6,
                format: Some(wgpu::TextureFormat::Rgba16Float),
                usage: wgpu::TextureUsages::TEXTURE_BINDING,
                dimension: wgpu::TextureDimension::D2,
                mip_levels: 1,
            }),
        )
        .optional_input(
            standard_resources::brdf_lut(),
            ResourceSpec::Texture(TextureKey::d2(
                512,
                512,
                wgpu::TextureFormat::Rgba16Float,
                wgpu::TextureUsages::TEXTURE_BINDING,
            )),
        )
}

pub fn get_or_create_light_layout(context: &mut FrameContext) -> wgpu::BindGroupLayout {
    let device = &context.render_context.device;
    if let Some(layout) = context
        .backend
        .get_bind_group_layout("light_bind_group_layout")
    {
        return layout.clone();
    }

    let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Common Light Layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
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
                    view_dimension: wgpu::TextureViewDimension::D2Array,
                    sample_type: wgpu::TextureSampleType::Depth,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 3,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 4,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    multisampled: false,
                    view_dimension: wgpu::TextureViewDimension::CubeArray,
                    sample_type: wgpu::TextureSampleType::Depth,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 5,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    multisampled: false,
                    view_dimension: wgpu::TextureViewDimension::D2,
                    sample_type: wgpu::TextureSampleType::Float { filterable: false },
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 6,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    multisampled: false,
                    view_dimension: wgpu::TextureViewDimension::Cube,
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 7,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 8,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 9,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 10,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 11,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 12,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    multisampled: false,
                    view_dimension: wgpu::TextureViewDimension::D3,
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 13,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    multisampled: false,
                    view_dimension: wgpu::TextureViewDimension::Cube,
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 14,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    multisampled: false,
                    view_dimension: wgpu::TextureViewDimension::D2,
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                },
                count: None,
            },
        ],
    });

    context
        .backend
        .add_bind_group_layout("light_bind_group_layout", layout.clone());
    layout
}

pub fn get_mesh_bind_groups(
    context: &mut FrameContext,
) -> (wgpu::BindGroup, wgpu::BindGroup, wgpu::BindGroup) {
    let light_bind_group_layout = get_or_create_light_layout(context);

    // 1. Camera
    let camera_buffer = context.buffer(&standard_resources::camera_buffer());
    let camera_layout = context
        .backend
        .get_bind_group_layout("camera_bind_group_layout")
        .unwrap()
        .clone();
    let camera_bg =
        context.create_bind_group("camera_bind_group_layout", vec![camera_buffer.id], |ctx| {
            ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &camera_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &camera_buffer.buffer,
                        offset: 0,
                        size: Some(
                            wgpu::BufferSize::new(size_of::<CameraUniform>() as u64).unwrap(),
                        ),
                    }),
                }],
                label: None,
            })
        });

    // 2. Light
    let ssao_blur = context.texture(&standard_resources::ssao_blur());
    let dir_shadow = context.texture(&standard_resources::directional_shadow_map());
    let point_shadow = context.texture(&standard_resources::point_shadow_map());
    let volumetric_tex = context.texture(&standard_resources::volumetric_lighting_texture());
    let light_uniform = context.buffer(&standard_resources::light_uniform_buffer());
    let cascade_buffer = context.buffer(&standard_resources::shadow_cascade_buffer());
    let pt_storage = context.buffer(&standard_resources::point_light_storage_buffer());
    let grid = context.buffer(&standard_resources::light_grid_buffer());
    let indices = context.buffer(&standard_resources::light_index_list_buffer());
    let config = context.buffer(&standard_resources::cluster_config_buffer());

    let irradiance = context.texture(&standard_resources::irradiance_map());
    let brdf_lut = context.texture(&standard_resources::brdf_lut());

    let irradiance_view = irradiance.get_view(&wgpu::TextureViewDescriptor {
        dimension: Some(wgpu::TextureViewDimension::Cube),
        ..Default::default()
    });

    let cascade_view = dir_shadow.get_view(&wgpu::TextureViewDescriptor {
        dimension: Some(wgpu::TextureViewDimension::D2Array),
        aspect: wgpu::TextureAspect::DepthOnly,
        array_layer_count: Some(3),
        ..Default::default()
    });
    let pt_shadow_view = point_shadow.get_view(&wgpu::TextureViewDescriptor {
        dimension: Some(wgpu::TextureViewDimension::CubeArray),
        aspect: wgpu::TextureAspect::DepthOnly,
        array_layer_count: Some(MAX_SHADOWED_POINT_LIGHTS as u32 * 6),
        ..Default::default()
    });
    let vol_view = volumetric_tex.get_view(&wgpu::TextureViewDescriptor {
        dimension: Some(wgpu::TextureViewDimension::D3),
        ..Default::default()
    });
    let (sky_view, sky_view_id) = if let Some(id) = context.backend.sky_imported_resources.texture {
        let cache = context.backend.imported_texture_cache.read().unwrap();
        let t = cache.get(id).unwrap();
        (t.view.clone(), t.view_id)
    } else {
        (context.backend.dummy_cube_view.clone(), 0)
    };

    let shadow_sampler = context.get_sampler(SamplerKey {
        compare: Some(wgpu::CompareFunction::LessEqual),
        ..Default::default()
    });
    let skybox_sampler = context.get_sampler(SamplerKey::default());

    let light_bg = context.create_bind_group(
        "light_bind_group_layout",
        vec![
            light_uniform.id,
            cascade_view.1,
            pt_shadow_view.1,
            ssao_blur.view_id,
            sky_view_id,
            vol_view.1,
            cascade_buffer.id,
            pt_storage.id,
            grid.id,
            indices.id,
            config.id,
            irradiance_view.1,
            brdf_lut.view_id,
        ],
        |ctx| {
            ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &light_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: light_uniform.buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(&cascade_view.0),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::Sampler(&shadow_sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: cascade_buffer.buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: wgpu::BindingResource::TextureView(&pt_shadow_view.0),
                    },
                    wgpu::BindGroupEntry {
                        binding: 5,
                        resource: wgpu::BindingResource::TextureView(&ssao_blur.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 6,
                        resource: wgpu::BindingResource::TextureView(&sky_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 7,
                        resource: wgpu::BindingResource::Sampler(&skybox_sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 8,
                        resource: pt_storage.buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 9,
                        resource: grid.buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 10,
                        resource: indices.buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 11,
                        resource: config.buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 12,
                        resource: wgpu::BindingResource::TextureView(&vol_view.0),
                    },
                    wgpu::BindGroupEntry {
                        binding: 13,
                        resource: wgpu::BindingResource::TextureView(&irradiance_view.0),
                    },
                    wgpu::BindGroupEntry {
                        binding: 14,
                        resource: wgpu::BindingResource::TextureView(&brdf_lut.view),
                    },
                ],
                label: None,
            })
        },
    );

    let mat_buf = context.buffer(&standard_resources::material_storage_buffer());
    let bindless_bg = context
        .get_bind_group("bindless_bind_group_layout", vec![mat_buf.id])
        .clone();

    (camera_bg, light_bg, bindless_bg)
}
