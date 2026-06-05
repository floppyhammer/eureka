use crate::render::render_graph::{FrameContext, Node};
use crate::render::TextureId;

pub struct FxaaNode {
    pipeline: Option<wgpu::RenderPipeline>,
    bind_group: Option<wgpu::BindGroup>,
    current_texture_id: Option<TextureId>,
}

impl Default for FxaaNode {
    fn default() -> Self {
        Self {
            pipeline: None,
            bind_group: None,
            current_texture_id: None,
        }
    }
}

impl Node for FxaaNode {
    fn prepare(&mut self, context: &mut FrameContext) {
        let world = context.render_world;
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

        if self.current_texture_id != Some(world.main_color_texture) {
            let texture = world.texture_cache.get(world.main_color_texture).unwrap();
            let bind_group_layout = self.pipeline.as_ref().unwrap().get_bind_group_layout(0);
            self.bind_group = Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("fxaa bind group"),
                layout: &bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&texture.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&texture.sampler),
                    },
                ],
            }));
            self.current_texture_id = Some(world.main_color_texture);
        }
    }

    fn run(&mut self, context: &mut FrameContext) {
        if let (Some(pipeline), Some(bind_group)) = (&self.pipeline, &self.bind_group) {
            let mut render_pass = context
                .encoder
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("fxaa pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: context.final_output_view,
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
            render_pass.set_bind_group(0, bind_group, &[]);
            render_pass.draw(0..3, 0..1);
        }
    }
}
