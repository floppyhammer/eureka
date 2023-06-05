use std::collections::HashMap;
use crate::render::atlas::{AtlasInstance, AtlasInstanceRaw, AtlasParamsUniform};
use crate::render::vertex::{VectorVertex, Vertex2d, Vertex3d, VertexBuffer, VertexSky};
use crate::scene::CameraUniform;
use crate::{resources, scene, Camera2d, Camera3d, Light, SamplerBindingType, Texture};
use bevy_ecs::system::Resource;
use cgmath::Point2;
use std::mem;
use std::time::Instant;
use wgpu::util::DeviceExt;
use wgpu::PolygonMode::Point;
use wgpu::{BufferAddress, TextureFormat};

#[derive(Resource)]
pub struct RenderServer {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub surface: wgpu::Surface,
    pub surface_config: wgpu::SurfaceConfiguration,

    bind_group_layout_cache: HashMap<&'static str, wgpu::BindGroupLayout>,
    render_pipeline_cache: HashMap<&'static str, wgpu::RenderPipeline>,
}

impl RenderServer {
    pub(crate) fn new(
        surface: wgpu::Surface,
        surface_config: wgpu::SurfaceConfiguration,
        device: wgpu::Device,
        queue: wgpu::Queue,
    ) -> Self {
        let now = Instant::now();

        let mut bind_group_layout_cache = HashMap::new();

        // Create various bind group layouts, which are used to create bind groups.
        // ------------------------------------------------------------------
        {
            let label = "camera bind group layout";

            let camera_bind_group_layout =
                device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    entries: &[wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    }],
                    label: Some(label),
                });

            bind_group_layout_cache.insert(label, camera_bind_group_layout);
        }

        // Mesh textures.
        {
            let label = "mesh textures bind group layout";

            let mesh_textures_bind_group_layout =
                device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    entries: &[
                        // Diffuse texture.
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
                        // Normal texture.
                        wgpu::BindGroupLayoutEntry {
                            binding: 2,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Texture {
                                multisampled: false,
                                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                                view_dimension: wgpu::TextureViewDimension::D2,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 3,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Sampler {
                                0: SamplerBindingType::Filtering,
                            },
                            count: None,
                        },
                    ],
                    label: Some(label),
                });

            bind_group_layout_cache.insert(label, mesh_textures_bind_group_layout);
        }

        {
            let label = "light bind group layout";

            let light_bind_group_layout =
                device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    entries: &[wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    }],
                    label: Some(label),
                });

            bind_group_layout_cache.insert(label, light_bind_group_layout);
        }

        {
            let label = "sprite texture bind group layout";

            let sprite_texture_bind_group_layout =
                device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
                    label: Some(label),
                });

            bind_group_layout_cache.insert(label, sprite_texture_bind_group_layout);
        }

        {
            let label = "skybox texture bind group layout";

            let skybox_texture_bind_group_layout =
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
                                0: SamplerBindingType::Filtering,
                            },
                            count: None,
                        },
                    ],
                    label: Some(label),
                });

            bind_group_layout_cache.insert(label, skybox_texture_bind_group_layout);
        }

        {
            let label = "sprite3d params bind group layout";

            let bind_group_layout =
                device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    entries: &[wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    }],
                    label: Some(label),
                });

            bind_group_layout_cache.insert(label, bind_group_layout);
        }

        {
            let label = "atlas params bind group layout";

            let bind_group_layout =
                device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    entries: &[wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    }],
                    label: Some(label),
                });

            bind_group_layout_cache.insert(label, bind_group_layout);
        }
        // ------------------------------------------------------------------

        let mut render_pipeline_cache = HashMap::new();

        let mut server = Self {
            device,
            queue,
            surface,
            surface_config,

            bind_group_layout_cache,
            render_pipeline_cache,
        };

        server.build_atlas_pipeline();
        server.build_gizmo_pipeline();
        server.build_sprite2d_pipeline();
        server.build_sprite3d_pipeline();
        server.build_skybox_pipeline();
        server.build_sprite_v_pipeline();

        let elapsed_time = now.elapsed();
        log::info!(
            "Render server setup took {} milliseconds",
            elapsed_time.as_millis()
        );

        server
    }

    pub fn build_sprite2d_pipeline(&mut self) {
        let pipeline_label = "sprite2d pipeline";

        let pipeline = {
            // Set up resource pipeline layout using bind group layouts.
            let pipeline_layout = self.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("sprite2d pipeline layout"),
                bind_group_layouts: &[
                    self.get_bind_group_layout("camera bind group layout").unwrap(),
                    self.get_bind_group_layout("sprite texture bind group layout").unwrap(),
                ],
                push_constant_ranges: &[],
            });

            // Shader descriptor, not a shader module yet.
            let shader = wgpu::ShaderModuleDescriptor {
                label: Some("sprite2d shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/blit.wgsl").into()),
            };

            create_render_pipeline(
                &self.device,
                &pipeline_layout,
                self.surface_config.format,
                Some(resources::texture::Texture::DEPTH_FORMAT),
                &[Vertex2d::desc()],
                shader,
                pipeline_label,
                true,
                Some(wgpu::Face::Back),
            )
        };

        self.render_pipeline_cache.insert(pipeline_label, pipeline);
    }

    pub fn build_mesh_pipeline(&mut self) {
        let pipeline_label = "mesh pipeline";

        let pipeline = {
            // Set up resource pipeline layout using bind group layouts.
            let pipeline_layout = self.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("model pipeline layout"),
                bind_group_layouts: &[
                    self.get_bind_group_layout("mesh textures").unwrap(),
                    self.get_bind_group_layout("camera").unwrap(),
                    self.get_bind_group_layout("light").unwrap(),
                ],
                push_constant_ranges: &[],
            });

            // Shader descriptor, not a shader module yet.
            let shader = wgpu::ShaderModuleDescriptor {
                label: Some("model shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/model.wgsl").into()),
            };

            create_render_pipeline(
                &self.device,
                &pipeline_layout,
                self.surface_config.format,
                Some(resources::texture::Texture::DEPTH_FORMAT),
                &[Vertex3d::desc(), scene::model::InstanceRaw::desc()],
                shader,
                pipeline_label,
                false,
                Some(wgpu::Face::Back),
            )
        };

        self.render_pipeline_cache.insert(pipeline_label, pipeline);
    }

    pub fn build_sprite3d_pipeline(&mut self) {
        let pipeline_label = "sprite3d pipeline";

        let pipeline = {
            // Set up resource pipeline layout using bind group layouts.
            let pipeline_layout = self.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("sprite3d pipeline layout"),
                bind_group_layouts: &[
                    self.get_bind_group_layout("camera bind group layout").unwrap(),
                    self.get_bind_group_layout("sprite texture bind group layout").unwrap(),
                    self.get_bind_group_layout("sprite3d params bind group layout").unwrap(),
                ],
                push_constant_ranges: &[],
            });

            // Shader descriptor, not a shader module yet.
            let shader = wgpu::ShaderModuleDescriptor {
                label: Some("sprite3d shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/sprite3d.wgsl").into()),
            };

            // FIXME(floppyhammer): Transparency
            create_render_pipeline(
                &self.device,
                &pipeline_layout,
                self.surface_config.format,
                Some(resources::texture::Texture::DEPTH_FORMAT),
                &[Vertex3d::desc()],
                shader,
                pipeline_label,
                false,
                Some(wgpu::Face::Back),
            )
        };

        self.render_pipeline_cache.insert(pipeline_label, pipeline);
    }

    pub fn build_sprite_v_pipeline(&mut self) {
        let pipeline_label = "sprite v pipeline";

        // Vector sprite pipeline.
        let pipeline = {
            // Set up resource pipeline layout using bind group layouts.
            let pipeline_layout = self.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("sprite v pipeline layout"),
                bind_group_layouts: &[self.get_bind_group_layout("camera bind group layout").unwrap(),
                ],
                push_constant_ranges: &[],
            });

            // Shader descriptor, not a shader module yet.
            let shader = wgpu::ShaderModuleDescriptor {
                label: Some("sprite v shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/vector.wgsl").into()),
            };

            create_render_pipeline(
                &self.device,
                &pipeline_layout,
                self.surface_config.format,
                Some(resources::texture::Texture::DEPTH_FORMAT),
                &[VectorVertex::desc()],
                shader,
                pipeline_label,
                true,
                None,
            )
        };

        self.render_pipeline_cache.insert(pipeline_label, pipeline);
    }

    pub fn build_atlas_pipeline(&mut self) {
        let pipeline_label = "atlas pipeline";

        let pipeline = {
            let pipeline_layout = self.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("atlas pipeline layout"),
                bind_group_layouts: &[
                    self.get_bind_group_layout("atlas params bind group layout").unwrap(),
                    self.get_bind_group_layout("sprite texture bind group layout").unwrap(),
                ],
                push_constant_ranges: &[],
            });

            let shader = wgpu::ShaderModuleDescriptor {
                label: Some("atlas shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/atlas.wgsl").into()),
            };
            let shader_module = self.device.create_shader_module(shader);

            self.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(pipeline_label),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader_module,
                    entry_point: "vs_main",
                    buffers: &[AtlasInstanceRaw::desc()],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader_module,
                    entry_point: "fs_main",
                    targets: &[Some(wgpu::ColorTargetState {
                        format: self.surface_config.format,
                        blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleStrip, // Has to be triangle strip.
                    front_face: wgpu::FrontFace::Cw,
                    cull_mode: None,
                    ..Default::default()
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: resources::texture::Texture::DEPTH_FORMAT,
                    depth_write_enabled: false,
                    depth_compare: wgpu::CompareFunction::Less,
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
            })
        };

        self.render_pipeline_cache.insert(pipeline_label, pipeline);
    }

    pub fn build_skybox_pipeline(&mut self) {
        let pipeline_label = "skybox pipeline";

        let pipeline = {
            let pipeline_layout = self.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("skybox pipeline layout"),
                bind_group_layouts: &[
                    self.get_bind_group_layout("camera bind group layout").unwrap(),
                    self.get_bind_group_layout("skybox texture bind group layout").unwrap(),
                ],
                push_constant_ranges: &[],
            });

            let shader = wgpu::ShaderModuleDescriptor {
                label: Some("skybox shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/skybox.wgsl").into()),
            };

            create_render_pipeline(
                &self.device,
                &pipeline_layout,
                self.surface_config.format,
                Some(resources::texture::Texture::DEPTH_FORMAT),
                &[VertexSky::desc()],
                shader,
                pipeline_label,
                false,
                Some(wgpu::Face::Back),
            )
        };

        self.render_pipeline_cache.insert(pipeline_label, pipeline);
    }

    pub fn build_gizmo_pipeline(&mut self) {
        let pipeline_label = "gizmo pipeline";

        let pipeline = {
            let pipeline_layout = self.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("gizmo pipeline layout"),
                bind_group_layouts: &[self.get_bind_group_layout("camera bind group layout").unwrap()],
                push_constant_ranges: &[],
            });

            let shader = wgpu::ShaderModuleDescriptor {
                label: Some("gizmo shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/gizmo.wgsl").into()),
            };
            let shader_module = self.device.create_shader_module(shader);

            self.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(pipeline_label),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader_module,
                    entry_point: "vs_main_grid",
                    buffers: &[],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader_module,
                    entry_point: "fs_main_grid",
                    targets: &[Some(wgpu::ColorTargetState {
                        format: self.surface_config.format,
                        blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleStrip, // Has to be triangle strip.
                    front_face: wgpu::FrontFace::Cw,
                    cull_mode: None,
                    ..Default::default()
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: resources::texture::Texture::DEPTH_FORMAT,
                    depth_write_enabled: false,
                    depth_compare: wgpu::CompareFunction::Less,
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
            })
        };

        self.render_pipeline_cache.insert(pipeline_label, pipeline);
    }

    pub fn get_bind_group_layout(&self, label: &'static str) -> Option<&wgpu::BindGroupLayout> {
        return self.bind_group_layout_cache.get(label);
    }

    pub fn get_render_pipeline(&self, label: &'static str) -> Option<&wgpu::RenderPipeline> {
        self.render_pipeline_cache.get(label)
    }

    pub(crate) fn create_camera2d_resources(
        &self,
        device: &wgpu::Device,
    ) -> (wgpu::Buffer, wgpu::BindGroup) {
        // Create a buffer for the camera uniform.
        let camera_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("camera2d buffer"),
            size: mem::size_of::<CameraUniform>() as BufferAddress,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: self.get_bind_group_layout("camera bind group layout").unwrap(),
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
            label: Some("camera2d bind group"),
        });

        (camera_buffer, camera_bind_group)
    }

    pub fn create_sprite2d_bind_group(&self, texture: &Texture) -> wgpu::BindGroup {
        self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: self.get_bind_group_layout("sprite texture bind group layout").unwrap(),
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
        })
    }

    pub fn create_atlas_params_bind_group(&self) -> (wgpu::Buffer, wgpu::BindGroup) {
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("atlas params uniform buffer"),
            size: mem::size_of::<AtlasParamsUniform>() as BufferAddress,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: self.get_bind_group_layout("atlas params bind group layout").unwrap(),
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
            label: Some("atlas params bind group"),
        });

        (buffer, bind_group)
    }
}

/// Set up resource pipeline using the pipeline layout.
pub fn create_render_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    color_format: wgpu::TextureFormat,
    depth_format: Option<wgpu::TextureFormat>,
    vertex_layouts: &[wgpu::VertexBufferLayout],
    shader: wgpu::ShaderModuleDescriptor,
    label: &str,
    transparency: bool,
    cull_mode: Option<wgpu::Face>,
) -> wgpu::RenderPipeline {
    // Create actual shader module using the shader descriptor.
    let shader = device.create_shader_module(shader);

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(label),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: "vs_main",
            buffers: vertex_layouts,
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: "fs_main",
            targets: &[Some(wgpu::ColorTargetState {
                format: color_format,
                blend: Some(if !transparency {
                    wgpu::BlendState {
                        alpha: wgpu::BlendComponent::REPLACE,
                        color: wgpu::BlendComponent::REPLACE,
                    }
                } else {
                    wgpu::BlendState {
                        alpha: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::One,
                            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                            operation: wgpu::BlendOperation::Add,
                        },
                        color: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::One,
                            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                            operation: wgpu::BlendOperation::Add,
                        },
                    }
                }),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode,
            // Setting this to anything other than Fill requires Features::NON_FILL_POLYGON_MODE
            polygon_mode: wgpu::PolygonMode::Fill,
            // Requires Features::DEPTH_CLIP_CONTROL
            unclipped_depth: false,
            // Requires Features::CONSERVATIVE_RASTERIZATION
            conservative: false,
        },
        depth_stencil: depth_format.map(|format| wgpu::DepthStencilState {
            format,
            depth_write_enabled: !transparency,
            // The depth_compare function tells us when to discard a new pixel.
            // Using LESS means pixels will be drawn front to back.
            // This has to be LESS_OR_EQUAL for correct skybox rendering.
            depth_compare: wgpu::CompareFunction::LessEqual,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState {
            count: 1,
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
        // If the pipeline will be used with a multiview resource pass, this
        // indicates how many array layers the attachments will have.
        multiview: None,
    })
}
