use crate::render::render_graph::{Node, RenderContext};
use crate::render::camera::CameraType;
use crate::render::light::render_shadow;
use crate::render::atlas::render_atlas;
use crate::render::sprite::render_sprite;
use crate::render::sky::render_sky;
use crate::render::{render_meshes, prepare_meshes};
use crate::render::vertex::VertexBuffer;

pub struct CullingNode {
    pipeline: Option<wgpu::ComputePipeline>,
}

impl Default for CullingNode {
    fn default() -> Self {
        Self { pipeline: None }
    }
}

impl Node for CullingNode {
    fn prepare(&mut self, context: &mut RenderContext) {
        if self.pipeline.is_some() {
            return;
        }

        let world = context.render_world;
        let resources = &world.mesh_render_resources;
        let device = &context.render_server.device;

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("cull layout"),
            bind_group_layouts: &[&resources.cull_bind_group_layout],
            push_constant_ranges: &[],
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("cull shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../../../shaders/cull.wgsl").into()),
        });

        self.pipeline = Some(device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("cull pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("main"),
            cache: None,
            compilation_options: Default::default(),
        }));
    }

    fn run(&mut self, context: &mut RenderContext) {
        let world = context.render_world;
        let resources = &world.mesh_render_resources;

        if let Some(pipeline) = &self.pipeline {
            if let Some(bind_group) = &resources.cull_bind_group {
                let mut compute_pass =
                    context.encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                        label: Some("Global Culling Pass"),
                        timestamp_writes: None,
                    });
                compute_pass.set_pipeline(pipeline);
                compute_pass.set_bind_group(0, bind_group, &[0]); // Camera offset

                // Dispatch based on total instance count across all meshes
                // We'll calculate total instances in prepare_instances or just use a large enough number
                let total_instances: u32 = world.extracted.meshes.len() as u32;
                if total_instances > 0 {
                    compute_pass.dispatch_workgroups((total_instances + 63) / 64, 1, 1);
                }
            }
        }
    }
}

pub struct ShadowNode {
    pipeline: Option<wgpu::RenderPipeline>,
}

impl Default for ShadowNode {
    fn default() -> Self {
        Self { pipeline: None }
    }
}

impl Node for ShadowNode {
    fn prepare(&mut self, context: &mut RenderContext) {
        if self.pipeline.is_some() {
            return;
        }

        let device = &context.render_server.device;
        let camera_resources = &context.render_world.camera_render_resources;

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("shadow pipeline layout"),
            bind_group_layouts: &[&camera_resources.bind_group_layout],
            push_constant_ranges: &[],
        });

        let shader = wgpu::ShaderModuleDescriptor {
            label: Some("shadow shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../../../shaders/shadow.wgsl").into()),
        };

        use crate::render::vertex::Vertex3d;
        use crate::render::{create_render_pipeline, InstanceRaw, Texture};

        let pipeline = create_render_pipeline(
            device,
            &pipeline_layout,
            None,
            Some(Texture::DEPTH_FORMAT),
            &[Vertex3d::desc(), InstanceRaw::desc()],
            shader,
            "shadow pipeline",
            false,
            Some(wgpu::Face::Front),
        );

        self.pipeline = Some(pipeline);
    }

    fn run(&mut self, context: &mut RenderContext) {
        let world = context.render_world;
        if let Some(pipeline) = &self.pipeline {
            render_shadow(
                context.encoder,
                &world.texture_cache,
                &world.light_render_resources,
                &world.extracted.lights,
                &world.extracted.meshes,
                &world.mesh_cache,
                &world.mesh_render_resources,
                &world.extracted.bvh,
                pipeline,
            );
        }
    }
}

pub struct SsaoNode {
    normal_pipeline: Option<wgpu::RenderPipeline>,
    ssao_pipeline: Option<wgpu::RenderPipeline>,
    blur_pipeline: Option<wgpu::RenderPipeline>,
}

impl Default for SsaoNode {
    fn default() -> Self {
        Self {
            normal_pipeline: None,
            ssao_pipeline: None,
            blur_pipeline: None,
        }
    }
}

impl Node for SsaoNode {
    fn prepare(&mut self, context: &mut RenderContext) {
        if self.normal_pipeline.is_some() {
            return;
        }

        let device = &context.render_server.device;
        let world = context.render_world;
        let camera_resources = &world.camera_render_resources;

        use crate::render::vertex::Vertex3d;
        use crate::render::{create_render_pipeline, InstanceRaw, Texture};

        // 1. Normal Pipeline
        let normal_pipeline = {
            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("SSAO Normal Pipeline Layout"),
                bind_group_layouts: &[&camera_resources.bind_group_layout],
                push_constant_ranges: &[],
            });
            let shader = wgpu::ShaderModuleDescriptor {
                label: Some("SSAO Normal Shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("../../../shaders/normal.wgsl").into()),
            };
            create_render_pipeline(
                device,
                &layout,
                Some(wgpu::TextureFormat::Rgba16Float),
                Some(Texture::DEPTH_FORMAT),
                &[Vertex3d::desc(), InstanceRaw::desc()],
                shader,
                "SSAO Normal Pipeline",
                false,
                Some(wgpu::Face::Back),
            )
        };

        // 2. SSAO Pipeline
        let ssao_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("SSAO Bind Group Layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry { binding: 0, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None }, count: None },
                    wgpu::BindGroupLayoutEntry { binding: 1, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: true }, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false }, count: None },
                    wgpu::BindGroupLayoutEntry { binding: 2, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering), count: None },
                    wgpu::BindGroupLayoutEntry { binding: 3, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Depth, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false }, count: None },
                    wgpu::BindGroupLayoutEntry { binding: 4, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: false }, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false }, count: None },
                    wgpu::BindGroupLayoutEntry { binding: 5, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering), count: None },
                ],
            });

        let ssao_pipeline = {
            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("SSAO Pipeline Layout"),
                bind_group_layouts: &[&camera_resources.bind_group_layout, &ssao_bind_group_layout],
                push_constant_ranges: &[],
            });
            let shader = wgpu::ShaderModuleDescriptor {
                label: Some("SSAO Shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("../../../shaders/ssao.wgsl").into()),
            };
            create_render_pipeline(
                device,
                &layout,
                Some(wgpu::TextureFormat::R8Unorm),
                None,
                &[],
                shader,
                "SSAO Pipeline",
                false,
                None,
            )
        };

        // 3. Blur Pipeline
        let blur_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("SSAO Blur Bind Group Layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });

        let blur_pipeline = {
            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("SSAO Blur Pipeline Layout"),
                bind_group_layouts: &[&blur_bind_group_layout],
                push_constant_ranges: &[],
            });
            let shader = wgpu::ShaderModuleDescriptor {
                label: Some("SSAO Blur Shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("../../../shaders/ssao_blur.wgsl").into()),
            };
            create_render_pipeline(
                device,
                &layout,
                Some(wgpu::TextureFormat::R8Unorm),
                None,
                &[],
                shader,
                "SSAO Blur Pipeline",
                false,
                None,
            )
        };

        self.normal_pipeline = Some(normal_pipeline);
        self.ssao_pipeline = Some(ssao_pipeline);
        self.blur_pipeline = Some(blur_pipeline);
    }

    fn run(&mut self, context: &mut RenderContext) {
        let world = context.render_world;
        if world.extracted.meshes.is_empty() {
            return;
        }

        // Check if any camera wants SSAO.
        let mut camera_wants_ssao = false;
        let mut ssao_camera_index = 0;
        for i in 0..world.extracted.cameras.types.len() {
            if world.extracted.cameras.types[i] == CameraType::D3 {
                if world.extracted.cameras.uniforms[i].ssao_enabled == 1 {
                    camera_wants_ssao = true;
                    ssao_camera_index = i;
                    break;
                }
            }
        }

        if !camera_wants_ssao {
            return;
        }

        let camera_bind_group = world.camera_render_resources.bind_group.as_ref().unwrap();

        let normal_view = &world
            .texture_cache
            .get(world.ssao_render_resources.normal_texture)
            .unwrap()
            .view;
        let depth_view = &world
            .texture_cache
            .get(world.surface_depth_texture)
            .unwrap()
            .view;

        // 1. Normal Pass
        {
            let mut render_pass = context.encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("SSAO Normal Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: normal_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            world.ssao_render_resources.render_normal(
                &mut render_pass,
                &world.extracted.meshes,
                &world.mesh_cache,
                &world.mesh_render_resources,
                camera_bind_group,
                ssao_camera_index,
                &world.extracted.cameras.uniforms[ssao_camera_index],
                &world.extracted.bvh,
                self.normal_pipeline.as_ref().unwrap(),
            );
        }

        // 2. SSAO Pass
        {
            let ssao_view = &world
                .texture_cache
                .get(world.ssao_render_resources.ssao_texture)
                .unwrap()
                .view;
            let mut render_pass = context.encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("SSAO Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: ssao_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            render_pass.set_pipeline(self.ssao_pipeline.as_ref().unwrap());
            render_pass.set_bind_group(0, camera_bind_group, &[0]);
            render_pass.set_bind_group(1, &world.ssao_render_resources.ssao_bind_group, &[]);
            render_pass.draw(0..3, 0..1);
        }

        // 3. Blur Pass
        {
            let blur_view = &world
                .texture_cache
                .get(world.ssao_render_resources.blur_texture)
                .unwrap()
                .view;
            let mut render_pass = context.encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("SSAO Blur Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: blur_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            render_pass.set_pipeline(self.blur_pipeline.as_ref().unwrap());
            render_pass.set_bind_group(0, &world.ssao_render_resources.blur_bind_group, &[]);
            render_pass.draw(0..3, 0..1);
        }
    }
}

pub struct MainPassNode;

impl Node for MainPassNode {
    fn prepare(&mut self, context: &mut RenderContext) {
        let world = context.render_world;
        let skybox_texture_id = world.extracted.sky.as_ref().map(|sky| sky.texture);

        // This is a bit ugly because we are mutably borrowing world inside RenderContext
        // but for now we'll assume prepare_meshes can be called here.
        // Actually, RenderContext has &RenderWorld (immutable).
        // We need to fix RenderContext or the way we call prepare.
    }

    fn run(&mut self, context: &mut RenderContext) {
        let world = context.render_world;
        let depth_texture = world
            .texture_cache
            .get(world.surface_depth_texture)
            .unwrap();

        let ssao_ran = {
            let mut wants_ssao = false;
            for (i, cam_type) in world.extracted.cameras.types.iter().enumerate() {
                if *cam_type == CameraType::D3 {
                    if world.extracted.cameras.uniforms[i].ssao_enabled == 1 {
                        wants_ssao = true;
                        break;
                    }
                }
            }
            wants_ssao && !world.extracted.meshes.is_empty()
        };

        let mut render_pass = context.encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("main render pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: context.output_view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.1,
                        g: 0.2,
                        b: 0.3,
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &depth_texture.view,
                depth_ops: Some(wgpu::Operations {
                    load: if ssao_ran {
                        wgpu::LoadOp::Load
                    } else {
                        wgpu::LoadOp::Clear(1.0)
                    },
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        for i in 0..world.extracted.cameras.uniforms.len() {
            if world.extracted.cameras.types[i] == CameraType::D2 {
                render_atlas(
                    &world.extracted.atlases,
                    &world.atlas_render_resources,
                    &mut render_pass,
                );

                // Draw sprites.
                render_sprite(
                    &world.sprite_batches,
                    &world.sprite_render_resources,
                    &mut render_pass,
                    world.camera_render_resources.bind_group.as_ref().unwrap(),
                );
            } else {
                if world.camera_render_resources.bind_group.is_some() {
                    render_sky(
                        world.camera_render_resources.bind_group.as_ref().unwrap(),
                        &world.sky_render_resources,
                        &mut render_pass,
                        &world.mesh_render_resources.mesh_allocator,
                    );
                }

                render_meshes(
                    &world.extracted.meshes,
                    &world.mesh_cache,
                    &world.mesh_render_resources,
                    &world.camera_render_resources,
                    i,
                    &world.extracted.cameras.uniforms[i],
                    &world.gizmo_render_resources,
                    &mut render_pass,
                    &world.extracted.bvh,
                );
            }
        }
    }
}
