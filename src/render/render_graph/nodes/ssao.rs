use crate::render::camera::{CameraType, CameraUniform};
use crate::render::render_backend::PreparedFrame;
use crate::render::render_graph::{
    standard_resources, BufferKey, FrameContext, Node, SamplerKey, TextureKey,
};
use crate::render::vertex::{Vertex3d, VertexBuffer};
use crate::render::{create_render_pipeline, InstanceRaw, Texture};
use glam::vec3;
use std::any::Any;
use wgpu::BufferAddress;

/// SsaoNode 实现了屏幕空间环境光遮蔽 (Screen Space Ambient Occlusion)。
///
/// 该节点不负责渲染法线和深度，而是直接复用 `PrePassNode` 的产出。
///
/// 该节点包含两个主要的计算步骤：
/// 1. **SSAO Main Pass**: 采样 `PrePass` 的法线和深度，在每个像素周围进行随机半球采样，
///    计算遮蔽因子。
/// 2. **Blur Pass**: 使用一个简单的双边滤波（或均值滤波）对结果进行降噪，生成平滑的遮挡图。
pub struct SsaoNode {
    ssao_pipeline: Option<wgpu::RenderPipeline>,   // Calc SSAO.
    blur_pipeline: Option<wgpu::RenderPipeline>,   // Blur SSAO.
}

impl Default for SsaoNode {
    fn default() -> Self {
        Self {
            ssao_pipeline: None,
            blur_pipeline: None,
        }
    }
}

impl Node for SsaoNode {
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn node_resources(
        &self,
        prepared: &PreparedFrame,
    ) -> crate::render::render_graph::resource::NodeResources {
        use crate::render::render_graph::resource::{ResourceSpec, TextureKey};
        use crate::render::render_graph::standard_resources;

        let camera_buffer_size = CameraUniform::get_uniform_offset_unit()
            * crate::render::render_graph::nodes::prepare_view::MAX_CAMERAS;

        crate::render::render_graph::resource::NodeResources::new()
            .input(
                standard_resources::camera_buffer(),
                ResourceSpec::buffer(camera_buffer_size as u64, wgpu::BufferUsages::UNIFORM),
            )
            .input(
                standard_resources::main_depth(),
                ResourceSpec::Texture(TextureKey {
                    width: 0,
                    height: 0,
                    format: Some(Texture::DEPTH_FORMAT),
                    usage: wgpu::TextureUsages::TEXTURE_BINDING,
                    layers: 1,
                    mip_levels: 1,
                    dimension: wgpu::TextureDimension::D2,
                }),
            )
            .input(
                standard_resources::prepass_normal(),
                ResourceSpec::Texture(TextureKey {
                    width: 0,
                    height: 0,
                    format: Some(wgpu::TextureFormat::Rgba16Float),
                    usage: wgpu::TextureUsages::TEXTURE_BINDING,
                    layers: 1,
                    mip_levels: 1,
                    dimension: wgpu::TextureDimension::D2,
                }),
            )
            .internal(
                standard_resources::ssao_output(),
                ResourceSpec::Texture(TextureKey {
                    width: 0,
                    height: 0,
                    format: Some(wgpu::TextureFormat::R8Unorm),
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                        | wgpu::TextureUsages::TEXTURE_BINDING,
                    layers: 1,
                    mip_levels: 1,
                    dimension: wgpu::TextureDimension::D2,
                }),
            )
            .output(
                standard_resources::ssao_blur(),
                ResourceSpec::Texture(TextureKey {
                    width: 0,
                    height: 0,
                    format: Some(wgpu::TextureFormat::R8Unorm),
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                        | wgpu::TextureUsages::TEXTURE_BINDING,
                    layers: 1,
                    mip_levels: 1,
                    dimension: wgpu::TextureDimension::D2,
                }),
            )
    }

    fn run(&mut self, context: &mut FrameContext) {
        if context.prepared.instance_buffer_size == 0 {
            return;
        }

        let camera_bind_group_layout = context
            .backend
            .get_bind_group_layout("camera_bind_group_layout")
            .unwrap()
            .clone();

        let bindless_bind_group_layout = context
            .backend
            .get_bind_group_layout("bindless_bind_group_layout")
            .unwrap()
            .clone();

        if self.ssao_pipeline.is_none() {
            let device = &context.render_context.device;

            let ssao_bind_group_layout = context.render_context.device.create_bind_group_layout(
                &wgpu::BindGroupLayoutDescriptor {
                    label: Some("SSAO Bind Group Layout"),
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
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
                            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
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
                            ty: wgpu::BindingType::Texture {
                                sample_type: wgpu::TextureSampleType::Float { filterable: false },
                                view_dimension: wgpu::TextureViewDimension::D2,
                                multisampled: false,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 5,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
                            count: None,
                        },
                    ],
                },
            );

            let ssao_pipeline = {
                let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("SSAO Pipeline Layout"),
                    bind_group_layouts: &[Some(&camera_bind_group_layout), Some(&ssao_bind_group_layout)],
                    immediate_size: 0,
                });
                let source = include_str!("../../../shaders/ssao.wgsl")
                    .replace("#import eureka::camera::Camera", crate::render::camera::CAMERA_STRUCT_WGSL);

                let shader = wgpu::ShaderModuleDescriptor {
                    label: Some("SSAO Shader"),
                    source: wgpu::ShaderSource::Wgsl(source.into()),
                };
                create_render_pipeline(
                    device,
                    &layout,
                    Some(wgpu::TextureFormat::R8Unorm),
                    None,
                    &[],
                    shader,
                    "SSAO Pipeline",
                    false,
                    None,
                )
            };

            let blur_bind_group_layout =
                device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("SSAO Blur Bind Group Layout"),
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

            let blur_pipeline = {
                let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("SSAO Blur Pipeline Layout"),
                    bind_group_layouts: &[Some(&blur_bind_group_layout)],
                    immediate_size: 0,
                });
                let shader = wgpu::ShaderModuleDescriptor {
                    label: Some("SSAO Blur Shader"),
                    source: wgpu::ShaderSource::Wgsl(
                        include_str!("../../../shaders/ssao_blur.wgsl").into(),
                    ),
                };
                create_render_pipeline(
                    device,
                    &layout,
                    Some(wgpu::TextureFormat::R8Unorm),
                    None,
                    &[],
                    shader,
                    "SSAO Blur Pipeline",
                    false,
                    None,
                )
            };

            self.ssao_pipeline = Some(ssao_pipeline);
            self.blur_pipeline = Some(blur_pipeline);

            // 永久缓存
            context
                .backend
                .add_bind_group_layout("ssao_bind_group_layout", ssao_bind_group_layout);
            context
                .backend
                .add_bind_group_layout("blur_bind_group_layout", blur_bind_group_layout);
        }

        // Inputs
        let ssao_depth_tex = context.texture(&standard_resources::main_depth());
        let normal_tex = context.texture(&standard_resources::prepass_normal());

        // Internals
        let ssao_tex = context.texture(&standard_resources::ssao_output());

        // Outputs
        let blur_tex = context.texture(&standard_resources::ssao_blur());

        let mut ssao_camera_index = 0;

        // 查找是否有任何 3D 相机开启了 SSAO
        let mut camera_wants_ssao = false;

        for i in 0..context.extracted.cameras.types.len() {
            if context.extracted.cameras.types[i] == CameraType::D3 {
                if context.extracted.cameras.uniforms[i].ssao_enabled == 1 {
                    camera_wants_ssao = true;
                    ssao_camera_index = i;
                    break;
                }
            }
        }

        // 如果全局禁用，或者没有任何相机需要，则跳过SSAO计算
        let skip_ssao = !context.extracted.ssao_enabled
            || !camera_wants_ssao;

        if skip_ssao {
            // 清空blur输出以防复用脏数据
            context
                .encoder
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("SSAO Disabled Clear Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &blur_tex.view,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                });
        }

        // Noise texture (4x4)
        let noise_texture = context.get_texture(
            "SSAO Noise Texture",
            TextureKey {
                width: 4,
                height: 4,
                format: Some(wgpu::TextureFormat::Rgba32Float),
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                layers: 1,
                mip_levels: 1,
                dimension: wgpu::TextureDimension::D2,
            },
        );

        let mut noise_data = Vec::new();
        let mut seed = 42u32;
        for _ in 0..16 {
            noise_data.push(rand_f32(&mut seed) * 2.0 - 1.0);
            noise_data.push(rand_f32(&mut seed) * 2.0 - 1.0);
            noise_data.push(0.0);
            noise_data.push(0.0);
        }

        context.render_context.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &noise_texture.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            bytemuck::cast_slice(&noise_data),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * 16),
                rows_per_image: Some(4),
            },
            wgpu::Extent3d {
                width: 4,
                height: 4,
                depth_or_array_layers: 1,
            },
        );

        // 2. Uniforms
        let mut kernel = [0.0f32; 64 * 4];
        for i in 0..64 {
            let sample = vec3(
                rand_f32(&mut seed) * 2.0 - 1.0,
                rand_f32(&mut seed) * 2.0 - 1.0,
                rand_f32(&mut seed),
            )
            .normalize()
                * rand_f32(&mut seed);

            let mut scale = i as f32 / 64.0;
            scale = lerp(0.1, 1.0, scale * scale);
            let sample = sample * scale;

            kernel[i * 4] = sample.x;
            kernel[i * 4 + 1] = sample.y;
            kernel[i * 4 + 2] = sample.z;
            kernel[i * 4 + 3] = 0.0;
        }

        let ssao_uniform_buffer = context.get_buffer(
            "SSAO Uniform Buffer",
            BufferKey {
                size: size_of_val(&kernel) as u64,
                usage: wgpu::BufferUsages::UNIFORM,
            },
        );

        context.render_context.queue.write_buffer(
            &ssao_uniform_buffer.buffer,
            0,
            bytemuck::cast_slice(&kernel),
        );

        let sampler = context.get_sampler(SamplerKey {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let noise_sampler = context.get_sampler(SamplerKey {
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        if !skip_ssao {
            let (ssao_bind_group, blur_bind_group) = {
                let ssao_bind_group_layout = context
                    .backend
                    .get_bind_group_layout("ssao_bind_group_layout")
                    .unwrap()
                    .clone();
                let blur_bind_group_layout = context
                    .backend
                    .get_bind_group_layout("blur_bind_group_layout")
                    .unwrap()
                    .clone();

                let ssao_bind_group = context.create_bind_group(
                    "ssao_bind_group_layout",
                    vec![
                        ssao_uniform_buffer.id,
                        normal_tex.view_id,
                        ssao_depth_tex.view_id,
                        noise_texture.view_id,
                    ],
                    |ctx| {
                        ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
                            layout: &ssao_bind_group_layout,
                            entries: &[
                                wgpu::BindGroupEntry {
                                    binding: 0,
                                    resource: ssao_uniform_buffer.buffer.as_entire_binding(),
                                },
                                wgpu::BindGroupEntry {
                                    binding: 1,
                                    resource: wgpu::BindingResource::TextureView(&normal_tex.view),
                                },
                                wgpu::BindGroupEntry {
                                    binding: 2,
                                    resource: wgpu::BindingResource::Sampler(&sampler),
                                },
                                wgpu::BindGroupEntry {
                                    binding: 3,
                                    resource: wgpu::BindingResource::TextureView(&ssao_depth_tex.view),
                                },
                                wgpu::BindGroupEntry {
                                    binding: 4,
                                    resource: wgpu::BindingResource::TextureView(&noise_texture.view),
                                },
                                wgpu::BindGroupEntry {
                                    binding: 5,
                                    resource: wgpu::BindingResource::Sampler(&noise_sampler),
                                },
                            ],
                            label: Some("SSAO Bind Group"),
                        })
                    },
                );

                let blur_bind_group = context.create_bind_group(
                    "blur_bind_group_layout",
                    vec![ssao_tex.view_id],
                    |ctx| {
                        ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
                            layout: &blur_bind_group_layout,
                            entries: &[
                                wgpu::BindGroupEntry {
                                    binding: 0,
                                    resource: wgpu::BindingResource::TextureView(&ssao_tex.view),
                                },
                                wgpu::BindGroupEntry {
                                    binding: 1,
                                    resource: wgpu::BindingResource::Sampler(&sampler),
                                },
                            ],
                            label: Some("SSAO Blur Bind Group"),
                        })
                    },
                );

                (ssao_bind_group, blur_bind_group)
            };

            let camera_bind_group_layout = context
                .backend
                .get_bind_group_layout("camera_bind_group_layout")
                .unwrap()
                .clone();

            let camera_buffer = context.buffer(&standard_resources::camera_buffer());

            let camera_bind_group =
                context.create_bind_group("camera_bind_group_layout", vec![camera_buffer.id], |ctx| {
                    ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
                        layout: &camera_bind_group_layout,
                        entries: &[wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                                buffer: &camera_buffer.buffer,
                                offset: 0,
                                size: Some(
                                    wgpu::BufferSize::new(size_of::<CameraUniform>() as u64).unwrap(),
                                ),
                            }),
                        }],
                        label: Some("Camera Bind Group"),
                    })
                });

            {
                let mut render_pass = context
                    .encoder
                    .begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("SSAO Pass"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &ssao_tex.view,
                            depth_slice: None,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
                                store: wgpu::StoreOp::Store,
                            },
                        })],
                        depth_stencil_attachment: None,
                        timestamp_writes: None,
                        occlusion_query_set: None,
                        multiview_mask: None,
                    });

                render_pass.set_pipeline(self.ssao_pipeline.as_ref().unwrap());
                render_pass.set_bind_group(0, &camera_bind_group, &[0]);
                render_pass.set_bind_group(1, &ssao_bind_group, &[]);
                render_pass.draw(0..3, 0..1);
            }

            {
                let mut render_pass = context
                    .encoder
                    .begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("SSAO Blur Pass"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &blur_tex.view,
                            depth_slice: None,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
                                store: wgpu::StoreOp::Store,
                            },
                        })],
                        depth_stencil_attachment: None,
                        timestamp_writes: None,
                        occlusion_query_set: None,
                        multiview_mask: None,
                    });

                render_pass.set_pipeline(self.blur_pipeline.as_ref().unwrap());
                render_pass.set_bind_group(0, &blur_bind_group, &[]);
                render_pass.draw(0..3, 0..1);
            }
        }
    }
}

fn rand_f32(seed: &mut u32) -> f32 {
    *seed = seed.wrapping_mul(1103515245).wrapping_add(12345);
    ((*seed >> 16) & 0x7FFF) as f32 / 32767.0
}

fn lerp(a: f32, b: f32, f: f32) -> f32 {
    a + f * (b - a)
}
