use crate::render::render_backend::PreparedFrame;
use crate::render::render_graph::{standard_resources, SamplerKey};
use crate::render::render_graph::{FrameContext, Node};
use std::any::Any;

pub struct BloomNode {
    downsample_pipeline: Option<wgpu::RenderPipeline>,
    upsample_pipeline: Option<wgpu::RenderPipeline>,
}

impl Default for BloomNode {
    fn default() -> Self {
        Self {
            downsample_pipeline: None,
            upsample_pipeline: None,
        }
    }
}

impl Node for BloomNode {
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
            format: Some(wgpu::TextureFormat::Rgba16Float),
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
            layers: 1,
            mip_levels: 1,
            dimension: wgpu::TextureDimension::D2,
        });

        crate::render::render_graph::resource::NodeResources::new()
            .input(standard_resources::main_color(), color_spec.clone())
            .output(standard_resources::bloom_texture(), color_spec)
    }

    fn run(&mut self, context: &mut FrameContext) {
        if self.downsample_pipeline.is_none() {
            let device = &context.render_context.device;

            let bind_group_layout =
                device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Bloom Bind Group Layout"),
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
                    ],
                });

            let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Bloom Pipeline Layout"),
                bind_group_layouts: &[&bind_group_layout],
                push_constant_ranges: &[],
            });

            let shader = wgpu::ShaderModuleDescriptor {
                label: Some("Bloom Shader"),
                source: wgpu::ShaderSource::Wgsl(
                    include_str!("../../../shaders/bloom.wgsl").into(),
                ),
            };

            use crate::render::create_render_pipeline_with_entry;
            self.downsample_pipeline = Some(create_render_pipeline_with_entry(
                device,
                &pipeline_layout,
                Some(wgpu::TextureFormat::Rgba16Float),
                None,
                &[],
                shader.clone(),
                "Bloom Downsample Pipeline",
                false,
                None,
                "vs_main",
                "fs_downsample",
            ));

            self.upsample_pipeline = Some(create_render_pipeline_with_entry(
                device,
                &pipeline_layout,
                Some(wgpu::TextureFormat::Rgba16Float),
                None,
                &[],
                shader,
                "Bloom Upsample Pipeline",
                false,
                None,
                "vs_main",
                "fs_upsample",
            ));

            context
                .backend
                .add_bind_group_layout("bloom_bind_group_layout", bind_group_layout);
        }

        let main_color = context.texture(&standard_resources::main_color());
        let bloom_output = context.texture(&standard_resources::bloom_texture());

        let sampler = context.get_sampler(SamplerKey {
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let bind_group_layout = context
            .backend
            .get_bind_group_layout("bloom_bind_group_layout")
            .unwrap()
            .clone();

        let levels = 5;
        let mut downsampled_textures = Vec::new();

        let mut current_width = context.render_context.surface_config.width;
        let mut current_height = context.render_context.surface_config.height;

        // 1. Downsample chain
        let mut prev_view = main_color.view.clone();
        let mut prev_id = main_color.id;

        for i in 0..levels {
            current_width /= 2;
            current_height /= 2;
            if current_width == 0 || current_height == 0 {
                break;
            }

            let key = crate::render::render_graph::resource::TextureKey {
                width: current_width,
                height: current_height,
                format: Some(wgpu::TextureFormat::Rgba16Float),
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                layers: 1,
                mip_levels: 1,
                dimension: wgpu::TextureDimension::D2,
            };

            let name = format!("bloom_down_{}", i);
            let tex = context.get_texture(name, key);

            // Bind group for downsample
            let bg = context.create_bind_group(
                "bloom_bind_group_layout",
                vec![prev_id, prev_id], // Use prev_id twice to satisfy layout
                |ctx| {
                    ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
                        label: Some("Bloom Downsample BG"),
                        layout: &bind_group_layout,
                        entries: &[
                            wgpu::BindGroupEntry {
                                binding: 0,
                                resource: wgpu::BindingResource::TextureView(&prev_view),
                            },
                            wgpu::BindGroupEntry {
                                binding: 1,
                                resource: wgpu::BindingResource::Sampler(&sampler),
                            },
                            wgpu::BindGroupEntry {
                                binding: 2,
                                resource: wgpu::BindingResource::TextureView(&prev_view),
                            },
                        ],
                    })
                },
            );

            {
                let mut rpass = context
                    .encoder
                    .begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("Bloom Downsample Pass"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &tex.view,
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
                rpass.set_pipeline(self.downsample_pipeline.as_ref().unwrap());
                rpass.set_bind_group(0, &bg, &[]);
                rpass.draw(0..3, 0..1);
            }

            prev_view = tex.view.clone();
            prev_id = tex.id;
            downsampled_textures.push(tex);
        }

        // 2. Upsample chain
        for i in (0..downsampled_textures.len() - 1).rev() {
            let target_tex = &downsampled_textures[i];

            // Create a temporary texture for the upsampled result
            let key = crate::render::render_graph::resource::TextureKey {
                width: target_tex.texture.width(),
                height: target_tex.texture.height(),
                format: Some(target_tex.texture.format()),
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                layers: 1,
                mip_levels: 1,
                dimension: wgpu::TextureDimension::D2,
            };
            let upsample_rt = context.get_texture(format!("bloom_up_{}", i), key);

            let bg = context.create_bind_group(
                "bloom_bind_group_layout",
                vec![prev_id, target_tex.id],
                |ctx| {
                    ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
                        label: Some("Bloom Upsample BG"),
                        layout: &bind_group_layout,
                        entries: &[
                            wgpu::BindGroupEntry {
                                binding: 0,
                                resource: wgpu::BindingResource::TextureView(&prev_view),
                            },
                            wgpu::BindGroupEntry {
                                binding: 1,
                                resource: wgpu::BindingResource::Sampler(&sampler),
                            },
                            wgpu::BindGroupEntry {
                                binding: 2,
                                resource: wgpu::BindingResource::TextureView(&target_tex.view),
                            },
                        ],
                    })
                },
            );

            {
                let mut rpass = context
                    .encoder
                    .begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("Bloom Upsample Pass"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &upsample_rt.view,
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
                rpass.set_pipeline(self.upsample_pipeline.as_ref().unwrap());
                rpass.set_bind_group(0, &bg, &[]);
                rpass.draw(0..3, 0..1);
            }
            prev_view = upsample_rt.view.clone();
            prev_id = upsample_rt.id;
        }

        // 3. Final copy to bloom_output
        // The final prev_view is the result of the top-most upsample (1/2 size)
        let bg =
            context.create_bind_group("bloom_bind_group_layout", vec![prev_id, prev_id], |ctx| {
                ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("Bloom Final BG"),
                    layout: &bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(&prev_view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::Sampler(&sampler),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: wgpu::BindingResource::TextureView(&prev_view),
                        },
                    ],
                })
            });

        let mut rpass = context
            .encoder
            .begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Bloom Final Copy Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &bloom_output.view,
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
        rpass.set_pipeline(self.downsample_pipeline.as_ref().unwrap());
        rpass.set_bind_group(0, &bg, &[]);
        rpass.draw(0..3, 0..1);
    }
}
