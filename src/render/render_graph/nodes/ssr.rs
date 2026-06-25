use crate::render::render_backend::PreparedFrame;
use crate::render::render_graph::{standard_resources, FrameContext, Node, SamplerKey, TextureKey};
use crate::render::Texture;
use std::any::Any;

pub struct SsrNode {
    pipeline: Option<wgpu::RenderPipeline>,
}

impl Default for SsrNode {
    fn default() -> Self {
        Self { pipeline: None }
    }
}

impl Node for SsrNode {
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn node_resources(
        &self,
        _prepared: &PreparedFrame,
    ) -> crate::render::render_graph::resource::NodeResources {
        use crate::render::render_graph::resource::{NodeResources, ResourceSpec};

        let hdr_spec = ResourceSpec::Texture(TextureKey {
            width: 0,
            height: 0,
            format: Some(wgpu::TextureFormat::Rgba16Float),
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
            layers: 1,
            mip_levels: 1,
            dimension: wgpu::TextureDimension::D2,
        });

        let depth_spec = ResourceSpec::Texture(TextureKey {
            width: 0,
            height: 0,
            format: Some(Texture::DEPTH_FORMAT),
            usage: wgpu::TextureUsages::TEXTURE_BINDING,
            layers: 1,
            mip_levels: 1,
            dimension: wgpu::TextureDimension::D2,
        });

        let normal_spec = ResourceSpec::Texture(TextureKey {
            width: 0,
            height: 0,
            format: Some(wgpu::TextureFormat::Rgba16Float),
            usage: wgpu::TextureUsages::TEXTURE_BINDING,
            layers: 1,
            mip_levels: 1,
            dimension: wgpu::TextureDimension::D2,
        });

        NodeResources::new()
            // 输入：TAA 输出的颜色、PrePass 的 normal/roughness、主深度
            .input(standard_resources::taa_output(), hdr_spec.clone())
            .input(standard_resources::prepass_normal(), normal_spec)
            .input(standard_resources::main_depth(), depth_spec)
            .input(
                standard_resources::camera_buffer(),
                ResourceSpec::buffer(
                    size_of::<crate::render::camera::CameraUniform>() as u64 * 16,
                    wgpu::BufferUsages::UNIFORM,
                ),
            )
            // 输出：SSR 结果
            .output(standard_resources::ssr_output(), hdr_spec)
    }

    fn run(&mut self, context: &mut FrameContext) {
        if self.pipeline.is_none() {
            let device = &context.render_context.device;

            let bind_group_layout =
                device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("SSR Bind Group Layout"),
                    entries: &[
                        // Camera uniform
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: true,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        // Color texture (TAA output)
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
                        // Normal/Roughness texture
                        wgpu::BindGroupLayoutEntry {
                            binding: 2,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Texture {
                                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                                view_dimension: wgpu::TextureViewDimension::D2,
                                multisampled: false,
                            },
                            count: None,
                        },
                        // Depth texture
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
                        // Nearest sampler
                        wgpu::BindGroupLayoutEntry {
                            binding: 4,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
                            count: None,
                        },
                        // Linear sampler (for color sampling)
                        wgpu::BindGroupLayoutEntry {
                            binding: 5,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                            count: None,
                        },
                    ],
                });

            let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("SSR Pipeline Layout"),
                bind_group_layouts: &[Some(&bind_group_layout)],
                immediate_size: 0,
            });

            let source = include_str!("../../../shaders/ssr.wgsl")
                .replace("#import eureka::camera::Camera", crate::render::camera::CAMERA_STRUCT_WGSL);

            let shader = wgpu::ShaderModuleDescriptor {
                label: Some("SSR Shader"),
                source: wgpu::ShaderSource::Wgsl(source.into()),
            };

            use crate::render::create_render_pipeline;
            self.pipeline = Some(create_render_pipeline(
                device,
                &pipeline_layout,
                Some(wgpu::TextureFormat::Rgba16Float),
                None,
                &[],
                shader,
                "SSR Pipeline",
                false,
                None,
            ));

            context
                .backend
                .add_bind_group_layout("ssr_bind_group_layout", bind_group_layout);
        }

        let color_texture = context.texture(&standard_resources::taa_output());
        let normal_texture = context.texture(&standard_resources::prepass_normal());
        let depth_texture = context.texture(&standard_resources::main_depth());
        let output_texture = context.texture(&standard_resources::ssr_output());
        let camera_buffer = context.buffer(&standard_resources::camera_buffer());

        // 如果 SSR 未开启，直接清空并跳过后续绘制
        let ssr_enabled = context.extracted.cameras.uniforms.iter().any(|u| u.ssr_enabled != 0);
        if !ssr_enabled {
            context.encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("SSR Pass (Disabled)"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &output_texture.view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            return;
        }

        let nearest_sampler = context.get_sampler(SamplerKey {
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let linear_sampler = context.get_sampler(SamplerKey {
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let bind_group_layout = context
            .backend
            .get_bind_group_layout("ssr_bind_group_layout")
            .unwrap()
            .clone();

        let bind_group = context.create_bind_group(
            "ssr_bind_group",
            vec![
                camera_buffer.id,
                color_texture.id,
                normal_texture.id,
                depth_texture.id,
            ],
            |ctx| {
                ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("SSR Bind Group"),
                    layout: &bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                                buffer: &camera_buffer.buffer,
                                offset: 0,
                                size: Some(
                                    wgpu::BufferSize::new(
                                        size_of::<crate::render::camera::CameraUniform>() as u64,
                                    )
                                    .unwrap(),
                                ),
                            }),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::TextureView(&color_texture.view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: wgpu::BindingResource::TextureView(&normal_texture.view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 3,
                            resource: wgpu::BindingResource::TextureView(&depth_texture.view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 4,
                            resource: wgpu::BindingResource::Sampler(&nearest_sampler),
                        },
                        wgpu::BindGroupEntry {
                            binding: 5,
                            resource: wgpu::BindingResource::Sampler(&linear_sampler),
                        },
                    ],
                })
            },
        );

        let pipeline = self.pipeline.as_ref().unwrap();

        {
            let mut render_pass = context
                .encoder
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("SSR Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &output_texture.view,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                });

            render_pass.set_pipeline(pipeline);
            render_pass.set_bind_group(0, &bind_group, &[0]);
            render_pass.draw(0..3, 0..1);
        }
    }
}