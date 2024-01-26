// use crate::render::atlas::{AtlasInstance, AtlasInstanceRaw, AtlasParamsUniform};
use crate::render::texture::Texture;
// use crate::render::vertex::{VectorVertex, Vertex2d, Vertex3d, VertexBuffer, VertexSky};
// use crate::scene::CameraUniform;
use crate::{scene, Camera2d, SamplerBindingType};
// use crate::{scene, Camera2d, Camera3d, Light, SamplerBindingType};

use crate::render::bind_group::BindGroupCache;
use crate::render::camera::CameraUniform;
use crate::render::shader_maker::ShaderMaker;
use crate::render::sprite::{DrawSprite2d, ExtractedSprite2d, SpriteRenderResources};
use crate::render::TextureCache;
use cgmath::Point2;
use std::collections::HashMap;
use std::mem;
use std::time::Instant;
use wgpu::util::DeviceExt;
use wgpu::PolygonMode::Point;
use wgpu::{BufferAddress, TextureFormat};

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

        let mut server = Self {
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
    color_format: wgpu::TextureFormat,
    depth_format: Option<wgpu::TextureFormat>,
    vertex_layouts: &[wgpu::VertexBufferLayout],
    shader: wgpu::ShaderModuleDescriptor,
    label: &str,
    transparency: bool,
    cull_mode: Option<wgpu::Face>,
) -> wgpu::RenderPipeline {
    // Create actual shader module using the shader descriptor.
    let shader = device.create_shader_module(shader);

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(label),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: "vs_main",
            buffers: vertex_layouts,
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: "fs_main",
            targets: &[Some(wgpu::ColorTargetState {
                format: color_format,
                blend: Some(if !transparency {
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
                }),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode,
            // Setting this to anything other than Fill requires Features::NON_FILL_POLYGON_MODE
            polygon_mode: wgpu::PolygonMode::Fill,
            // Requires Features::DEPTH_CLIP_CONTROL
            unclipped_depth: false,
            // Requires Features::CONSERVATIVE_RASTERIZATION
            conservative: false,
        },
        depth_stencil: depth_format.map(|format| wgpu::DepthStencilState {
            format,
            depth_write_enabled: !transparency,
            // The depth_compare function tells us when to discard a new pixel.
            // Using LESS means pixels will be drawn front to back.
            // This has to be LESS_OR_EQUAL for correct skybox rendering.
            depth_compare: wgpu::CompareFunction::LessEqual,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState {
            count: 1,
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
        // If the pipeline will be used with a multiview resource pass, this
        // indicates how many array layers the attachments will have.
        multiview: None,
    })
}
