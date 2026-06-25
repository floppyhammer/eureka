use crate::render::render_backend::PreparedFrame;
use crate::render::render_graph::{standard_resources, FrameContext, Node, SamplerKey, TextureKey};
use crate::render::Texture;
use std::any::Any;

pub struct SsgiNode {
    pipeline: Option<wgpu::RenderPipeline>,
}

impl Default for SsgiNode {
    fn default() -> Self {
        Self { pipeline: None }
    }
}

impl Node for SsgiNode {
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn node_resources(
        &self,
        _prepared: &PreparedFrame,
    ) -> crate::render::render_graph::resource::NodeResources {
        use crate::render::render_graph::resource::{NodeResources, ResourceSpec};

        let hdr_spec = ResourceSpec::Texture(TextureKey {
            width: 0,
            height: 0,
            format: Some(wgpu::TextureFormat::Rgba16Float),
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING,
            layers: 1,
            mip_levels: 1,
            dimension: wgpu::TextureDimension::D2,
        });

        NodeResources::new()
            .input(standard_resources::taa_output(), hdr_spec.clone())
            .input(standard_resources::prepass_normal(), ResourceSpec::Texture(TextureKey::default()))
            .input(standard_resources::main_depth(), ResourceSpec::Texture(TextureKey::default()))
            .input(
                standard_resources::camera_buffer(),
                ResourceSpec::buffer(
                    size_of::<crate::render::camera::CameraUniform>() as u64 * 16,
                    wgpu::BufferUsages::UNIFORM,
                ),
            )
            .output(standard_resources::ssgi_output(), hdr_spec)
    }

    fn run(&mut self, context: &mut FrameContext) {
        if self.pipeline.is_none() {
            let device = &context.render_context.device;
            let camera_layout = context.backend.get_bind_group_layout("camera_bind_group_layout").unwrap();

            let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("SSGI Bind Group Layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0, visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: true, min_binding_size: None },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1, visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: true }, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2, visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: true }, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 3, visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Depth, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 4, visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 5, visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });

            let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("SSGI Pipeline Layout"),
                bind_group_layouts: &[Some(&bind_group_layout)],
                immediate_size: 0,
            });

            let source = include_str!("../../../shaders/ssgi.wgsl")
                .replace("#import eureka::camera::Camera", crate::render::camera::CAMERA_STRUCT_WGSL);

            self.pipeline = Some(crate::render::create_render_pipeline(
                device, &pipeline_layout, Some(wgpu::TextureFormat::Rgba16Float), None,
                &[], wgpu::ShaderModuleDescriptor { label: Some("SSGI Shader"), source: wgpu::ShaderSource::Wgsl(source.into()) },
                "SSGI Pipeline", false, None,
            ));

            context.backend.add_bind_group_layout("ssgi_bind_group_layout", bind_group_layout);
        }

        let color = context.texture(&standard_resources::taa_output());
        let normal = context.texture(&standard_resources::prepass_normal());
        let depth = context.texture(&standard_resources::main_depth());
        let output = context.texture(&standard_resources::ssgi_output());
        let camera_buffer = context.buffer(&standard_resources::camera_buffer());
        let sampler = context.get_sampler(SamplerKey { mag_filter: wgpu::FilterMode::Nearest, min_filter: wgpu::FilterMode::Nearest, ..Default::default() });
        let default_sampler = context.get_sampler(SamplerKey::default());

        let bind_group = context.render_context.device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &context.backend.get_bind_group_layout("ssgi_bind_group_layout").unwrap(),
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding { buffer: &camera_buffer.buffer, offset: 0, size: Some(wgpu::BufferSize::new(size_of::<crate::render::camera::CameraUniform>() as u64).unwrap()) }) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&color.view) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(&normal.view) },
                wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::TextureView(&depth.view) },
                wgpu::BindGroupEntry { binding: 4, resource: wgpu::BindingResource::Sampler(&sampler) },
                wgpu::BindGroupEntry { binding: 5, resource: wgpu::BindingResource::Sampler(&default_sampler) },
            ],
            label: Some("SSGI Bind Group"),
        });

        let mut rpass = context.encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("SSGI Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment { view: &output.view, depth_slice: None, resolve_target: None, ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::BLACK), store: wgpu::StoreOp::Store } })],
            depth_stencil_attachment: None, timestamp_writes: None, occlusion_query_set: None, multiview_mask: None,
        });

        rpass.set_pipeline(self.pipeline.as_ref().unwrap());
        rpass.set_bind_group(0, &bind_group, &[0]);
        rpass.draw(0..3, 0..1);
    }
}
