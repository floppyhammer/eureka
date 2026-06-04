use crate::math::aabb::Aabb;
use crate::math::frustum::Frustum;
use crate::math::transform::Transform3d;
use crate::render::camera::{CameraRenderResources, CameraUniform};
use crate::render::gizmo::GizmoRenderResources;
use crate::render::light::{ExtractedLights, LightRenderResources, LightUniform, MAX_POINT_LIGHTS};
use crate::render::material::{MaterialCache, MaterialId, MaterialStandard};
use crate::render::shader_maker::ShaderMaker;
use crate::render::vertex::{Vertex2d, Vertex3d, VertexBuffer, VertexSky};
use crate::render::{create_render_pipeline, RenderServer, Texture, TextureCache, TextureId};
use crate::scene::Bvh;
use glam::{Mat3, Mat4, Quat, Vec3};
use std::collections::HashMap;
use std::mem;
use std::ops::Range;
use wgpu::util::DeviceExt;
use wgpu::{BufferAddress, SamplerBindingType};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct MeshId(uuid::Uuid);

pub struct MeshCache {
    pub(crate) storage: HashMap<MeshId, Mesh>,
}

impl MeshCache {
    pub(crate) fn new() -> Self {
        Self {
            storage: HashMap::new(),
        }
    }

    pub(crate) fn add(&mut self, mesh: Mesh) -> MeshId {
        let id = MeshId(uuid::Uuid::new_v4());
        self.storage.insert(id, mesh);
        id
    }

    pub(crate) fn get(&self, mesh_id: MeshId) -> Option<&Mesh> {
        self.storage.get(&mesh_id)
    }

    pub(crate) fn get_mut(&mut self, mesh_id: MeshId) -> Option<&mut Mesh> {
        self.storage.get_mut(&mesh_id)
    }

    pub(crate) fn remove(&mut self, mesh_id: MeshId) {
        self.storage.remove(&mesh_id);
    }
}

pub struct Mesh {
    pub name: String,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub index_count: u32,
    pub aabb: Aabb,
}

impl Mesh {
    pub fn default_2d(device: &wgpu::Device) -> Mesh {
        let vertices = [
            Vertex2d {
                position: [0.0, 0.0],
                uv: [0.0, 0.0],
                color: [1.0, 1.0, 1.0],
            },
            Vertex2d {
                position: [0.0, -1.0],
                uv: [0.0, 1.0],
                color: [1.0, 1.0, 1.0],
            },
            Vertex2d {
                position: [1.0, -1.0],
                uv: [1.0, 1.0],
                color: [1.0, 1.0, 1.0],
            },
            Vertex2d {
                position: [1.0, 0.0],
                uv: [1.0, 0.0],
                color: [1.0, 1.0, 1.0],
            },
        ];

        let indices = [0, 1, 2, 2, 3, 0];

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("default 2d mesh's vertex buffer")),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("default 2d mesh's index buffer")),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        let aabb = Aabb::from_points(
            &vertices
                .iter()
                .map(|v| Vec3::new(v.position[0], v.position[1], 0.0))
                .collect::<Vec<_>>(),
        );

        Self {
            name: "default 2d mesh".to_string(),
            vertex_buffer,
            index_buffer,
            index_count: indices.len() as u32,
            aabb,
        }
    }

    pub fn default_3d(device: &wgpu::Device) -> Mesh {
        let vertices = [
            Vertex3d {
                position: [0.0, 0.0, 0.0],
                uv: [0.0, 1.0],
                normal: [0.0, 0.0, 0.0],
                tangent: [0.0, 0.0, 0.0],
                bi_tangent: [0.0, 0.0, 0.0],
            },
            Vertex3d {
                position: [1.0, 0.0, 0.0],
                uv: [1.0, 1.0],
                normal: [0.0, 0.0, 0.0],
                tangent: [0.0, 0.0, 0.0],
                bi_tangent: [0.0, 0.0, 0.0],
            },
            Vertex3d {
                position: [1.0, 1.0, 0.0],
                uv: [1.0, 0.0],
                normal: [0.0, 0.0, 0.0],
                tangent: [0.0, 0.0, 0.0],
                bi_tangent: [0.0, 0.0, 0.0],
            },
            Vertex3d {
                position: [0.0, 1.0, 0.0],
                uv: [0.0, 0.0],
                normal: [0.0, 0.0, 0.0],
                tangent: [0.0, 0.0, 0.0],
                bi_tangent: [0.0, 0.0, 0.0],
            },
        ];

        let indices = [0, 1, 2, 2, 3, 0];

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("default 3d mesh's vertex buffer")),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("default 3d mesh's index buffer")),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        let aabb = Aabb::from_points(
            &vertices
                .iter()
                .map(|v| Vec3::from_slice(&v.position))
                .collect::<Vec<_>>(),
        );

        Self {
            name: "default 3d mesh".to_string(),
            vertex_buffer,
            index_buffer,
            index_count: indices.len() as u32,
            aabb,
        }
    }

    pub fn default_skybox(device: &wgpu::Device) -> Mesh {
        let vertices = [
            VertexSky {
                position: [-1.0, -1.0, -1.0],
            },
            VertexSky {
                position: [1.0, -1.0, -1.0],
            },
            VertexSky {
                position: [1.0, 1.0, -1.0],
            },
            VertexSky {
                position: [-1.0, 1.0, -1.0],
            },
            VertexSky {
                position: [-1.0, -1.0, 1.0],
            },
            VertexSky {
                position: [1.0, -1.0, 1.0],
            },
            VertexSky {
                position: [1.0, 1.0, 1.0],
            },
            VertexSky {
                position: [-1.0, 1.0, 1.0],
            },
        ];

        let indices = [
            0, 1, 2, 2, 3, 0, 4, 6, 5, 6, 4, 7, 2, 6, 7, 2, 7, 3, 1, 5, 6, 1, 6, 2, 3, 7, 0, 4, 0,
            7, 5, 1, 4, 4, 1, 0,
        ];

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("default skybox mesh's vertex buffer")),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("default skybox mesh's index buffer")),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        let aabb = Aabb::from_points(
            &vertices
                .iter()
                .map(|v| Vec3::from_slice(&v.position))
                .collect::<Vec<_>>(),
        );

        Self {
            name: "default skybox mesh".to_string(),
            vertex_buffer,
            index_buffer,
            index_count: indices.len() as u32,
            aabb,
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct ExtractedMesh {
    pub(crate) transform: Transform3d,
    pub(crate) mesh_id: MeshId,
    pub(crate) material_id: Option<MaterialId>,
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct InstanceRaw {
    model: [[f32; 4]; 4],
    normal: [[f32; 4]; 3], // Each row of 3x3 matrix padded to 4 floats
    material_idx: u32,
    _pad: [u32; 3],
}

pub(crate) struct Instance {
    pub(crate) position: Vec3,
    pub(crate) scale: Vec3,
    pub(crate) rotation: Quat,
    pub(crate) material_idx: u32,
}

impl Instance {
    pub(crate) fn to_raw(&self) -> InstanceRaw {
        let model = Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.position);
        let normal = Mat3::from_cols(
            self.rotation * Vec3::new(1.0 / self.scale.x, 0.0, 0.0),
            self.rotation * Vec3::new(0.0, 1.0 / self.scale.y, 0.0),
            self.rotation * Vec3::new(0.0, 0.0, 1.0 / self.scale.z),
        );

        let n = normal.to_cols_array_2d();
        InstanceRaw {
            model: model.to_cols_array_2d(),
            normal: [
                [n[0][0], n[0][1], n[0][2], 0.0],
                [n[1][0], n[1][1], n[1][2], 0.0],
                [n[2][0], n[2][1], n[2][2], 0.0],
            ],
            material_idx: self.material_idx,
            _pad: [0; 3],
        }
    }
}

impl InstanceRaw {
    pub(crate) fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<InstanceRaw>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 5,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 6,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 8]>() as wgpu::BufferAddress,
                    shader_location: 7,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 12]>() as wgpu::BufferAddress,
                    shader_location: 8,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 16]>() as wgpu::BufferAddress,
                    shader_location: 9,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 20]>() as wgpu::BufferAddress,
                    shader_location: 10,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 24]>() as wgpu::BufferAddress,
                    shader_location: 11,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 28]>() as wgpu::BufferAddress,
                    shader_location: 12,
                    format: wgpu::VertexFormat::Uint32,
                },
            ],
        }
    }
}

pub struct MeshRenderResources {
    pub(crate) light_bind_group_layout: wgpu::BindGroupLayout,
    pub(crate) light_bind_group: Option<wgpu::BindGroup>,
    pub(crate) light_uniform_buffer: Option<wgpu::Buffer>,

    pub(crate) dummy_2d_view: wgpu::TextureView,
    pub(crate) dummy_cube_view: wgpu::TextureView,
    pub(crate) dummy_sampler: wgpu::Sampler,
    pub(crate) current_skybox: Option<TextureId>,

    // Bindless resources
    pub(crate) bindless_bind_group_layout: wgpu::BindGroupLayout,
    pub(crate) bindless_bind_group: Option<wgpu::BindGroup>,
    pub(crate) materials_storage_buffer: Option<wgpu::Buffer>,
    pub(crate) texture_index_map: HashMap<TextureId, u32>,
    pub(crate) material_index_map: HashMap<MaterialId, u32>,

    // GPU Culling resources
    pub(crate) cull_pipeline: Option<wgpu::ComputePipeline>,
    pub(crate) cull_bind_group_layout: wgpu::BindGroupLayout,

    pub(crate) pipeline_cache: HashMap<u32, wgpu::RenderPipeline>,
    pub material_cache: MaterialCache,

    pub(crate) instance_cache: HashMap<MeshId, InstanceMetadata>,
    pub(crate) instance_offsets: HashMap<usize, u32>,
}

pub(crate) struct InstanceMetadata {
    pub(crate) buffer: wgpu::Buffer,                  // Source instances (Storage)
    pub(crate) visible_buffer: wgpu::Buffer,          // Culled instances (Vertex/Storage)
    pub(crate) indirect_buffer: wgpu::Buffer,         // Indirect draw command
    pub(crate) mesh_aabb_buffer: wgpu::Buffer,        // Mesh AABB for culling
    pub(crate) cull_bind_group: Option<wgpu::BindGroup>,
    pub(crate) instance_count: u64,
}

impl MeshRenderResources {
    pub(crate) fn new(render_server: &RenderServer) -> Self {
        let light_bind_group_layout =
            render_server
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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

        let bindless_bind_group_layout =
            render_server
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: true },
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
                                view_dimension: wgpu::TextureViewDimension::D2,
                                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            },
                            count: std::num::NonZeroU32::new(1024),
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 2,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                            count: None,
                        },
                    ],
                    label: Some("bindless bind group layout"),
                });

        let dummy_2d_view = {
            let size = wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            };
            let texture = render_server.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("dummy 2d texture"),
                size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });
            texture.create_view(&wgpu::TextureViewDescriptor {
                label: Some("dummy 2d view"),
                dimension: Some(wgpu::TextureViewDimension::D2),
                ..Default::default()
            })
        };

        let dummy_cube_view = {
            let size = wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 6,
            };
            let texture = render_server.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("dummy cube texture"),
                size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });
            texture.create_view(&wgpu::TextureViewDescriptor {
                label: Some("dummy cube view"),
                dimension: Some(wgpu::TextureViewDimension::Cube),
                ..Default::default()
            })
        };

        let dummy_sampler = render_server.device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("dummy sampler"),
            ..Default::default()
        });

        let cull_bind_group_layout =
            render_server
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    entries: &[
                        // Camera
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: true,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        // Mesh AABB
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        // All Instances
                        wgpu::BindGroupLayoutEntry {
                            binding: 2,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: true },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        // Visible Instances
                        wgpu::BindGroupLayoutEntry {
                            binding: 3,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: false },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        // Indirect Buffer
                        wgpu::BindGroupLayoutEntry {
                            binding: 4,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: false },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                    ],
                    label: Some("cull bind group layout"),
                });

        Self {
            light_bind_group_layout,
            light_uniform_buffer: None,
            dummy_2d_view,
            dummy_cube_view,
            dummy_sampler,
            current_skybox: None,

            bindless_bind_group_layout,
            bindless_bind_group: None,
            materials_storage_buffer: None,
            texture_index_map: HashMap::new(),
            material_index_map: HashMap::new(),

            cull_pipeline: None,
            cull_bind_group_layout,

            light_bind_group: None,

            pipeline_cache: Default::default(),
            material_cache: MaterialCache::new(),
            instance_cache: HashMap::new(),
            instance_offsets: HashMap::new(),
        }
    }

    pub fn prepare_materials(
        &mut self,
        texture_cache: &TextureCache,
        render_server: &RenderServer,
    ) {
        self.texture_index_map.clear();
        let mut texture_views = Vec::new();

        let mut sorted_materials: Vec<_> = self.material_cache.storage.iter().collect();
        // Use name for stable sorting if Uuid is not directly accessible
        sorted_materials.sort_by(|(id1, m1), (id2, m2)| m1.name.cmp(&m2.name).then(id1.0.cmp(&id2.0)));

        for (_, material) in &sorted_materials {
            let tex_ids = [
                material.color_texture,
                material.normal_texture,
                material.metallic_roughness_texture,
            ];

            for id in tex_ids.into_iter().flatten() {
                if !self.texture_index_map.contains_key(&id) {
                    if let Some(texture) = texture_cache.get(id) {
                        self.texture_index_map
                            .insert(id, texture_views.len() as u32);
                        texture_views.push(&texture.view);
                    }
                }
            }
        }

        let placeholder_view = if !texture_views.is_empty() {
            texture_views[0]
        } else {
            &self.dummy_2d_view
        };

        let mut final_views = vec![placeholder_view; 1024];
        for (i, view) in texture_views.iter().enumerate() {
            final_views[i] = view;
        }

        self.material_index_map.clear();
        let mut material_uniforms = Vec::new();

        for (id, material) in &sorted_materials {
            self.material_index_map
                .insert(**id, material_uniforms.len() as u32);
            material_uniforms.push(material.to_uniform(&self.texture_index_map));
        }

        if material_uniforms.is_empty() {
            material_uniforms.push(MaterialStandard::new("dummy").to_uniform(&HashMap::new()));
        }

        let buffer_size = (material_uniforms.len() * mem::size_of::<crate::render::material::MaterialUniform>()) as u64;

        if self.materials_storage_buffer.is_none() || self.materials_storage_buffer.as_ref().unwrap().size() < buffer_size {
            self.materials_storage_buffer = Some(render_server.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("materials storage buffer"),
                size: buffer_size,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }));
        }

        render_server.queue.write_buffer(
            self.materials_storage_buffer.as_ref().unwrap(),
            0,
            bytemuck::cast_slice(&material_uniforms),
        );

        self.bindless_bind_group = Some(render_server.device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &self.bindless_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.materials_storage_buffer.as_ref().unwrap().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureViewArray(&final_views),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&self.dummy_sampler),
                },
            ],
            label: Some("bindless bind group"),
        }));
    }

    pub fn prepare_pipeline(
        &mut self,
        render_server: &RenderServer,
        shader_maker: &mut ShaderMaker,
        camera_bind_group_layout: &wgpu::BindGroupLayout,
    ) {
        if self.cull_pipeline.is_none() {
            let pipeline_layout = render_server.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("cull pipeline layout"),
                bind_group_layouts: &[&self.cull_bind_group_layout],
                push_constant_ranges: &[],
            });

            let shader = render_server.device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("cull shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/cull.wgsl").into()),
            });

            self.cull_pipeline = Some(render_server.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("cull pipeline"),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: Some("main"),
                cache: None,
                compilation_options: Default::default(),
            }));
        }

        const PIPELINE_KEY: u32 = 0;

        if !self.pipeline_cache.contains_key(&PIPELINE_KEY) {
            let pipeline_layout = render_server.device.create_pipeline_layout(
                &wgpu::PipelineLayoutDescriptor {
                    label: Some("mesh bindless pipeline layout"),
                    bind_group_layouts: &[
                        camera_bind_group_layout,
                        &self.light_bind_group_layout,
                        &self.bindless_bind_group_layout,
                    ],
                    push_constant_ranges: &[],
                },
            );

            let shader = wgpu::ShaderModuleDescriptor {
                label: Some("standard bindless shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/mesh.wgsl").into()),
            };

            let pipeline = create_render_pipeline(
                &render_server.device,
                &pipeline_layout,
                Some(render_server.surface_config.format),
                Some(Texture::DEPTH_FORMAT),
                &[Vertex3d::desc(), InstanceRaw::desc()],
                shader,
                "standard bindless pipeline",
                false,
                Some(wgpu::Face::Back),
            );

            self.pipeline_cache.insert(PIPELINE_KEY, pipeline);
        }
    }

    pub fn get_pipeline(&self) -> &wgpu::RenderPipeline {
        self.pipeline_cache.get(&0).unwrap()
    }

    pub(crate) fn prepare_instances(
        &mut self,
        render_server: &RenderServer,
        meshes: &Vec<ExtractedMesh>,
        mesh_cache: &MeshCache,
        camera_uniform_buffer: &wgpu::Buffer,
    ) {
        let mut grouped_instances: HashMap<MeshId, Vec<InstanceRaw>> = HashMap::new();
        self.instance_offsets.clear();

        for (i, mesh) in meshes.iter().enumerate() {
            let instances = grouped_instances.entry(mesh.mesh_id).or_default();
            self.instance_offsets.insert(i, instances.len() as u32);

            let material_idx = mesh.material_id
                .and_then(|id| self.material_index_map.get(&id))
                .cloned()
                .unwrap_or(0); // Use 0 for default

            let instance = Instance {
                position: mesh.transform.position,
                scale: mesh.transform.scale,
                rotation: mesh.transform.rotation,
                material_idx,
            };
            instances.push(instance.to_raw());
        }

        for (mesh_id, instance_data) in grouped_instances {
            let mesh = mesh_cache.get(mesh_id).unwrap();
            let buffer_size = (instance_data.len() * mem::size_of::<InstanceRaw>()) as BufferAddress;

            let metadata = self.instance_cache.entry(mesh_id).or_insert_with(|| {
                let buffer = render_server.device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("model instance buffer"),
                    size: buffer_size,
                    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                });

                let visible_buffer = render_server.device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("visible instance buffer"),
                    size: buffer_size,
                    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                });

                let indirect_buffer = render_server.device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("indirect buffer"),
                    size: 20,
                    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::INDIRECT | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                });

                let mesh_aabb_buffer = render_server.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("mesh aabb buffer"),
                    contents: bytemuck::cast_slice(&[mesh.aabb.min.extend(0.0).to_array(), mesh.aabb.max.extend(0.0).to_array()]),
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                });

                InstanceMetadata {
                    buffer,
                    visible_buffer,
                    indirect_buffer,
                    mesh_aabb_buffer,
                    cull_bind_group: None,
                    instance_count: instance_data.len() as u64,
                }
            });

            if (metadata.instance_count as usize) < instance_data.len() {
                metadata.buffer = render_server.device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("model instance buffer"),
                    size: buffer_size,
                    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                });
                metadata.visible_buffer = render_server.device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("visible instance buffer"),
                    size: buffer_size,
                    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                });
                metadata.cull_bind_group = None;
            }
            metadata.instance_count = instance_data.len() as u64;

            render_server.queue.write_buffer(
                &metadata.buffer,
                0,
                bytemuck::cast_slice(&instance_data),
            );

            let indirect_data = [
                mesh.index_count,
                0,
                0,
                0,
                0,
            ];
            render_server.queue.write_buffer(&metadata.indirect_buffer, 0, bytemuck::cast_slice(&indirect_data));

            if metadata.cull_bind_group.is_none() {
                metadata.cull_bind_group = Some(render_server.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    layout: &self.cull_bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                                buffer: camera_uniform_buffer,
                                offset: 0,
                                size: Some(wgpu::BufferSize::new(mem::size_of::<CameraUniform>() as u64).unwrap()),
                            }),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: metadata.mesh_aabb_buffer.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: metadata.buffer.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 3,
                            resource: metadata.visible_buffer.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 4,
                            resource: metadata.indirect_buffer.as_entire_binding(),
                        },
                    ],
                    label: Some("cull bind group"),
                }));
            }
        }
    }

    pub(crate) fn prepare_lights(
        &mut self,
        render_server: &RenderServer,
        lights: &ExtractedLights,
        light_render_resources: &LightRenderResources,
        texture_cache: &TextureCache,
        ssao_texture_id: TextureId,
        skybox_texture_id: Option<TextureId>,
    ) {
        let light_uniform_size = mem::size_of::<LightUniform>();

        if self.light_uniform_buffer.is_none() {
            self.light_uniform_buffer =
                Some(render_server.device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("light uniform buffer"),
                    size: light_uniform_size as BufferAddress,
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                }));
        }

        let needs_recreate = self.light_bind_group.is_none() || self.current_skybox != skybox_texture_id;

        if needs_recreate
            && light_render_resources.directional_shadow_map.is_some()
            && light_render_resources.cascade_uniform_buffer.is_some()
            && light_render_resources.point_shadow_map.is_some()
        {
            self.current_skybox = skybox_texture_id;

            let shadow_map = texture_cache
                .get(light_render_resources.directional_shadow_map.unwrap())
                .unwrap();

            let point_shadow_map = texture_cache
                .get(light_render_resources.point_shadow_map.unwrap())
                .unwrap();

            let shadow_sampler = render_server
                .device
                .create_sampler(&wgpu::SamplerDescriptor {
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

            let point_shadow_view =
                point_shadow_map
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor {
                        label: Some("point shadow cube array view"),
                        format: Some(Texture::DEPTH_FORMAT),
                        dimension: Some(wgpu::TextureViewDimension::CubeArray),
                        usage: Some(wgpu::TextureUsages::TEXTURE_BINDING),
                        aspect: wgpu::TextureAspect::DepthOnly,
                        base_mip_level: 0,
                        mip_level_count: None,
                        base_array_layer: 0,
                        array_layer_count: Some(MAX_POINT_LIGHTS as u32 * 6),
                    });

            let ssao_texture = texture_cache.get(ssao_texture_id).unwrap();

            let (skybox_view, skybox_sampler) = if let Some(id) = skybox_texture_id {
                let sky_tex = texture_cache.get(id).unwrap();
                (&sky_tex.view, &sky_tex.sampler)
            } else {
                (&self.dummy_cube_view, &self.dummy_sampler)
            };

            let bind_group = render_server
                .device
                .create_bind_group(&wgpu::BindGroupDescriptor {
                    layout: &self.light_bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: self
                                .light_uniform_buffer
                                .as_ref()
                                .unwrap()
                                .as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::TextureView(&shadow_map.view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: wgpu::BindingResource::Sampler(&shadow_sampler),
                        },
                        wgpu::BindGroupEntry {
                            binding: 3,
                            resource: light_render_resources
                                .cascade_uniform_buffer
                                .as_ref()
                                .unwrap()
                                .as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 4,
                            resource: wgpu::BindingResource::TextureView(&point_shadow_view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 5,
                            resource: wgpu::BindingResource::TextureView(&ssao_texture.view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 6,
                            resource: wgpu::BindingResource::TextureView(skybox_view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 7,
                            resource: wgpu::BindingResource::Sampler(skybox_sampler),
                        },
                    ],
                    label: Some("light bind group with shadow and skybox"),
                });

            self.light_bind_group = Some(bind_group);
        }

        let mut light_uniform = LightUniform::default();
        light_uniform.ambient_color = Vec3::ONE.to_array();
        light_uniform.ambient_strength = 0.01;

        light_uniform.point_light_count = lights.point_lights.len() as u32;
        for i in 0..lights.point_lights.len() {
            light_uniform.point_lights[i] = lights.point_lights[i];
        }

        if lights.directional_light.is_some() {
            light_uniform.directional_light = lights.directional_light.unwrap();
        }

        render_server.queue.write_buffer(
            self.light_uniform_buffer.as_ref().unwrap(),
            0,
            bytemuck::cast_slice(&[light_uniform]),
        );
    }
}

pub(crate) fn prepare_meshes(
    extracted_meshes: &Vec<ExtractedMesh>,
    extracted_lights: &ExtractedLights,
    texture_cache: &TextureCache,
    shader_maker: &mut ShaderMaker,
    mesh_render_resources: &mut MeshRenderResources,
    light_render_resources: &LightRenderResources,
    camera_render_resources: &CameraRenderResources,
    render_server: &RenderServer,
    mesh_cache: &MeshCache,
    ssao_texture_id: TextureId,
    skybox_texture_id: Option<TextureId>,
) {
    mesh_render_resources.prepare_materials(&texture_cache, render_server);
    mesh_render_resources.prepare_pipeline(
        render_server,
        shader_maker,
        &camera_render_resources.bind_group_layout,
    );

    mesh_render_resources.prepare_lights(
        render_server,
        extracted_lights,
        light_render_resources,
        texture_cache,
        ssao_texture_id,
        skybox_texture_id,
    );

    mesh_render_resources.prepare_instances(
        render_server,
        &extracted_meshes,
        mesh_cache,
        camera_render_resources.uniform_buffer.as_ref().unwrap(),
    );
}

pub(crate) fn render_meshes<'a, 'b: 'a>(
    extracted_meshes: &'b Vec<ExtractedMesh>,
    mesh_cache: &'b MeshCache,
    mesh_render_resources: &'b MeshRenderResources,
    camera_render_resources: &'b CameraRenderResources,
    camera_index: usize,
    camera_uniform: &CameraUniform,
    gizmo_render_resources: &'b GizmoRenderResources,
    render_pass: &mut wgpu::RenderPass<'a>,
    bvh: &'b Bvh,
) {
    if camera_render_resources.bind_group.is_none() {
        return;
    }
    if mesh_render_resources.light_bind_group.is_none() {
        return;
    }
    if mesh_render_resources.bindless_bind_group.is_none() {
        return;
    }

    let camera_bind_group = camera_render_resources.bind_group.as_ref().unwrap();
    let light_bind_group = mesh_render_resources.light_bind_group.as_ref().unwrap();
    let bindless_bind_group = mesh_render_resources.bindless_bind_group.as_ref().unwrap();

    let frustum = Frustum::from_view_proj(Mat4::from_cols_array_2d(&camera_uniform.view_proj));

    let mut visible_indices = Vec::new();
    if bvh.root.is_some() {
        bvh.query(&frustum, &mut visible_indices);
    } else {
        visible_indices = (0..extracted_meshes.len()).collect();
    }

    let pipeline = mesh_render_resources.get_pipeline();
    render_pass.set_pipeline(pipeline);
    render_pass.set_bind_group(0, camera_bind_group, &[camera_index as u32 * CameraUniform::get_uniform_offset_unit()]);
    render_pass.set_bind_group(1, light_bind_group, &[]);
    render_pass.set_bind_group(2, bindless_bind_group, &[]);

    // Grouping by mesh_id is now intrinsic because we use indirect buffers per mesh
    for (mesh_id, metadata) in &mesh_render_resources.instance_cache {
        let mesh = mesh_cache.get(*mesh_id).unwrap();

        render_pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
        render_pass.set_vertex_buffer(1, metadata.visible_buffer.slice(..));
        render_pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);

        render_pass.draw_indexed_indirect(&metadata.indirect_buffer, 0);
    }

    gizmo_render_resources.render(
        render_pass,
        camera_render_resources.bind_group.as_ref().unwrap(),
    );
}
