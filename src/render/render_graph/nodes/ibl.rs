use crate::render::render_backend::PreparedFrame;
use crate::render::render_graph::{standard_resources, FrameContext, Node, NodeResources, ResourceSpec, TextureKey, SamplerKey};
use crate::render::TextureId;
use std::any::Any;

pub struct IBLNode {
    irradiance_pipeline: Option<wgpu::ComputePipeline>,
    brdf_lut_pipeline: Option<wgpu::ComputePipeline>,
    last_skybox_id: Option<TextureId>,
    brdf_lut_done: bool,
}

impl Default for IBLNode {
    fn default() -> Self {
        Self {
            irradiance_pipeline: None,
            brdf_lut_pipeline: None,
            last_skybox_id: None,
            brdf_lut_done: false,
        }
    }
}

impl Node for IBLNode {
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn node_resources(&self, _prepared: &PreparedFrame) -> NodeResources {
        NodeResources::new()
            .persistent_output(
                standard_resources::irradiance_map(),
                ResourceSpec::Texture(TextureKey {
                    width: 64,
                    height: 64,
                    layers: 6,
                    format: Some(wgpu::TextureFormat::Rgba16Float),
                    usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
                    dimension: wgpu::TextureDimension::D2,
                    mip_levels: 1,
                }),
            )
            .persistent_output(
                standard_resources::brdf_lut(),
                ResourceSpec::Texture(TextureKey {
                    width: 512,
                    height: 512,
                    layers: 1,
                    format: Some(wgpu::TextureFormat::Rgba16Float), // Use RGBA as storage RG is less supported
                    usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
                    dimension: wgpu::TextureDimension::D2,
                    mip_levels: 1,
                }),
            )
    }

    fn run(&mut self, context: &mut FrameContext) {
        let device = &context.render_context.device;

        // 1. Prepare Pipelines
        if self.irradiance_pipeline.is_none() {
            let irradiance_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Irradiance Layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::Cube,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::StorageTexture {
                            access: wgpu::StorageTextureAccess::WriteOnly,
                            format: wgpu::TextureFormat::Rgba16Float,
                            view_dimension: wgpu::TextureViewDimension::D2Array,
                        },
                        count: None,
                    },
                ],
            });

            let irradiance_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Irradiance Shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("../../../shaders/ibl_irradiance.wgsl").into()),
            });

            self.irradiance_pipeline = Some(device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Irradiance Pipeline"),
                layout: Some(&device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Irradiance Pipeline Layout"),
                    bind_group_layouts: &[&irradiance_layout],
                    push_constant_ranges: &[],
                })),
                module: &irradiance_shader,
                entry_point: Some("main"),
                cache: None,
                compilation_options: Default::default(),
            }));

            context.backend.add_bind_group_layout("irradiance_layout", irradiance_layout);
        }

        if self.brdf_lut_pipeline.is_none() {
            let brdf_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("BRDF LUT Layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::StorageTexture {
                            access: wgpu::StorageTextureAccess::WriteOnly,
                            format: wgpu::TextureFormat::Rgba16Float,
                            view_dimension: wgpu::TextureViewDimension::D2,
                        },
                        count: None,
                    },
                ],
            });

            let brdf_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("BRDF LUT Shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("../../../shaders/ibl_brdf_lut.wgsl").into()),
            });

            self.brdf_lut_pipeline = Some(device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("BRDF LUT Pipeline"),
                layout: Some(&device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("BRDF LUT Pipeline Layout"),
                    bind_group_layouts: &[&brdf_layout],
                    push_constant_ranges: &[],
                })),
                module: &brdf_shader,
                entry_point: Some("main"),
                cache: None,
                compilation_options: Default::default(),
            }));

            context.backend.add_bind_group_layout("brdf_lut_layout", brdf_layout);
        }

        // 2. Run BRDF LUT (once)
        if !self.brdf_lut_done {
            let brdf_lut = context.texture(&standard_resources::brdf_lut());
            let brdf_layout = context.backend.get_bind_group_layout("brdf_lut_layout").unwrap();
            let brdf_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("BRDF LUT Bind Group"),
                layout: &brdf_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&brdf_lut.view),
                    },
                ],
            });

            let mut cpass = context.encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("BRDF LUT Compute Pass"),
                timestamp_writes: None,
            });
            cpass.set_pipeline(self.brdf_lut_pipeline.as_ref().unwrap());
            cpass.set_bind_group(0, &brdf_bg, &[]);
            cpass.dispatch_workgroups(512 / 8, 512 / 8, 1);
            self.brdf_lut_done = true;
        }

        // 3. Run Irradiance Map (when skybox changes)
        if let Some(skybox_id) = context.backend.sky_imported_resources.texture {
            if self.last_skybox_id != Some(skybox_id) {
                let irradiance_map = context.texture(&standard_resources::irradiance_map());
                let sampler = context.get_sampler(SamplerKey {
                    mag_filter: wgpu::FilterMode::Linear,
                    min_filter: wgpu::FilterMode::Linear,
                    ..Default::default()
                });

                let sky_cache = context.backend.imported_texture_cache.read().unwrap();
                let sky_texture = sky_cache.get(skybox_id).unwrap();

                let irradiance_layout = context.backend.get_bind_group_layout("irradiance_layout").unwrap();

                // We need a special view for irradiance map as it is a D2Array for storage binding
                let irradiance_storage_view = irradiance_map.handle.texture.create_view(&wgpu::TextureViewDescriptor {
                    label: Some("Irradiance Storage View"),
                    format: Some(wgpu::TextureFormat::Rgba16Float),
                    dimension: Some(wgpu::TextureViewDimension::D2Array),
                    base_mip_level: 0,
                    mip_level_count: Some(1),
                    base_array_layer: 0,
                    array_layer_count: Some(6),
                    ..Default::default()
                });

                let irradiance_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("Irradiance Bind Group"),
                    layout: &irradiance_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(&sky_texture.view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::Sampler(&sampler),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: wgpu::BindingResource::TextureView(&irradiance_storage_view),
                        },
                    ],
                });

                let mut cpass = context.encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("Irradiance Compute Pass"),
                    timestamp_writes: None,
                });
                cpass.set_pipeline(self.irradiance_pipeline.as_ref().unwrap());
                cpass.set_bind_group(0, &irradiance_bg, &[]);
                cpass.dispatch_workgroups(64 / 8, 64 / 8, 6);

                self.last_skybox_id = Some(skybox_id);
            }
        }
    }
}
