use crate::render::render_backend::PreparedFrame;
use crate::render::render_graph::{standard_resources, SamplerKey};
use crate::render::render_graph::{FrameContext, Node};
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

    fn node_resources(
        &self,
        _prepared: &PreparedFrame,
    ) -> crate::render::render_graph::resource::NodeResources {
        use crate::render::render_graph::resource::{ResourceSpec, TextureKey};

        let color_spec = ResourceSpec::Texture(TextureKey {
            width: 0,
            height: 0,
            format: None,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            layers: 1,
        });

        crate::render::render_graph::resource::NodeResources::new()
            .input(standard_resources::hdr_resolved(), color_spec.clone())
            .output(standard_resources::final_output(), color_spec)
    }

    fn run(&mut self, context: &mut FrameContext) {
        if self.pipeline.is_none() {
            let device = &context.render_context.device;

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
                        // 新增：Uniform 开关
                        wgpu::BindGroupLayoutEntry {
                            binding: 2,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
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
                .backend
                .add_bind_group_layout("fxaa_bind_group_layout", bind_group_layout);
        }

        let input_texture = context.texture(&standard_resources::hdr_resolved());

        let fxaa_enabled = context.extracted.fxaa_enabled;
        let pipeline = self.pipeline.as_ref().unwrap();

        let sampler = context.get_sampler(SamplerKey {
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        // 使用 context 的方法获取池化 Buffer，并写入开关状态
        let settings_data = if fxaa_enabled { 1u32 } else { 0u32 };
        let settings_buffer = context.get_buffer(
            "fxaa_settings",
            crate::render::render_graph::BufferKey {
                size: 4,
                usage: wgpu::BufferUsages::UNIFORM,
            },
        );
        context.write_buffer(&settings_buffer.buffer, &[settings_data]);

        let bind_group_layout = context
            .backend
            .get_bind_group_layout("fxaa_bind_group_layout")
            .unwrap()
            .clone();

        // 使用 context.create_bind_group 以利用缓存机制
        let bind_group = context.create_bind_group(
            "fxaa_bind_group_layout",
            vec![input_texture.id, settings_buffer.id],
            |ctx| {
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
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: settings_buffer.buffer.as_entire_binding(),
                        },
                    ],
                })
            },
        );

        let mut render_pass = context
            .encoder
            .begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("FXAA Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &context.final_output_view,
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
