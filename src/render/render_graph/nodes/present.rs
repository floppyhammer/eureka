use crate::render::camera::CameraUniform;
use crate::render::create_render_pipeline;
use crate::render::render_graph::{standard_resources, SamplerKey, TextureId};
use crate::render::render_graph::{FrameContext, Node, TextureKey};
use std::any::Any;

pub struct PresentNode {
    pipeline: Option<wgpu::RenderPipeline>,
    pub input_resource_id: TextureId,
}

impl Default for PresentNode {
    fn default() -> Self {
        Self {
            pipeline: None,
            input_resource_id: standard_resources::main_color(),
        }
    }
}

impl PresentNode {
    pub fn with_input(mut self, input_id: TextureId) -> Self {
        self.input_resource_id = input_id;
        self
    }
}

impl Node for PresentNode {
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn node_resources(&self) -> crate::render::render_graph::resource::NodeResources {
        use crate::render::render_graph::resource::{ResourceSpec, TextureKey};

        crate::render::render_graph::resource::NodeResources::new()
            .input(
                self.input_resource_id.clone(),
                ResourceSpec::Texture(TextureKey {
                    width: 0,
                    height: 0,
                    format: wgpu::TextureFormat::Bgra8UnormSrgb, // 将被自动修正
                    usage: wgpu::TextureUsages::TEXTURE_BINDING,
                    layers: 1,
                }),
            )
            .output(
                standard_resources::final_output(),
                ResourceSpec::Texture(TextureKey {
                    width: 0,
                    height: 0,
                    format: wgpu::TextureFormat::Bgra8UnormSrgb,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                    layers: 1,
                }),
            )
    }

    fn prepare(&mut self, context: &mut FrameContext) {
        if self.pipeline.is_some() {
            return;
        }

        let device = &context.render_context.device;

        let bind_group_layout = context
            .pool
            .get_bind_group_layout("present_bind_group_layout");

        if bind_group_layout.is_none() {
            let bind_group_layout =
                device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Present Bind Group Layout"),
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

            context
                .pool
                .add_bind_group_layout("present_bind_group_layout", bind_group_layout);
        }

        let bind_group_layout = context
            .pool
            .get_bind_group_layout("present_bind_group_layout")
            .unwrap();

        if self.pipeline.is_none() {
            let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Present Pipeline Layout"),
                bind_group_layouts: &[&bind_group_layout],
                push_constant_ranges: &[],
            });

            let shader = wgpu::ShaderModuleDescriptor {
                label: Some("Present Shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("../../../shaders/blit.wgsl").into()),
            };

            self.pipeline = Some(create_render_pipeline(
                device,
                &pipeline_layout,
                Some(context.render_context.surface_config.format),
                None,
                &[],
                shader,
                "Present Pipeline",
                false,
                None,
            ));
        }
    }

    fn run(&mut self, context: &mut FrameContext) {
        let input_key = TextureKey {
            width: context.render_context.surface_config.width,
            height: context.render_context.surface_config.height,
            format: context.render_context.surface_config.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            layers: 1,
        };
        let input_texture = context.get_texture_by_id(&self.input_resource_id, input_key);

        let sampler = context
            .get_sampler(SamplerKey {
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                ..Default::default()
            })
            .clone();

        let bind_group_layout = context
            .pool
            .get_bind_group_layout("present_bind_group_layout")
            .unwrap()
            .clone();

        let bind_group =
            context.create_bind_group(&bind_group_layout, vec![input_texture.id], |ctx| {
                ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
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
                    label: Some("Present Bind Group"),
                })
            });

        let mut render_pass = context
            .encoder
            .begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Present Pass"),
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

        render_pass.set_pipeline(self.pipeline.as_ref().unwrap());
        render_pass.set_bind_group(0, &bind_group, &[]);
        render_pass.draw(0..3, 0..1);
    }
}
