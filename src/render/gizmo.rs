use crate::render::{RenderContext, Texture};

pub(crate) struct Gizmo {
    pub(crate) color: [f32; 3],
}

pub(crate) struct GizmoRenderResources {
    pub(crate) pipeline: wgpu::RenderPipeline,
}

impl GizmoRenderResources {
    pub(crate) fn new(
        render_server: &RenderContext,
        camera_bind_group_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let device = &render_server.device;

        let pipeline = {
            let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("gizmo pipeline layout"),
                bind_group_layouts: &[Some(camera_bind_group_layout)],
                immediate_size: 0,
            });

            let shader = wgpu::ShaderModuleDescriptor {
                label: Some("gizmo shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/gizmo.wgsl").into()),
            };
            let shader_module = device.create_shader_module(shader);

            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("gizmo pipeline"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader_module,
                    entry_point: Some("vs_main_grid"),
                    compilation_options: Default::default(),
                    buffers: &[],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader_module,
                    entry_point: Some("fs_main_grid"),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: render_server.surface_config.format,
                        blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleStrip, // Has to be triangle strip.
                    front_face: wgpu::FrontFace::Cw,
                    cull_mode: None,
                    ..Default::default()
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: Texture::DEPTH_FORMAT,
                    depth_write_enabled: Some(false),
                    depth_compare: Some(wgpu::CompareFunction::Less),
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
                multisample: wgpu::MultisampleState::default(),
                cache: None,
                multiview_mask: None,
            })
        };

        Self { pipeline }
    }

    pub(crate) fn render<'a, 'b: 'a>(
        &'b self,
        render_pass: &mut wgpu::RenderPass<'a>,
        camera_bind_group: &'b wgpu::BindGroup,
    ) {
        render_pass.set_pipeline(&self.pipeline);

        // FIXME
        // Set camera group.
        render_pass.set_bind_group(0, camera_bind_group, &[0]);

        render_pass.draw(0..4, 0..1);
    }
}
