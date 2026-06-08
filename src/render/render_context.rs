use std::time::Instant;

/// Contains render context (but not GPU resources)
pub struct RenderContext<'a> {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub surface: wgpu::Surface<'a>,
    pub surface_config: wgpu::SurfaceConfiguration,
    pub frames_in_flight: u32,
}

impl<'a> RenderContext<'a> {
    pub(crate) fn new<'b: 'a>(
        surface: wgpu::Surface<'b>,
        surface_config: wgpu::SurfaceConfiguration,
        device: wgpu::Device,
        queue: wgpu::Queue,
        frames_in_flight: u32,
    ) -> Self {
        let now = Instant::now();

        let context = Self {
            device,
            queue,
            surface,
            surface_config,
            frames_in_flight,
        };

        let elapsed_time = now.elapsed();
        log::info!(
            "Render context setup took {} milliseconds",
            elapsed_time.as_millis()
        );

        context
    }
}

/// Set up resource pipeline using the pipeline layout.
pub fn create_render_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    color_format: Option<wgpu::TextureFormat>,
    depth_format: Option<wgpu::TextureFormat>,
    vertex_layouts: &[wgpu::VertexBufferLayout],
    shader: wgpu::ShaderModuleDescriptor,
    label: &str,
    transparency: bool,
    cull_mode: Option<wgpu::Face>,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(shader);

    // 1. 先准备好混合状态
    let blend = Some(if !transparency {
        wgpu::BlendState {
            alpha: wgpu::BlendComponent::REPLACE,
            color: wgpu::BlendComponent::REPLACE,
        }
    } else {
        wgpu::BlendState {
            alpha: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                operation: wgpu::BlendOperation::Add,
            },
            color: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                operation: wgpu::BlendOperation::Add,
            },
        }
    });

    // 2. 创建一个拥有所有权的目标状态数组（如果有颜色格式的话）
    let targets = color_format.map(|format| {
        [Some(wgpu::ColorTargetState {
            format,
            blend,
            write_mask: wgpu::ColorWrites::ALL,
        })]
    });

    // 3. 将数组引用映射到 FragmentState
    let fragment = targets.as_ref().map(|targets| wgpu::FragmentState {
        module: &shader,
        entry_point: Some("fs_main"),
        compilation_options: Default::default(),
        targets,
    });

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(label),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            compilation_options: Default::default(),
            buffers: vertex_layouts,
        },
        fragment,
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode,
            polygon_mode: wgpu::PolygonMode::Fill,
            unclipped_depth: false,
            conservative: false,
        },
        depth_stencil: depth_format.map(|format| wgpu::DepthStencilState {
            format,
            depth_write_enabled: !transparency,
            depth_compare: wgpu::CompareFunction::LessEqual,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState {
            count: 1,
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
        multiview: None,
        cache: None,
    })
}