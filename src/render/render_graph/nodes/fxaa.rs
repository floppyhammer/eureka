use crate::render::render_graph::{FrameContext, Node, TextureKey};
use std::any::Any;

pub struct FxaaNode {
    pipeline: Option<wgpu::RenderPipeline>,
}

impl Default for FxaaNode {
    fn default() -> Self {
        Self {
            pipeline: None,
        }
    }
}

impl Node for FxaaNode {
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn prepare(&mut self, context: &mut FrameContext) {
        let device = &context.render_context.device;

        if self.pipeline.is_none() {
            let bind_group_layout =
                device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("fxaa bind group layout"),
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

            let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("fxaa pipeline layout"),
                bind_group_layouts: &[&bind_group_layout],
                push_constant_ranges: &[],
            });

            let shader = wgpu::ShaderModuleDescriptor {
                label: Some("fxaa shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("../../../shaders/fxaa.wgsl").into()),
            };

            use crate::render::create_render_pipeline;
            self.pipeline = Some(create_render_pipeline(
                device,
                &pipeline_layout,
                Some(context.render_context.surface_config.format),
                None,
                &[],
                shader,
                "fxaa pipeline",
                false,
                None,
            ));
        }
    }

    fn run(&mut self, context: &mut FrameContext) {
        let key = TextureKey {
            width: context.render_context.surface_config.width,
            height: context.render_context.surface_config.height,
            format: context.render_context.surface_config.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        };
        let input_texture = context.get_texture("main_color", key);
        let output_texture = context.get_texture("fxaa_color", key);

        let bind_group = context.render_context.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("fxaa bind group"),
            layout: &self.pipeline.as_ref().unwrap().get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&input_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&input_texture.sampler),
                },
            ],
        });

        let mut render_pass = context.encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("fxaa pass"),
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
        });

        render_pass.set_pipeline(self.pipeline.as_ref().unwrap());
        render_pass.set_bind_group(0, &bind_group, &[]);
        render_pass.draw(0..3, 0..1);
    }
}
