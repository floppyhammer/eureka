use std::time::Instant;

/// Contains render context (but not GPU resources)
pub struct RenderServer<'a> {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub surface: wgpu::Surface<'a>,
    pub surface_config: wgpu::SurfaceConfiguration,
    // bind_group_layout_cache: HashMap<&'static str, wgpu::BindGroupLayout>,
    // material_3d_bind_group_layout_cache: HashMap<u32, wgpu::BindGroupLayout>,
    // render_pipeline_cache: HashMap<&'static str, wgpu::RenderPipeline>,
    // material_3d_pipeline_cache: HashMap<u32, wgpu::RenderPipeline>,
}

impl<'a> RenderServer<'a> {
    pub(crate) fn new<'b: 'a>(
        surface: wgpu::Surface<'b>,
        surface_config: wgpu::SurfaceConfiguration,
        device: wgpu::Device,
        queue: wgpu::Queue,
    ) -> Self {
        let now = Instant::now();

        // let mut bind_group_layout_cache = HashMap::new();
        // let material_3d_bind_group_layout_cache = HashMap::new();

        // Create various bind group layouts, which are used to create bind groups.
        // ------------------------------------------------------------------

        //
        // {
        //     let label = "sprite3d params bind group layout";
        //
        //     let bind_group_layout =
        //         device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        //             entries: &[wgpu::BindGroupLayoutEntry {
        //                 binding: 0,
        //                 visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
        //                 ty: wgpu::BindingType::Buffer {
        //                     ty: wgpu::BufferBindingType::Uniform,
        //                     has_dynamic_offset: false,
        //                     min_binding_size: None,
        //                 },
        //                 count: None,
        //             }],
        //             label: Some(label),
        //         });
        //
        //     bind_group_layout_cache.insert(label, bind_group_layout);
        // }

        let server = Self {
            device,
            queue,
            surface,
            surface_config,
        };

        let elapsed_time = now.elapsed();
        log::info!(
            "Render server setup took {} milliseconds",
            elapsed_time.as_millis()
        );

        server
    }

    //
    // pub fn build_sprite3d_pipeline(&mut self) {
    //     let pipeline_label = "sprite3d pipeline";
    //
    //     let pipeline = {
    //         // Set up resource pipeline layout using bind group layouts.
    //         let pipeline_layout =
    //             self.device
    //                 .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
    //                     label: Some("sprite3d pipeline layout"),
    //                     bind_group_layouts: &[
    //                         self.get_bind_group_layout("camera bind group layout")
    //                             .unwrap(),
    //                         self.get_bind_group_layout("sprite texture bind group layout")
    //                             .unwrap(),
    //                         self.get_bind_group_layout("sprite3d params bind group layout")
    //                             .unwrap(),
    //                     ],
    //                     push_constant_ranges: &[],
    //                 });
    //
    //         // Shader descriptor, not a shader module yet.
    //         let shader = wgpu::ShaderModuleDescriptor {
    //             label: Some("sprite3d shader"),
    //             source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/sprite3d.wgsl").into()),
    //         };
    //
    //         // FIXME(floppyhammer): Transparency
    //         create_render_pipeline(
    //             &self.device,
    //             &pipeline_layout,
    //             self.surface_config.format,
    //             Some(Texture::DEPTH_FORMAT),
    //             &[Vertex3d::desc()],
    //             shader,
    //             pipeline_label,
    //             false,
    //             Some(wgpu::Face::Back),
    //         )
    //     };
    //
    //     // self.render_pipeline_cache.insert(pipeline_label, pipeline);
    // }
    //
    // pub fn build_sprite_v_pipeline(&mut self) {
    //     let pipeline_label = "sprite v pipeline";
    //
    //     // Vector sprite pipeline.
    //     let pipeline = {
    //         // Set up resource pipeline layout using bind group layouts.
    //         let pipeline_layout =
    //             self.device
    //                 .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
    //                     label: Some("sprite v pipeline layout"),
    //                     bind_group_layouts: &[self
    //                         .get_bind_group_layout("camera bind group layout")
    //                         .unwrap()],
    //                     push_constant_ranges: &[],
    //                 });
    //
    //         // Shader descriptor, not a shader module yet.
    //         let shader = wgpu::ShaderModuleDescriptor {
    //             label: Some("sprite v shader"),
    //             source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/vector.wgsl").into()),
    //         };
    //
    //         create_render_pipeline(
    //             &self.device,
    //             &pipeline_layout,
    //             self.surface_config.format,
    //             Some(Texture::DEPTH_FORMAT),
    //             &[VectorVertex::desc()],
    //             shader,
    //             pipeline_label,
    //             true,
    //             None,
    //         )
    //     };
    //
    //     // self.render_pipeline_cache.insert(pipeline_label, pipeline);
    // }
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
