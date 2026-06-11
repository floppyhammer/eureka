use crate::render::camera::{CameraType, CameraUniform};
use crate::render::light::{CascadeUniform, LightUniform, MAX_POINT_LIGHTS};
use crate::render::render_graph::resource::BufferKey;
use crate::render::render_graph::{standard_resources, SamplerKey};
use crate::render::render_graph::{FrameContext, Node, TextureKey};
use crate::render::vertex::{Vertex3d, VertexBuffer};
use crate::render::{create_render_pipeline, InstanceRaw, MeshRenderResources, Texture};
use std::any::Any;
use wgpu::BufferAddress;

pub struct MeshNode {
    pipeline: Option<wgpu::RenderPipeline>,
}

impl Default for MeshNode {
    fn default() -> Self {
        Self { pipeline: None }
    }
}

impl Node for MeshNode {
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn node_resources(&self) -> crate::render::render_graph::resource::NodeResources {
        use crate::render::render_graph::resource::{ResourceSpec, TextureKey};
        use crate::render::render_graph::standard_resources;
        use crate::render::Texture;

        let camera_buffer_size = CameraUniform::get_uniform_offset_unit()
            * crate::render::render_graph::nodes::prepare_view::MAX_CAMERAS;

        crate::render::render_graph::resource::NodeResources::new()
            .input(
                standard_resources::camera_buffer(),
                ResourceSpec::buffer(camera_buffer_size as u64, wgpu::BufferUsages::UNIFORM),
            )
            .input(
                standard_resources::shadow_cascade_buffer(),
                ResourceSpec::buffer(
                    size_of::<CascadeUniform>() as BufferAddress,
                    wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                ),
            )
            .optional_input(
                standard_resources::point_shadow_map(),
                ResourceSpec::Texture(TextureKey {
                    width: 512,
                    height: 512,
                    format: Some(Texture::DEPTH_FORMAT),
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                        | wgpu::TextureUsages::TEXTURE_BINDING,
                    layers: (MAX_POINT_LIGHTS * 6) as u32,
                }),
            )
            .optional_input(
                standard_resources::directional_shadow_map(),
                ResourceSpec::Texture(TextureKey {
                    width: 2048,
                    height: 2048,
                    format: Some(Texture::DEPTH_FORMAT),
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                        | wgpu::TextureUsages::TEXTURE_BINDING,
                    layers: crate::render::light::NUM_CASCADES as u32,
                }),
            )
            .optional_input(
                standard_resources::ssao_blur(),
                ResourceSpec::Texture(TextureKey {
                    width: 0,
                    height: 0,
                    format: Some(wgpu::TextureFormat::R8Unorm),
                    usage: wgpu::TextureUsages::TEXTURE_BINDING,
                    layers: 1,
                }),
            )
            .output(
                standard_resources::light_uniform_buffer(),
                ResourceSpec::buffer(
                    size_of::<LightUniform>() as u64,
                    wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                ),
            )
            .output(
                standard_resources::main_color(),
                ResourceSpec::Texture(TextureKey {
                    width: 0,
                    height: 0,
                    format: Some(wgpu::TextureFormat::Rgba16Float),
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                        | wgpu::TextureUsages::TEXTURE_BINDING,
                    layers: 1,
                }),
            )
            .output(
                standard_resources::main_depth(),
                ResourceSpec::Texture(TextureKey {
                    width: 0,
                    height: 0,
                    format: Some(Texture::DEPTH_FORMAT),
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                        | wgpu::TextureUsages::TEXTURE_BINDING,
                    layers: 1,
                }),
            )
    }

    fn prepare(&mut self, context: &mut FrameContext) {
        let device = &context.render_context.device;
        let world = &*context.render_world;
        let resources = &world.mesh_render_resources;

        let camera_bind_group_layout = context
            .pool
            .get_bind_group_layout("camera_bind_group_layout")
            .unwrap()
            .clone();

        let light_bind_group_layout = context
            .pool
            .get_bind_group_layout("light_bind_group_layout");
        if light_bind_group_layout.is_none() {
            let light_bind_group_layout =
                device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
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
                                multisampled: false,
                                view_dimension: wgpu::TextureViewDimension::D2Array,
                                sample_type: wgpu::TextureSampleType::Depth,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 2,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 3,
                            visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 4,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Texture {
                                multisampled: false,
                                view_dimension: wgpu::TextureViewDimension::CubeArray,
                                sample_type: wgpu::TextureSampleType::Depth,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 5,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Texture {
                                multisampled: false,
                                view_dimension: wgpu::TextureViewDimension::D2,
                                sample_type: wgpu::TextureSampleType::Float { filterable: false },
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 6,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Texture {
                                multisampled: false,
                                view_dimension: wgpu::TextureViewDimension::Cube,
                                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 7,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                            count: None,
                        },
                    ],
                    label: Some("mesh light bind group layout"),
                });

            context
                .pool
                .add_bind_group_layout("light_bind_group_layout", light_bind_group_layout);
        }

        let light_bind_group_layout = context
            .pool
            .get_bind_group_layout("light_bind_group_layout")
            .unwrap()
            .clone();

        if self.pipeline.is_none() {
            let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Mesh Layout"),
                bind_group_layouts: &[
                    &camera_bind_group_layout,
                    &light_bind_group_layout,
                    &resources.bindless_bind_group_layout,
                ],
                push_constant_ranges: &[],
            });

            let shader = wgpu::ShaderModuleDescriptor {
                label: Some("Mesh Shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("../../../shaders/mesh.wgsl").into()),
            };

            self.pipeline = Some(create_render_pipeline(
                device,
                &pipeline_layout,
                Some(wgpu::TextureFormat::Rgba16Float),
                Some(Texture::DEPTH_FORMAT),
                &[Vertex3d::desc(), InstanceRaw::desc()],
                shader,
                "Opaque Mesh Bindless",
                false,
                Some(wgpu::Face::Back),
            ));
        }

        // Upload light uniforms ----------------------
        let lights = world.extracted.lights.clone();

        let light_uniform_buffer = context.buffer(&standard_resources::light_uniform_buffer());

        let mut light_uniform = LightUniform::default();
        light_uniform.ambient_color = [1.0, 1.0, 1.0];
        light_uniform.ambient_strength = 0.01;
        light_uniform.point_light_count = lights.point_lights.len() as u32;
        for i in 0..lights.point_lights.len() {
            light_uniform.point_lights[i] = lights.point_lights[i];
        }
        if let Some(dl) = lights.directional_light {
            light_uniform.directional_light = dl;
        }
        context.render_context.queue.write_buffer(
            &light_uniform_buffer.buffer,
            0,
            bytemuck::cast_slice(&[light_uniform]),
        );
        // ------------------------------------------------
    }

    fn run(&mut self, context: &mut FrameContext) {
        let main_color = FrameContext::texture(context, &standard_resources::main_color());
        let main_depth = context.texture(&standard_resources::main_depth());

        let ssao_blur = context.texture(&standard_resources::ssao_blur());
        let directional_shadow_map = context.texture(&standard_resources::directional_shadow_map());
        let camera_buffer = context.buffer(&standard_resources::camera_buffer());

        let light_bind_group_layout = context
            .pool
            .get_bind_group_layout("light_bind_group_layout")
            .unwrap()
            .clone();

        let camera_bind_group_layout = context
            .pool
            .get_bind_group_layout("camera_bind_group_layout")
            .unwrap()
            .clone();

        let camera_bind_group =
            context.create_bind_group(&camera_bind_group_layout, vec![camera_buffer.id], |ctx| {
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

        if context.render_world.extracted.meshes.is_empty() {
            return;
        }

        // --- 动态更新 Light Bind Group ---

        let shadow_sampler = context.get_sampler(SamplerKey {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            compare: Some(wgpu::CompareFunction::LessEqual),
            ..Default::default()
        });

        let skybox_sampler = context.get_sampler(SamplerKey {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        // CSM 视图 ----------------
        let cascade_view = directional_shadow_map.get_view(&wgpu::TextureViewDescriptor {
            label: Some("shadow cascade view"),
            format: Some(Texture::DEPTH_FORMAT),
            dimension: Some(wgpu::TextureViewDimension::D2Array),
            usage: Some(wgpu::TextureUsages::TEXTURE_BINDING),
            aspect: wgpu::TextureAspect::DepthOnly,
            array_layer_count: Some(crate::render::light::NUM_CASCADES as u32),
            ..Default::default()
        });

        let cascade_uniform_buffer = context.buffer(&standard_resources::shadow_cascade_buffer());
        // -----------------------

        // Intervals
        let light_uniform_buffer = {
            let buffer_key = BufferKey {
                size: size_of::<LightUniform>() as u64,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            };

            context.get_buffer_by_id(&standard_resources::light_uniform_buffer(), buffer_key)
        };

        let point_shadow_map = {
            let point_shadow_map_key = TextureKey {
                width: 512,
                height: 512,
                format: Some(Texture::DEPTH_FORMAT),
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                layers: (MAX_POINT_LIGHTS * 6) as u32,
            };

            context.get_texture_by_id(
                &standard_resources::point_shadow_map(),
                point_shadow_map_key,
            )
        };

        let point_shadow_map_view = {
            point_shadow_map.get_view(&wgpu::TextureViewDescriptor {
                label: Some("point shadow map view"),
                format: Some(Texture::DEPTH_FORMAT),
                dimension: Some(wgpu::TextureViewDimension::CubeArray),
                aspect: wgpu::TextureAspect::DepthOnly,
                array_layer_count: Some(MAX_POINT_LIGHTS as u32 * 6),
                ..Default::default()
            })
        };
        // } else {
        //     context
        //         .render_world
        //         .mesh_render_resources
        //         .dummy_cube_view
        //         .clone()
        // };

        let (sky_view, sky_view_id) =
            if let Some(id) = context.render_world.sky_imported_resources.texture {
                let tex = context.render_world.imported_texture_cache.get(id).unwrap();
                (tex.view.clone(), tex.view_id)
            } else {
                (
                    context
                        .render_world
                        .mesh_render_resources
                        .dummy_cube_view
                        .clone(),
                    0,
                )
            };

        let light_bind_group = context.create_bind_group(
            &light_bind_group_layout,
            vec![
                light_uniform_buffer.id,
                ssao_blur.view_id,
                cascade_view.1,
                cascade_uniform_buffer.id,
                point_shadow_map_view.1,
                sky_view_id,
            ],
            |ctx| {
                ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    layout: &light_bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: light_uniform_buffer.buffer.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::TextureView(&cascade_view.0),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: wgpu::BindingResource::Sampler(&shadow_sampler),
                        },
                        wgpu::BindGroupEntry {
                            binding: 3,
                            resource: cascade_uniform_buffer.buffer.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 4,
                            resource: wgpu::BindingResource::TextureView(&point_shadow_map_view.0),
                        },
                        wgpu::BindGroupEntry {
                            binding: 5,
                            resource: wgpu::BindingResource::TextureView(&ssao_blur.view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 6,
                            resource: wgpu::BindingResource::TextureView(&sky_view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 7,
                            resource: wgpu::BindingResource::Sampler(&skybox_sampler),
                        },
                    ],
                    label: Some("light bind group (dynamic)"),
                })
            },
        );

        let world = &*context.render_world;
        let mut render_pass = context
            .encoder
            .begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("mesh render pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &main_color.view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &main_depth.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

        for camera_idx in 0..world.extracted.cameras.uniforms.len() {
            let camera_offset = camera_idx as u32 * CameraUniform::get_uniform_offset_unit();

            if world.extracted.cameras.types[camera_idx] == CameraType::D3 {
                render_meshes(
                    &camera_bind_group,
                    &light_bind_group,
                    world
                        .mesh_render_resources
                        .bindless_bind_group
                        .as_ref()
                        .unwrap(),
                    &world.mesh_render_resources,
                    camera_offset,
                    &mut render_pass,
                    self.pipeline.as_ref().unwrap(),
                );
            }
        }
    }
}

pub(crate) fn render_meshes<'a, 'b: 'a>(
    camera_bind_group: &'b wgpu::BindGroup,
    light_bind_group: &'b wgpu::BindGroup,
    bindless_bind_group: &'b wgpu::BindGroup,
    mesh_render_resources: &'b MeshRenderResources,
    camera_offset: u32,
    render_pass: &mut wgpu::RenderPass<'a>,
    pipeline: &'b wgpu::RenderPipeline,
) {
    render_pass.set_pipeline(pipeline);

    render_pass.set_bind_group(0, camera_bind_group, &[camera_offset]);
    render_pass.set_bind_group(1, light_bind_group, &[]);
    render_pass.set_bind_group(2, bindless_bind_group, &[]);

    render_pass.set_vertex_buffer(
        0,
        mesh_render_resources.mesh_allocator.vertex_buffer.slice(..),
    );
    render_pass.set_vertex_buffer(
        1,
        mesh_render_resources
            .global_visible_instance_buffer
            .as_ref()
            .unwrap()
            .slice(..),
    );
    render_pass.set_index_buffer(
        mesh_render_resources.mesh_allocator.index_buffer.slice(..),
        wgpu::IndexFormat::Uint32,
    );

    if !mesh_render_resources.draw_counts.is_empty() && mesh_render_resources.draw_counts[0] > 0 {
        render_pass.multi_draw_indexed_indirect(
            mesh_render_resources
                .global_indirect_buffer
                .as_ref()
                .unwrap(),
            0,
            mesh_render_resources.draw_counts[0],
        );
    }

    // // todo: move to its own node
    // gizmo_render_resources.render(
    //     render_pass,
    //     camera_bind_group,
    // );
}
