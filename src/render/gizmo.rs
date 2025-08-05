use crate::render::material::{MaterialCache, MaterialId};
use crate::render::{InstanceMetadata, MeshId, RenderServer, Texture};
use rustybuzz::ttf_parser::gpos::Device;
use std::collections::HashMap;
use wgpu::BindGroupLayout;

pub(crate) struct Gizmo {
    pub(crate) color: [f32; 3],
}

pub(crate) struct GizmoRenderResources {
    pub(crate) pipeline: wgpu::RenderPipeline,
}

impl GizmoRenderResources {
    pub(crate) fn new(
        render_server: &RenderServer,
        camera_bind_group_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let device = &render_server.device;

        let pipeline = {
            let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("gizmo pipeline layout"),
                bind_group_layouts: &[camera_bind_group_layout],
                push_constant_ranges: &[],
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
                    entry_point: "vs_main_grid",
                    compilation_options: Default::default(),
                    buffers: &[],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader_module,
                    entry_point: "fs_main_grid",
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
                    depth_write_enabled: false,
                    depth_compare: wgpu::CompareFunction::Less,
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
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
