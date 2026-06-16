use crate::render::camera::{CameraType, CameraUniform};
use crate::render::light::{CascadeUniform, LightUniform, MAX_SHADOWED_POINT_LIGHTS};
use crate::render::render_backend::PreparedFrame;
use crate::render::render_graph::{standard_resources, SamplerKey};
use crate::render::render_graph::{FrameContext, Node, TextureKey};
use crate::render::vertex::{Vertex3d, VertexBuffer};
use crate::render::{create_render_pipeline, InstanceRaw, Texture};
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

    fn node_resources(
        &self,
        prepared: &PreparedFrame,
    ) -> crate::render::render_graph::resource::NodeResources {
        use crate::render::render_graph::resource::{ResourceSpec, TextureKey};
        use crate::render::render_graph::standard_resources;
        use crate::render::Texture;

        let camera_buffer_size = CameraUniform::get_uniform_offset_unit()
            * crate::render::render_graph::nodes::prepare_view::MAX_CAMERAS;

        let material_buffer_size = prepared.material_uniforms.len()
            * size_of::<crate::render::material::MaterialUniform>();

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
            .input(
                standard_resources::cull_visible_instance_buffer(),
                ResourceSpec::buffer(
                    prepared.instance_buffer_size as u64,
                    wgpu::BufferUsages::STORAGE
                        | wgpu::BufferUsages::VERTEX
                        | wgpu::BufferUsages::COPY_DST,
                ),
            )
            .input(
                standard_resources::cull_indirect_buffer(),
                ResourceSpec::buffer(
                    prepared.indirect_buffer_size as u64,
                    wgpu::BufferUsages::STORAGE
                        | wgpu::BufferUsages::INDIRECT
                        | wgpu::BufferUsages::COPY_DST,
                ),
            )
            .input(
                standard_resources::material_storage_buffer(),
                ResourceSpec::buffer(
                    material_buffer_size as u64,
                    wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
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
                    layers: (MAX_SHADOWED_POINT_LIGHTS * 6) as u32,
                    dimension: wgpu::TextureDimension::D2,
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
                    dimension: wgpu::TextureDimension::D2,
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
                    dimension: wgpu::TextureDimension::D2,
                }),
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
                    dimension: wgpu::TextureDimension::D2,
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
                    dimension: wgpu::TextureDimension::D2,
                }),
            )
            .input(
                standard_resources::light_uniform_buffer(),
                ResourceSpec::buffer(
                    size_of::<LightUniform>() as u64,
                    wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                ),
            )
            .input(
                standard_resources::point_light_storage_buffer(),
                ResourceSpec::buffer(
                    (size_of::<crate::render::light::PointLightUniform>() * 1024) as u64,
                    wgpu::BufferUsages::STORAGE,
                ),
            )
            .input(
                standard_resources::light_grid_buffer(),
                ResourceSpec::buffer(0, wgpu::BufferUsages::STORAGE),
            )
            .input(
                standard_resources::light_index_list_buffer(),
                ResourceSpec::buffer(0, wgpu::BufferUsages::STORAGE),
            )
            .input(
                standard_resources::cluster_config_buffer(),
                ResourceSpec::buffer(0, wgpu::BufferUsages::UNIFORM),
            )
            .input(
                standard_resources::volumetric_lighting_texture(),
                ResourceSpec::Texture(TextureKey {
                    width: crate::render::light::CLUSTER_GRID_SIZE[0],
                    height: crate::render::light::CLUSTER_GRID_SIZE[1],
                    layers: crate::render::light::CLUSTER_GRID_SIZE[2],
                    format: Some(wgpu::TextureFormat::Rgba16Float),
                    usage: wgpu::TextureUsages::TEXTURE_BINDING,
                    dimension: wgpu::TextureDimension::D3,
                }),
            )
    }

    fn run(&mut self, context: &mut FrameContext) {
        if context.prepared.instance_buffer_size == 0 {
            return;
        }

        let device = &context.render_context.device;
        let extracted = context.extracted;

        let camera_bind_group_layout = context
            .backend
            .get_bind_group_layout("camera_bind_group_layout")
            .unwrap()
            .clone();

        let light_bind_group_layout = context
            .backend
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
                        // All Point Lights
                        wgpu::BindGroupLayoutEntry {
                            binding: 8,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: true },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        // Light Grid
                        wgpu::BindGroupLayoutEntry {
                            binding: 9,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: true },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        // Light Index List
                        wgpu::BindGroupLayoutEntry {
                            binding: 10,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: true },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        // Cluster Config
                        wgpu::BindGroupLayoutEntry {
                            binding: 11,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        // Volumetric Texture
                        wgpu::BindGroupLayoutEntry {
                            binding: 12,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Texture {
                                multisampled: false,
                                view_dimension: wgpu::TextureViewDimension::D3,
                                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            },
                            count: None,
                        },
                    ],
                    label: Some("mesh light bind group layout"),
                });

            context
                .backend
                .add_bind_group_layout("light_bind_group_layout", light_bind_group_layout);
        }

        let light_bind_group_layout = context
            .backend
            .get_bind_group_layout("light_bind_group_layout")
            .unwrap()
            .clone();

        if self.pipeline.is_none() {
            let bindless_bind_group_layout = context
                .backend
                .get_bind_group_layout("bindless_bind_group_layout")
                .unwrap();

            let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Mesh Layout"),
                bind_group_layouts: &[
                    &camera_bind_group_layout,
                    &light_bind_group_layout,
                    &bindless_bind_group_layout,
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

        // --- 全局灯光 Uniform 的更新已移动到 LightCullingNode ---
        let light_uniform_buffer = context.buffer(&standard_resources::light_uniform_buffer());

        if context.prepared.draw_counts.is_empty() {
            return;
        }

        let main_color = FrameContext::texture(context, &standard_resources::main_color());
        let main_depth = context.texture(&standard_resources::main_depth());

        let ssao_blur = context.texture(&standard_resources::ssao_blur());
        let directional_shadow_map = context.texture(&standard_resources::directional_shadow_map());
        let point_shadow_map = context.texture(&standard_resources::point_shadow_map());

        let point_shadow_map_view = {
            point_shadow_map.get_view(&wgpu::TextureViewDescriptor {
                label: Some("point shadow map view"),
                format: Some(Texture::DEPTH_FORMAT),
                dimension: Some(wgpu::TextureViewDimension::CubeArray),
                aspect: wgpu::TextureAspect::DepthOnly,
                array_layer_count: Some(MAX_SHADOWED_POINT_LIGHTS as u32 * 6),
                ..Default::default()
            })
        };

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

        if extracted.meshes.is_empty() {
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

        let (sky_view, sky_view_id) =
            if let Some(id) = context.backend.sky_imported_resources.texture {
                let texture_cache = context.backend.imported_texture_cache.read().unwrap();
                let tex = texture_cache.get(id).unwrap();
                (tex.view.clone(), tex.view_id)
            } else {
                (context.backend.dummy_cube_view.clone(), 0)
            };

        let point_light_storage_buffer = context.buffer(&standard_resources::point_light_storage_buffer());
        let light_grid_buffer = context.buffer(&standard_resources::light_grid_buffer());
        let light_index_list_buffer = context.buffer(&standard_resources::light_index_list_buffer());
        let cluster_config_buffer = context.buffer(&standard_resources::cluster_config_buffer());
        let volumetric_tex = context.texture(&standard_resources::volumetric_lighting_texture());

        let volumetric_view = volumetric_tex.get_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::D3),
            ..Default::default()
        });

        let light_bind_group = context.create_bind_group(
            "light_bind_group_layout",
            vec![
                light_uniform_buffer.id,
                ssao_blur.view_id,
                cascade_view.1,
                cascade_uniform_buffer.id,
                point_shadow_map_view.1,
                sky_view_id,
                point_light_storage_buffer.id,
                light_grid_buffer.id,
                light_index_list_buffer.id,
                cluster_config_buffer.id,
                volumetric_view.1,
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
                        wgpu::BindGroupEntry {
                            binding: 8,
                            resource: point_light_storage_buffer.buffer.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 9,
                            resource: light_grid_buffer.buffer.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 10,
                            resource: light_index_list_buffer.buffer.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 11,
                            resource: cluster_config_buffer.buffer.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 12,
                            resource: wgpu::BindingResource::TextureView(&volumetric_view.0),
                        },
                    ],
                    label: Some("light bind group (dynamic)"),
                })
            },
        );

        let global_visible_instance_buffer =
            context.buffer(&standard_resources::cull_visible_instance_buffer());
        let global_indirect_buffer = context.buffer(&standard_resources::cull_indirect_buffer());
        let materials_storage_buffer =
            context.buffer(&standard_resources::material_storage_buffer());
        let bindless_bind_group = context
            .get_bind_group(
                "bindless_bind_group_layout",
                vec![materials_storage_buffer.id],
            )
            .clone();

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

        let (v_buf, i_buf) = {
            let allocator = context.backend.imported_mesh_allocator.read().unwrap();
            (
                allocator.vertex_buffer.clone(),
                allocator.index_buffer.clone(),
            )
        };

        for camera_idx in 0..extracted.cameras.uniforms.len() {
            let camera_offset = camera_idx as u32 * CameraUniform::get_uniform_offset_unit();

            if extracted.cameras.types[camera_idx] == CameraType::D3 {
                render_pass.set_pipeline(self.pipeline.as_ref().unwrap());

                render_pass.set_bind_group(0, &camera_bind_group, &[camera_offset]);
                render_pass.set_bind_group(1, &light_bind_group, &[]);
                render_pass.set_bind_group(2, &bindless_bind_group, &[]);

                render_pass.set_vertex_buffer(0, v_buf.slice(..));
                render_pass.set_vertex_buffer(1, global_visible_instance_buffer.buffer.slice(..));
                render_pass.set_index_buffer(i_buf.slice(..), wgpu::IndexFormat::Uint32);

                if !context.prepared.draw_counts.is_empty() && context.prepared.draw_counts[0] > 0 {
                    render_pass.multi_draw_indexed_indirect(
                        &global_indirect_buffer.buffer,
                        0,
                        context.prepared.draw_counts[0],
                    );
                }
            }
        }
    }
}
