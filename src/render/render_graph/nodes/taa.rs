use crate::render::render_backend::PreparedFrame;
use crate::render::render_graph::{standard_resources, SamplerKey, TextureKey};
use crate::render::render_graph::{FrameContext, Node};
use std::any::Any;

pub struct TaaNode {
    pipeline: Option<wgpu::RenderPipeline>,
}

impl Default for TaaNode {
    fn default() -> Self {
        Self { pipeline: None }
    }
}

impl Node for TaaNode {
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn node_resources(
        &self,
        _prepared: &PreparedFrame,
    ) -> crate::render::render_graph::resource::NodeResources {
        use crate::render::render_graph::resource::ResourceSpec;

        let hdr_spec = ResourceSpec::Texture(TextureKey {
            width: 0,
            height: 0,
            format: Some(wgpu::TextureFormat::Rgba16Float),
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC, // 新增
            layers: 1,
            mip_levels: 1,
            dimension: wgpu::TextureDimension::D2,
        });

        crate::render::render_graph::resource::NodeResources::new()
            .input(standard_resources::main_color(), hdr_spec.clone())
            .input(standard_resources::main_depth(), ResourceSpec::Texture(TextureKey {
                width: 0,
                height: 0,
                format: Some(wgpu::TextureFormat::Depth32Float),
                usage: wgpu::TextureUsages::TEXTURE_BINDING,
                layers: 1,
                mip_levels: 1,
                dimension: wgpu::TextureDimension::D2,
            }))
            .input(standard_resources::camera_buffer(), ResourceSpec::Buffer(crate::render::render_graph::BufferKey {
                size: (size_of::<crate::render::camera::CameraUniform>() * 16) as u64, // MAX_CAMERAS
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            }))
            .output(standard_resources::taa_output(), hdr_spec)
    }

    fn run(&mut self, context: &mut FrameContext) {
        if self.pipeline.is_none() {
            let device = &context.render_context.device;

            let bind_group_layout =
                device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("TAA Bind Group Layout"),
                    entries: &[
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
                            ty: wgpu::BindingType::Texture {
                                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                                view_dimension: wgpu::TextureViewDimension::D2,
                                multisampled: false,
                            },
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
                            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                            count: None,
                        },
                    ],
                });

            let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("TAA Pipeline Layout"),
                bind_group_layouts: &[&bind_group_layout],
                push_constant_ranges: &[],
            });

            let shader = wgpu::ShaderModuleDescriptor {
                label: Some("TAA Shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("../../../shaders/taa.wgsl").into()),
            };

            use crate::render::create_render_pipeline;
            self.pipeline = Some(create_render_pipeline(
                device,
                &pipeline_layout,
                Some(wgpu::TextureFormat::Rgba16Float),
                None,
                &[],
                shader,
                "TAA Pipeline",
                false,
                None,
            ));

            context
                .backend
                .add_bind_group_layout("taa_bind_group_layout", bind_group_layout);
        }

        let input_texture = context.texture(&standard_resources::main_color());
        let output_texture = context.texture(&standard_resources::taa_output());
        let depth_texture = context.texture(&standard_resources::main_depth());
        let camera_buffer = context.buffer(&standard_resources::camera_buffer());

        // 获取历史纹理
        let history_key = TextureKey {
            width: context.render_context.surface_config.width,
            height: context.render_context.surface_config.height,
            format: Some(wgpu::TextureFormat::Rgba16Float),
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_DST, // 新增
            layers: 1,
            mip_levels: 1,
            dimension: wgpu::TextureDimension::D2,
        };

        let history_texture = context.pool.acquire_history_texture(
            &context.render_context.device,
            "taa_history",
            history_key,
            1, // prev version
            context.frame_count(),
            context.render_context.frames_in_flight as u64,
        );

        let current_history_texture = context.pool.acquire_history_texture(
            &context.render_context.device,
            "taa_history",
            history_key,
            0, // current version (to write into)
            context.frame_count(),
            context.render_context.frames_in_flight as u64,
        );

        let sampler = context.get_sampler(SamplerKey {
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let bind_group_layout = context
            .backend
            .get_bind_group_layout("taa_bind_group_layout")
            .unwrap()
            .clone();

        let bind_group = context.create_bind_group(
            "taa_bind_group",
            vec![
                camera_buffer.id,
                input_texture.id,
                history_texture.id,
                depth_texture.id,
            ],
            |ctx| {
                ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("TAA Bind Group"),
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
                            resource: wgpu::BindingResource::TextureView(&input_texture.view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: wgpu::BindingResource::TextureView(&history_texture.view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 3,
                            resource: wgpu::BindingResource::TextureView(&depth_texture.view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 4,
                            resource: wgpu::BindingResource::Sampler(&sampler),
                        },
                    ],
                })
            },
        );

        let pipeline = self.pipeline.as_ref().unwrap();

        // 渲染到 output_texture
        {
            let mut render_pass = context
                .encoder
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("TAA Pass"),
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
            render_pass.set_bind_group(0, &bind_group, &[0]); // Assuming 1st camera
            render_pass.draw(0..3, 0..1);
        }

        // 将结果拷贝到当前历史纹理
        context.encoder.copy_texture_to_texture(
            output_texture.texture.as_image_copy(),
            current_history_texture.texture.as_image_copy(),
            wgpu::Extent3d {
                width: context.render_context.surface_config.width,
                height: context.render_context.surface_config.height,
                depth_or_array_layers: 1,
            },
        );
    }
}
