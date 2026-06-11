use crate::render::camera::CameraType;
use crate::render::camera::CameraUniform;
use crate::render::light::{CascadeUniform, LightUniform, MAX_POINT_LIGHTS};
use crate::render::mesh::{ExtractedMesh, MeshCache, MeshRenderResources};
use crate::render::render_graph::{standard_resources, BufferKey, FrameContext, Node, TextureKey};
use crate::render::vertex::{Vertex3d, VertexBuffer};
use crate::render::{create_render_pipeline, InstanceRaw, MeshId, Texture};
use glam::{Mat4, Vec3};
use std::any::Any;
use wgpu::BufferAddress;

pub struct TransparentMeshNode {
    pipeline: Option<wgpu::RenderPipeline>,
    instance_buffer: Option<wgpu::Buffer>,
}

impl Default for TransparentMeshNode {
    fn default() -> Self {
        Self {
            pipeline: None,
            instance_buffer: None,
        }
    }
}

impl Node for TransparentMeshNode {
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn node_resources(&self) -> crate::render::render_graph::resource::NodeResources {
        use crate::render::render_graph::resource::{ResourceId, ResourceSpec, TextureKey};
        use crate::render::render_graph::standard_resources;
        use crate::render::Texture;

        let color_spec = ResourceSpec::Texture(TextureKey {
            width: 0,
            height: 0,
            format: wgpu::TextureFormat::Rgba16Float, // 对齐 HDR
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            layers: 1,
        });
        let depth_spec = ResourceSpec::Texture(TextureKey {
            width: 0,
            height: 0,
            format: Texture::DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            layers: 1,
        });

        crate::render::render_graph::resource::NodeResources::new()
            .input(standard_resources::main_color(), color_spec.clone())
            .input(standard_resources::main_depth(), depth_spec.clone())
            .input(
                standard_resources::camera_buffer(),
                ResourceSpec::buffer(0, wgpu::BufferUsages::UNIFORM),
            )
            .optional_input(
                standard_resources::ssao_blur(),
                ResourceSpec::Texture(TextureKey {
                    width: 0,
                    height: 0,
                    format: wgpu::TextureFormat::R8Unorm,
                    usage: wgpu::TextureUsages::TEXTURE_BINDING,
                    layers: 1,
                }),
            )
            .output(standard_resources::main_color(), color_spec)
            .output(standard_resources::main_depth(), depth_spec)
    }

    fn prepare(&mut self, context: &mut FrameContext) {}

    fn run(&mut self, context: &mut FrameContext) {
        let width = context.render_context.surface_config.width;
        let height = context.render_context.surface_config.height;
        let format = context.render_context.surface_config.format;

        if context.render_world.extracted.transparent_meshes.is_empty() {
            return;
        }

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
            .get_bind_group_layout("light_bind_group_layout")
            .unwrap()
            .clone();

        if self.pipeline.is_none() {
            let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("transparent mesh layout"),
                bind_group_layouts: &[
                    &camera_bind_group_layout,
                    &light_bind_group_layout,
                    &resources.bindless_bind_group_layout,
                ],
                push_constant_ranges: &[],
            });

            let shader = wgpu::ShaderModuleDescriptor {
                label: Some("transparent mesh shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("../../../shaders/mesh.wgsl").into()),
            };

            self.pipeline = Some(create_render_pipeline(
                device,
                &pipeline_layout,
                Some(wgpu::TextureFormat::Rgba16Float), // 对齐 HDR
                Some(Texture::DEPTH_FORMAT),
                &[Vertex3d::desc(), InstanceRaw::desc()],
                shader,
                "Transparent Mesh Bindless",
                true,
                Some(wgpu::Face::Back),
            ));
        }

        let main_color_key = TextureKey {
            width,
            height,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            layers: 1,
        };
        let main_depth_key = TextureKey {
            width,
            height,
            format: Texture::DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            layers: 1,
        };

        let main_color =
            context.get_texture_by_id(&standard_resources::main_color(), main_color_key);
        let main_depth =
            context.get_texture_by_id(&standard_resources::main_depth(), main_depth_key);

        let r8_key = TextureKey {
            width,
            height,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING,
            layers: 1,
        };
        let ssao_blur = context.get_texture_by_id(&standard_resources::ssao_blur(), r8_key);

        let shadow_key = TextureKey {
            width: 2048,
            height: 2048,
            format: Texture::DEPTH_FORMAT,
            usage: wgpu::TextureUsages::TEXTURE_BINDING,
            layers: crate::render::light::NUM_CASCADES as u32,
        };
        let shadow_map =
            context.get_texture_by_id(&standard_resources::directional_shadow_map(), shadow_key);

        // --- 动态更新 Light Bind Group ---
        let device = &context.render_context.device;

        let shadow_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("shadow sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            compare: Some(wgpu::CompareFunction::LessEqual),
            ..Default::default()
        });
        let skybox_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let cascade_uniform_buffer = {
            let cascade_buffer_key = BufferKey {
                size: size_of::<CascadeUniform>() as BufferAddress,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            };

            context.get_buffer_by_id(
                &standard_resources::shadow_cascade_buffer(),
                cascade_buffer_key,
            )
        };

        let light_uniform_buffer = {
            let buffer_key = BufferKey {
                size: size_of::<LightUniform>() as u64,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            };

            context.get_buffer_by_id(&standard_resources::light_uniform_buffer(), buffer_key)
        };

        let cascade_view = shadow_map
            .texture
            .create_view(&wgpu::TextureViewDescriptor {
                label: Some("shadow cascade view"),
                format: Some(Texture::DEPTH_FORMAT),
                dimension: Some(wgpu::TextureViewDimension::D2Array),
                usage: Some(wgpu::TextureUsages::TEXTURE_BINDING),
                aspect: wgpu::TextureAspect::DepthOnly,
                array_layer_count: Some(crate::render::light::NUM_CASCADES as u32),
                ..Default::default()
            });

        let point_shadow_map = {
            let point_sm_key = TextureKey {
                width: 512,
                height: 512,
                format: Texture::DEPTH_FORMAT,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                layers: (MAX_POINT_LIGHTS * 6) as u32,
            };

            context.get_texture_by_id(&standard_resources::point_shadow_map(), point_sm_key)
        };

        let point_shadow_map_view =
            point_shadow_map
                .texture
                .create_view(&wgpu::TextureViewDescriptor {
                    label: Some("point shadow map view"),
                    format: Some(Texture::DEPTH_FORMAT),
                    dimension: Some(wgpu::TextureViewDimension::CubeArray),
                    aspect: wgpu::TextureAspect::DepthOnly,
                    array_layer_count: Some(MAX_POINT_LIGHTS as u32 * 6),
                    ..Default::default()
                });

        let sky_view = if let Some(id) = context.render_world.sky_imported_resources.texture {
            context
                .render_world
                .imported_texture_cache
                .get(id)
                .unwrap()
                .view
                .clone()
        } else {
            context
                .render_world
                .mesh_render_resources
                .dummy_cube_view
                .clone()
        };

        let light_bind_group = context.create_bind_group(
            &light_bind_group_layout,
            vec![
                ssao_blur.id,
                light_uniform_buffer.id,
                cascade_uniform_buffer.id,
                shadow_map.id,
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
                            resource: wgpu::BindingResource::TextureView(&cascade_view),
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
                            resource: wgpu::BindingResource::TextureView(&point_shadow_map_view),
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

        let camera_buffer = {
            let buffer_key = context
                .render_world
                .extracted
                .cameras
                .get_buffer_key()
                .clone();

            context
                .get_buffer_by_id(&standard_resources::camera_buffer(), buffer_key)
                .clone()
        };
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


        let world = &mut *context.render_world;

        let mut render_pass = context
            .encoder
            .begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("transparent mesh render pass"),
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
            if world.extracted.cameras.types[camera_idx] == CameraType::D3 {
                render_transparent_meshes(
                    &world.extracted.transparent_meshes,
                    &world.mesh_cache,
                    &world.mesh_render_resources,
                    &camera_bind_group,
                    &light_bind_group,
                    world
                        .mesh_render_resources
                        .bindless_bind_group
                        .as_ref()
                        .unwrap(),
                    camera_idx,
                    &world.extracted.cameras.uniforms[camera_idx],
                    &mut render_pass,
                    self.pipeline.as_ref().unwrap(),
                    &context.render_context.device,
                    &context.render_context.queue,
                    &mut self.instance_buffer,
                );
            }
        }
    }
}

fn render_transparent_meshes<'a, 'b: 'a>(
    extracted_meshes: &'b Vec<ExtractedMesh>,
    mesh_cache: &'b MeshCache,
    mesh_render_resources: &'b MeshRenderResources,
    camera_bind_group: &'b wgpu::BindGroup,
    light_bind_group: &'b wgpu::BindGroup,
    bindless_bind_group: &'b wgpu::BindGroup,
    camera_index: usize,
    camera_uniform: &CameraUniform,
    render_pass: &mut wgpu::RenderPass<'a>,
    pipeline: &'b wgpu::RenderPipeline,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    instance_buffer: &mut Option<wgpu::Buffer>,
) {
    let view_proj = Mat4::from_cols_array_2d(&camera_uniform.view_proj);

    let mut sorted_meshes: Vec<_> = extracted_meshes.iter().enumerate().collect();
    sorted_meshes.sort_by(|(_, a), (_, b)| {
        let a_center = a.transform.position;
        let b_center = b.transform.position;
        let a_dist = (view_proj * Vec3::new(a_center.x, a_center.y, a_center.z).extend(1.0)).z;
        let b_dist = (view_proj * Vec3::new(b_center.x, b_center.y, b_center.z).extend(1.0)).z;
        b_dist
            .partial_cmp(&a_dist)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let mut sorted_instances: Vec<InstanceRaw> = Vec::new();
    let mut sorted_mesh_info: Vec<(MeshId, u32, u32)> = Vec::new();

    for (_, mesh) in sorted_meshes {
        if let Some(m) = mesh_cache.get(mesh.mesh_id) {
            let material_idx = mesh
                .material_id
                .and_then(|id| mesh_render_resources.material_index_map.get(&id))
                .cloned()
                .unwrap_or(0);

            let instance_raw = crate::render::mesh::Instance {
                position: mesh.transform.position,
                scale: mesh.transform.scale,
                rotation: mesh.transform.rotation,
                material_idx,
            }
            .to_raw();

            sorted_instances.push(instance_raw);
            sorted_mesh_info.push((mesh.mesh_id, m.index_offset, m.index_count));
        }
    }

    if sorted_instances.is_empty() {
        return;
    }

    let buffer_size = (sorted_instances.len() * size_of::<InstanceRaw>()) as wgpu::BufferAddress;

    if instance_buffer.is_none() || instance_buffer.as_ref().unwrap().size() < buffer_size {
        *instance_buffer = Some(device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("transparent instance buffer"),
            size: buffer_size,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }));
    }

    queue.write_buffer(
        instance_buffer.as_ref().unwrap(),
        0,
        bytemuck::cast_slice(&sorted_instances),
    );

    render_pass.set_pipeline(pipeline);

    render_pass.set_bind_group(
        0,
        camera_bind_group,
        &[camera_index as u32 * CameraUniform::get_uniform_offset_unit()],
    );
    render_pass.set_bind_group(1, light_bind_group, &[]);
    render_pass.set_bind_group(2, bindless_bind_group, &[]);
    render_pass.set_vertex_buffer(
        0,
        mesh_render_resources.mesh_allocator.vertex_buffer.slice(..),
    );
    render_pass.set_index_buffer(
        mesh_render_resources.mesh_allocator.index_buffer.slice(..),
        wgpu::IndexFormat::Uint32,
    );
    render_pass.set_vertex_buffer(1, instance_buffer.as_ref().unwrap().slice(..));

    let mut base_instance = 0u32;
    for (_, index_offset, index_count) in sorted_mesh_info {
        render_pass.draw_indexed(
            index_offset..index_offset + index_count,
            0,
            base_instance..base_instance + 1,
        );
        base_instance += 1;
    }
}
