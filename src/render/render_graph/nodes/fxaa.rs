use crate::render::render_graph::{standard_resources, SamplerKey};
use crate::render::render_graph::{FrameContext, Node, TextureKey};
use std::any::Any;

pub struct FxaaNode {
    pipeline: Option<wgpu::RenderPipeline>,
}

impl Default for FxaaNode {
    fn default() -> Self {
        Self { pipeline: None }
    }
}

impl Node for FxaaNode {
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn node_resources(&self) -> crate::render::render_graph::resource::NodeResources {
        use crate::render::render_graph::resource::{ResourceSpec, TextureKey};

        let color_spec = ResourceSpec::Texture(TextureKey {
            width: 0,
            height: 0,
            format: wgpu::TextureFormat::Bgra8UnormSrgb, // 暂定，实际会合并
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            layers: 1,
        });

        crate::render::render_graph::resource::NodeResources::new()
            .input(standard_resources::main_color(), color_spec.clone())
            .output(standard_resources::fxaa_color(), color_spec)
    }

    fn prepare(&mut self, context: &mut FrameContext) {

    }

    fn run(&mut self, context: &mut FrameContext) {
        let device = &context.render_context.device;

        if self.pipeline.is_none() {
            let bind_group_layout =
                device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("FXAA Bind Group Layout"),
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
                label: Some("FXAA Pipeline Layout"),
                bind_group_layouts: &[&bind_group_layout],
                push_constant_ranges: &[],
            });

            let shader = wgpu::ShaderModuleDescriptor {
                label: Some("FXAA Shader"),
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
                "FXAA Pipeline",
                false,
                None,
            ));

            context
                .pool
                .add_bind_group_layout("fxaa_bind_group_layout", bind_group_layout);
        }
        
        let key = TextureKey {
            width: context.render_context.surface_config.width,
            height: context.render_context.surface_config.height,
            format: context.render_context.surface_config.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            layers: 1,
        };
        let input_texture = context.get_texture_by_id(&standard_resources::main_color(), key);
        let output_texture = context.get_texture_by_id(&standard_resources::fxaa_color(), key);

        let pipeline = self.pipeline.as_ref().unwrap();

        let sampler = context.get_sampler(SamplerKey {
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let bind_group_layout = context
            .pool
            .get_bind_group_layout("fxaa_bind_group_layout")
            .unwrap()
            .clone();

        let bind_group =
            context.create_bind_group(&bind_group_layout, vec![input_texture.id], |ctx| {
                ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("FXAA Bind Group"),
                    layout: &bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(&input_texture.view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::Sampler(&sampler),
                        },
                    ],
                })
            });

        let mut render_pass = context
            .encoder
            .begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("FXAA Pass"),
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

        render_pass.set_pipeline(pipeline);
        render_pass.set_bind_group(0, &bind_group, &[]);
        render_pass.draw(0..3, 0..1);
    }
}
