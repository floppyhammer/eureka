use crate::render::render_backend::PreparedFrame;
use crate::render::render_graph::{standard_resources, FrameContext, Node, SamplerKey, TextureKey};
use std::any::Any;

pub struct SsrApplyNode {
    pipeline: Option<wgpu::RenderPipeline>,
}

impl Default for SsrApplyNode {
    fn default() -> Self {
        Self { pipeline: None }
    }
}

impl Node for SsrApplyNode {
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
                | wgpu::TextureUsages::TEXTURE_BINDING,
            layers: 1,
            mip_levels: 1,
            dimension: wgpu::TextureDimension::D2,
        });

        NodeResources::new()
            .input(standard_resources::taa_output(), hdr_spec.clone())
            .input(standard_resources::ssr_output(), hdr_spec.clone())
            .output(standard_resources::ssr_combined(), hdr_spec)
    }

    fn run(&mut self, context: &mut FrameContext) {
        if self.pipeline.is_none() {
            let device = &context.render_context.device;

            let bind_group_layout =
                device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("SSR Apply Bind Group Layout"),
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
                    ],
                });

            let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("SSR Apply Pipeline Layout"),
                bind_group_layouts: &[&bind_group_layout],
                push_constant_ranges: &[],
            });

            let shader = wgpu::ShaderModuleDescriptor {
                label: Some("SSR Apply Shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("../../../shaders/ssr_apply.wgsl").into()),
            };

            use crate::render::create_render_pipeline;
            self.pipeline = Some(create_render_pipeline(
                device,
                &pipeline_layout,
                Some(wgpu::TextureFormat::Rgba16Float),
                None,
                &[],
                shader,
                "SSR Apply Pipeline",
                false,
                None,
            ));

            context
                .backend
                .add_bind_group_layout("ssr_apply_bind_group_layout", bind_group_layout);
        }

        let color_texture = context.texture(&standard_resources::taa_output());
        let ssr_texture = context.texture(&standard_resources::ssr_output());
        let output_texture = context.texture(&standard_resources::ssr_combined());

        let sampler = context.get_sampler(SamplerKey {
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let bind_group_layout = context
            .backend
            .get_bind_group_layout("ssr_apply_bind_group_layout")
            .unwrap()
            .clone();

        let bind_group = context.create_bind_group(
            "ssr_apply_bind_group",
            vec![color_texture.id, ssr_texture.id],
            |ctx| {
                ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("SSR Apply Bind Group"),
                    layout: &bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(&color_texture.view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::TextureView(&ssr_texture.view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: wgpu::BindingResource::Sampler(&sampler),
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
                    label: Some("SSR Apply Pass"),
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
}