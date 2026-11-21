use crate::render::camera::CameraUniform;
use crate::render::create_render_pipeline;
use crate::render::render_backend::PreparedFrame;
use crate::render::render_graph::config::DebugViewmode;
use crate::render::render_graph::{standard_resources, FrameContext, Node, SamplerKey};
use std::any::Any;

/// 调试视图节点，将渲染管线中间缓冲区可视化到屏幕
///
/// 支持以下调试模式：
/// - **Default**: 正常渲染输出（直通 main_color）
/// - **Normals**: 法线可视化（RGB = XYZ，从 [-1,1] 映射到 [0,1]）
/// - **Depth**: 深度缓冲区可视化（线性化深度）
/// - **Velocity**: 运动矢量可视化（方向→颜色，幅度→亮度）
/// - **Prepass**: PrePass 法线/粗糙度缓冲区（RGB=法线，A=粗糙度）
pub struct DebugViewmodeNode {
    pipeline: Option<wgpu::RenderPipeline>,
}

impl Default for DebugViewmodeNode {
    fn default() -> Self {
        Self { pipeline: None }
    }
}

impl Node for DebugViewmodeNode {
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn node_resources(
        &self,
        _prepared: &PreparedFrame,
    ) -> crate::render::render_graph::resource::NodeResources {
        use crate::render::render_graph::resource::{NodeResources, ResourceSpec};

        NodeResources::new()
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
            .input(
                standard_resources::main_depth(),
                ResourceSpec::texture(
                    0,
                    0,
                    Some(wgpu::TextureFormat::Depth32Float),
                    wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
                    1,
                ),
            )
            .input(
                standard_resources::prepass_normal(),
                ResourceSpec::texture(
                    0,
                    0,
                    Some(wgpu::TextureFormat::Rgba16Float),
                    wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
                    1,
                ),
            )
            .input(
                standard_resources::prepass_velocity(),
                ResourceSpec::texture(
                    0,
                    0,
                    Some(wgpu::TextureFormat::Rg16Float),
                    wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
                    1,
                ),
            )
            .input(
                standard_resources::camera_buffer(),
                ResourceSpec::buffer(
                    CameraUniform::get_uniform_offset_unit() as u64,
                    wgpu::BufferUsages::UNIFORM,
                ),
            )
            .output(
                standard_resources::debug_viewmode_uniform(),
                ResourceSpec::buffer(16, wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST),
            )
            .output(
                standard_resources::final_output(),
                ResourceSpec::texture(
                    0,
                    0,
                    None,
                    wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
                    1,
                ),
            )
    }

    fn run(&mut self, context: &mut FrameContext) {
        // Read current viewmode from config
        let viewmode = context.extracted.render_config.debug_viewmode;
        
        // Default mode: skip entirely, let the normal render pipeline pass through
        if viewmode == DebugViewmode::Default {
            return;
        }
        
        let mode_value: u32 = match viewmode {
            DebugViewmode::Default => 0,
            DebugViewmode::Normals => 1,
            DebugViewmode::Depth => 2,
            DebugViewmode::Velocity => 3,
            DebugViewmode::Prepass => 4,
        };
        let uniform_data = [mode_value, 0u32, 0u32, 0u32];

        let debug_uniform_buf = context.buffer(&standard_resources::debug_viewmode_uniform());
        context.write_buffer(&debug_uniform_buf.buffer, &uniform_data);

        // Create pipeline on first run
        if self.pipeline.is_none() {
            let device = &context.render_context.device;

            let bind_group_layout =
                device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Debug Viewmode Bind Group Layout"),
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
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
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
                            ty: wgpu::BindingType::Texture {
                                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                                view_dimension: wgpu::TextureViewDimension::D2,
                                multisampled: false,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 5,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Texture {
                                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                                view_dimension: wgpu::TextureViewDimension::D2,
                                multisampled: false,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 6,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                            count: None,
                        },
                    ],
                });

            let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Debug Viewmode Pipeline Layout"),
                bind_group_layouts: &[Some(&bind_group_layout)],
                immediate_size: 0,
            });

            let source = include_str!("../../../shaders/debug_viewmode.wgsl")
                .replace("#import eureka::camera::Camera", crate::render::camera::CAMERA_STRUCT_WGSL);

            let shader = wgpu::ShaderModuleDescriptor {
                label: Some("Debug Viewmode Shader"),
                source: wgpu::ShaderSource::Wgsl(source.into()),
            };

            self.pipeline = Some(create_render_pipeline(
                device,
                &pipeline_layout,
                Some(context.render_context.surface_config.format),
                None,
                &[],
                shader,
                "Debug Viewmode Pipeline",
                false,
                None,
            ));

            context
                .backend
                .add_bind_group_layout("debug_viewmode_bind_group_layout", bind_group_layout);
        }

        // Get resources
        let main_color_tex = context.texture(&standard_resources::main_color());
        let main_depth_tex = context.texture(&standard_resources::main_depth());
        let prepass_normal_tex = context.texture(&standard_resources::prepass_normal());
        let prepass_velocity_tex = context.texture(&standard_resources::prepass_velocity());
        let camera_buf = context.buffer(&standard_resources::camera_buffer());

        let sampler = context.get_sampler(SamplerKey {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let bind_group_layout = context
            .backend
            .get_bind_group_layout("debug_viewmode_bind_group_layout")
            .unwrap()
            .clone();

        let bind_group = context.create_bind_group(
            "debug_viewmode_bind_group_layout",
            vec![
                debug_uniform_buf.id,
                camera_buf.id,
                main_color_tex.id,
                main_depth_tex.view_id,
                prepass_normal_tex.id,
                prepass_velocity_tex.id,
            ],
            |ctx| {
                ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("Debug Viewmode Bind Group"),
                    layout: &bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: debug_uniform_buf.buffer.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: camera_buf.buffer.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: wgpu::BindingResource::TextureView(&main_color_tex.view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 3,
                            resource: wgpu::BindingResource::TextureView(&main_depth_tex.view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 4,
                            resource: wgpu::BindingResource::TextureView(&prepass_normal_tex.view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 5,
                            resource: wgpu::BindingResource::TextureView(&prepass_velocity_tex.view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 6,
                            resource: wgpu::BindingResource::Sampler(&sampler),
                        },
                    ],
                })
            },
        );

        let mut render_pass = context
            .encoder
            .begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Debug Viewmode Pass"),
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
                multiview_mask: None,
            });

        render_pass.set_pipeline(self.pipeline.as_ref().unwrap());
        render_pass.set_bind_group(0, &bind_group, &[]);
        render_pass.draw(0..3, 0..1);
    }
}
