use crate::render::create_render_pipeline;
use crate::render::render_graph::{standard_resources, SamplerKey};
use crate::render::render_graph::{FrameContext, Node};
use std::any::Any;
use crate::render::render_world::RenderWorld;

pub struct ToneMappingNode {
    pipeline: Option<wgpu::RenderPipeline>,
}

impl Default for ToneMappingNode {
    fn default() -> Self {
        Self { pipeline: None }
    }
}

impl Node for ToneMappingNode {
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn node_resources(&self, world: &RenderWorld) -> crate::render::render_graph::resource::NodeResources {
        use crate::render::render_graph::resource::ResourceSpec;

        crate::render::render_graph::resource::NodeResources::new()
            .input(
                standard_resources::main_color(),
                ResourceSpec::texture(
                    0,
                    0,
                    Some(wgpu::TextureFormat::Rgba16Float),
                    wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
                    1,
                ),
            )
            .output(
                standard_resources::hdr_resolved(), // Draw to SDR texture
                ResourceSpec::texture(
                    0,
                    0,
                    None,
                    wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
                    1,
                ),
            )
    }

    fn run(&mut self, context: &mut FrameContext) {
        if self.pipeline.is_none() {
            let device = &context.render_context.device;

            let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("ToneMapping Bind Group Layout"),
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
                label: Some("ToneMapping Pipeline Layout"),
                bind_group_layouts: &[&bind_group_layout],
                push_constant_ranges: &[],
            });

            let shader = wgpu::ShaderModuleDescriptor {
                label: Some("ToneMapping Shader"),
                source: wgpu::ShaderSource::Wgsl(
                    include_str!("../../../shaders/tonemapping.wgsl").into(),
                ),
            };

            self.pipeline = Some(create_render_pipeline(
                device,
                &pipeline_layout,
                Some(context.render_context.surface_config.format), // 必须匹配 Surface
                None,
                &[],
                shader,
                "ToneMapping Pipeline",
                false,
                None,
            ));

            context
                .pool
                .add_bind_group_layout("tonemapping_bind_group_layout", bind_group_layout);
        }
        
        let input_texture = context.texture(&standard_resources::main_color());
        let output_texture = context.texture(&standard_resources::hdr_resolved());

        let sampler = context.get_sampler(SamplerKey {
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let bind_group_layout = context
            .pool
            .get_bind_group_layout("tonemapping_bind_group_layout")
            .unwrap()
            .clone();
        let bind_group =
            context.create_bind_group(&bind_group_layout, vec![input_texture.id], |ctx| {
                ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("ToneMapping Bind Group"),
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
                label: Some("Tone Mapping Pass"),
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
