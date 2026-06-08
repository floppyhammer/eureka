use crate::render::camera::CameraType;
use crate::render::render_graph::{FrameContext, Node, TextureKey};
use crate::render::vertex::{Vertex3d, VertexBuffer};
use crate::render::{create_render_pipeline, InstanceRaw, Texture};
use std::any::Any;

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
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn prepare(&mut self, context: &mut FrameContext) {
        if self.normal_pipeline.is_some() {
            return;
        }

        let device = &context.render_context.device;
        let world = &*context.render_world;
        let camera_resources = &world.camera_render_resources;

        let normal_pipeline = {
            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("SSAO Normal Pipeline Layout"),
                bind_group_layouts: &[&camera_resources.bind_group_layout],
                push_constant_ranges: &[],
            });
            let shader = wgpu::ShaderModuleDescriptor {
                label: Some("SSAO Normal Shader"),
                source: wgpu::ShaderSource::Wgsl(
                    include_str!("../../../shaders/normal.wgsl").into(),
                ),
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

        let ssao_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("SSAO Bind Group Layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
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
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Depth,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 4,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: false },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 5,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
                        count: None,
                    },
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
                source: wgpu::ShaderSource::Wgsl(
                    include_str!("../../../shaders/ssao_blur.wgsl").into(),
                ),
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

    fn run(&mut self, context: &mut FrameContext) {
        let width = context.render_context.surface_config.width;
        let height = context.render_context.surface_config.height;

        let main_depth_key = TextureKey {
            width,
            height,
            format: Texture::DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        };

        let main_depth = context.get_texture("main_depth", main_depth_key);
        let depth_view = main_depth.view; // 现在是拥有所有权的句柄

        // 重新获取并标记为可变借用
        let world = &mut *context.render_world;

        if world.extracted.meshes.is_empty() || !world.extracted.ssao_enabled {
            let blur_view = &world
                .texture_cache
                .get(world.ssao_render_resources.blur_texture)
                .unwrap()
                .view;
            context.encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("SSAO Clear Pass"),
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
            return;
        }

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

        // 重要：更新 SSAO 的 BindGroup 以使用当前的瞬时深度图
        world.ssao_render_resources.update_bind_groups(
            &context.render_context.device,
            &world.texture_cache,
            &depth_view,
        );

        let camera_bind_group = world.camera_render_resources.bind_group.as_ref().unwrap();

        let normal_view = &world
            .texture_cache
            .get(world.ssao_render_resources.normal_texture)
            .unwrap()
            .view;

        {
            let mut render_pass = context
                .encoder
                .begin_render_pass(&wgpu::RenderPassDescriptor {
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
                        view: &depth_view,
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

        {
            let ssao_view = &world
                .texture_cache
                .get(world.ssao_render_resources.ssao_texture)
                .unwrap()
                .view;
            let mut render_pass = context
                .encoder
                .begin_render_pass(&wgpu::RenderPassDescriptor {
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

        {
            let blur_view = &world
                .texture_cache
                .get(world.ssao_render_resources.blur_texture)
                .unwrap()
                .view;
            let mut render_pass = context
                .encoder
                .begin_render_pass(&wgpu::RenderPassDescriptor {
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
