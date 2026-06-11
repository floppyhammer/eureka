use crate::math::aabb::Aabb;
use crate::math::transform::Transform3d;
use crate::render::gizmo::GizmoRenderResources;
use crate::render::material::{MaterialCache, MaterialId, MaterialStandard};
use crate::render::mesh_allocator::MeshAllocator;
use crate::render::shader_maker::ShaderMaker;
use crate::render::sprite::ExtractedSprite2d;
use crate::render::vertex::{Vertex3d, VertexSky};
use crate::render::{RenderContext, TextureCache, TextureId};
use glam::{Mat3, Mat4, Quat, Vec3};
use std::collections::HashMap;
use std::mem;
use wgpu::BufferAddress;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct MeshId(pub(crate) uuid::Uuid);

/// CPU mesh cache.
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

#[derive(Clone)]
pub struct Mesh {
    pub name: String,
    pub vertex_offset: u32,
    pub index_offset: u32,
    pub index_count: u32,
    pub aabb: Aabb,
}

impl Mesh {
    pub fn new(
        name: &str,
        vertex_offset: u32,
        index_offset: u32,
        index_count: u32,
        aabb: Aabb,
    ) -> Self {
        Self {
            name: name.to_string(),
            vertex_offset,
            index_offset,
            index_count,
            aabb,
        }
    }

    pub fn default_3d(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        allocator: &mut MeshAllocator,
        mesh_cache: &mut MeshCache,
    ) -> MeshId {
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
        let indices = [0u32, 1, 2, 2, 3, 0];
        let aabb = Aabb::from_points(
            &vertices
                .iter()
                .map(|v| Vec3::from_slice(&v.position))
                .collect::<Vec<_>>(),
        );
        let (v_offset, i_offset) = allocator.allocate(device, queue, &vertices, &indices);
        mesh_cache.add(Mesh::new(
            "default 3d mesh",
            v_offset,
            i_offset,
            indices.len() as u32,
            aabb,
        ))
    }

    pub fn default_skybox(queue: &wgpu::Queue, allocator: &mut MeshAllocator) -> Mesh {
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
        allocator.setup_skybox(queue, &vertices, &indices);
        Mesh::new(
            "default skybox",
            0,
            0,
            indices.len() as u32,
            Aabb::default(),
        )
    }
}

#[derive(Debug, Copy, Clone)]
pub struct ExtractedMesh {
    pub(crate) transform: Transform3d,
    pub(crate) mesh_id: MeshId,
    pub(crate) material_id: Option<MaterialId>,
    // todo: should remove, as it's already included in material
    pub(crate) transparent: bool,
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct InstanceRaw {
    model: [[f32; 4]; 4],
    normal: [[f32; 4]; 3],
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
                    offset: 16,
                    shader_location: 6,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: 32,
                    shader_location: 7,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: 48,
                    shader_location: 8,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: 64,
                    shader_location: 9,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: 80,
                    shader_location: 10,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: 96,
                    shader_location: 11,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: 112,
                    shader_location: 12,
                    format: wgpu::VertexFormat::Uint32,
                },
            ],
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct MeshMetadata {
    aabb_min: [f32; 4],
    aabb_max: [f32; 4],
    base_instance: u32,
    instance_count: u32,
    _pad: [u32; 2],
}

pub struct MeshRenderResources {
    pub(crate) dummy_2d_view: wgpu::TextureView,
    pub(crate) dummy_cube_view: wgpu::TextureView,
    pub(crate) dummy_sampler: wgpu::Sampler,
    pub(crate) bindless_bind_group_layout: wgpu::BindGroupLayout,
    pub(crate) bindless_bind_group: Option<wgpu::BindGroup>,
    pub(crate) materials_storage_buffer: Option<wgpu::Buffer>,
    pub(crate) texture_index_map: HashMap<TextureId, u32>,
    pub(crate) material_index_map: HashMap<MaterialId, u32>,
    pub material_cache: MaterialCache,
    pub(crate) mesh_allocator: MeshAllocator,

    /// Mesh
    pub(crate) global_instance_buffer: Option<wgpu::Buffer>,
    pub(crate) mesh_metadata_buffer: Option<wgpu::Buffer>,
    pub(crate) mesh_id_to_index: HashMap<MeshId, u32>,
    pub(crate) draw_counts: Vec<u32>,
    pub(crate) mesh_infos: Vec<MeshInstanceInfo>,

    /// After cull
    pub(crate) global_visible_instance_buffer: Option<wgpu::Buffer>,
    pub(crate) global_indirect_buffer: Option<wgpu::Buffer>,
}

#[derive(Clone, Copy)]
pub(crate) struct MeshInstanceInfo {
    pub(crate) mesh_id: MeshId,
    pub(crate) base_instance: u32,
    pub(crate) instance_count: u32,
}

impl MeshRenderResources {
    pub(crate) fn new(render_server: &RenderContext) -> Self {
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
            let texture = render_server
                .device
                .create_texture(&wgpu::TextureDescriptor {
                    label: Some("dummy 2d"),
                    size: wgpu::Extent3d {
                        width: 1,
                        height: 1,
                        depth_or_array_layers: 1,
                    },
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
            let texture = render_server
                .device
                .create_texture(&wgpu::TextureDescriptor {
                    label: Some("dummy cube"),
                    size: wgpu::Extent3d {
                        width: 1,
                        height: 1,
                        depth_or_array_layers: 6,
                    },
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

        let dummy_sampler = render_server
            .device
            .create_sampler(&wgpu::SamplerDescriptor {
                label: Some("mesh bindless sampler"),
                address_mode_u: wgpu::AddressMode::Repeat,
                address_mode_v: wgpu::AddressMode::Repeat,
                address_mode_w: wgpu::AddressMode::Repeat,
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                mipmap_filter: wgpu::FilterMode::Linear,
                ..Default::default()
            });

        Self {
            dummy_2d_view,
            dummy_cube_view,
            dummy_sampler,
            bindless_bind_group_layout,
            bindless_bind_group: None,
            materials_storage_buffer: None,
            texture_index_map: HashMap::new(),
            material_index_map: HashMap::new(),
            material_cache: MaterialCache::new(),
            mesh_allocator: MeshAllocator::new(&render_server.device),
            global_instance_buffer: None,
            global_visible_instance_buffer: None,
            global_indirect_buffer: None,
            mesh_metadata_buffer: None,
            mesh_id_to_index: HashMap::new(),
            draw_counts: Vec::new(),
            mesh_infos: Vec::new(),
        }
    }

    pub fn prepare_materials(
        &mut self,
        imported_texture_cache: &TextureCache,
        render_server: &RenderContext,
        extracted_sprites_2d: &[ExtractedSprite2d],
    ) {
        // 关键：我们不再每一帧都重建 Map。
        // 我们只在必要时清空并重建，或者采用增量方式。
        // 为了目前最稳妥的修复闪烁，我们先清空但确保 3D 材质的顺序是绝对固定的。
        self.texture_index_map.clear();
        let mut texture_views = Vec::new();

        // 1. 搜集材质纹理 (这部分顺序通过 sorted_materials 保证绝对固定)
        let mut sorted_materials: Vec<_> = self.material_cache.storage.iter().collect();
        sorted_materials.sort_by(|(id1, _), (id2, _)| id1.0.cmp(&id2.0));

        for (_, material) in &sorted_materials {
            for id in [
                material.color_texture,
                material.normal_texture,
                material.metallic_roughness_texture,
            ]
            .into_iter()
            .flatten()
            {
                if !self.texture_index_map.contains_key(&id) {
                    if let Some(texture) = imported_texture_cache.get(id) {
                        self.texture_index_map
                            .insert(id, texture_views.len() as u32);
                        texture_views.push(&texture.view);
                    }
                }
            }
        }

        // 2. 搜集 2D UI 纹理 (放在 3D 材质之后)
        // 这里的顺序也需要通过 ID 排序来保证固定
        let mut sprite_texture_ids: Vec<_> =
            extracted_sprites_2d.iter().map(|s| s.texture_id).collect();
        sprite_texture_ids.sort();
        sprite_texture_ids.dedup();

        for id in sprite_texture_ids {
            if !self.texture_index_map.contains_key(&id) {
                if let Some(texture) = imported_texture_cache.get(id) {
                    self.texture_index_map
                        .insert(id, texture_views.len() as u32);
                    texture_views.push(&texture.view);
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

        let buffer_size = (material_uniforms.len()
            * mem::size_of::<crate::render::material::MaterialUniform>())
            as u64;
        if self.materials_storage_buffer.is_none()
            || self.materials_storage_buffer.as_ref().unwrap().size() < buffer_size
        {
            self.materials_storage_buffer =
                Some(render_server.device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("materials storage"),
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

        self.bindless_bind_group = Some(
            render_server
                .device
                .create_bind_group(&wgpu::BindGroupDescriptor {
                    layout: &self.bindless_bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: self
                                .materials_storage_buffer
                                .as_ref()
                                .unwrap()
                                .as_entire_binding(),
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
                }),
        );
    }

    pub fn prepare_pipeline(
        &mut self,
        _render_server: &RenderContext,
        _shader_maker: &mut ShaderMaker,
        _camera_bind_group_layout: &wgpu::BindGroupLayout,
    ) {
    }

    pub(crate) fn prepare_instances(
        &mut self,
        render_server: &RenderContext,
        extracted_meshes: &Vec<ExtractedMesh>,
        mesh_cache: &MeshCache,
    ) {
        let mut grouped_instances: HashMap<MeshId, Vec<InstanceRaw>> = HashMap::new();
        for mesh in extracted_meshes {
            let material_idx = mesh
                .material_id
                .and_then(|id| self.material_index_map.get(&id))
                .cloned()
                .unwrap_or(0);
            grouped_instances.entry(mesh.mesh_id).or_default().push(
                Instance {
                    position: mesh.transform.position,
                    scale: mesh.transform.scale,
                    rotation: mesh.transform.rotation,
                    material_idx,
                }
                .to_raw(),
            );
        }

        let mut all_instances = Vec::new();
        let mut mesh_metadatas = Vec::new();
        let mut indirect_commands = Vec::new();
        self.mesh_id_to_index.clear();
        self.mesh_infos.clear();
        let mut current_base_instance = 0u32;
        let mut sorted_meshes: Vec<_> = grouped_instances.keys().cloned().collect();
        sorted_meshes.sort_by_key(|id| id.0);

        for mesh_id in sorted_meshes {
            let instances = &grouped_instances[&mesh_id];
            let mesh = mesh_cache.get(mesh_id).unwrap();
            self.mesh_id_to_index
                .insert(mesh_id, mesh_metadatas.len() as u32);
            self.mesh_infos.push(MeshInstanceInfo {
                mesh_id,
                base_instance: current_base_instance,
                instance_count: instances.len() as u32,
            });
            mesh_metadatas.push(MeshMetadata {
                aabb_min: mesh.aabb.min.extend(0.0).to_array(),
                aabb_max: mesh.aabb.max.extend(0.0).to_array(),
                base_instance: current_base_instance,
                instance_count: instances.len() as u32,
                _pad: [0; 2],
            });
            indirect_commands.push(wgpu::util::DrawIndexedIndirectArgs {
                index_count: mesh.index_count,
                instance_count: 0,
                first_index: mesh.index_offset,
                base_vertex: mesh.vertex_offset as i32,
                first_instance: current_base_instance,
            });
            all_instances.extend_from_slice(instances);
            current_base_instance += instances.len() as u32;
        }

        if all_instances.is_empty() {
            return;
        }
        let instance_buffer_size =
            (all_instances.len() * mem::size_of::<InstanceRaw>()) as BufferAddress;
        if self.global_instance_buffer.is_none()
            || self.global_instance_buffer.as_ref().unwrap().size() < instance_buffer_size
        {
            self.global_instance_buffer =
                Some(render_server.device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("global instances"),
                    size: instance_buffer_size,
                    usage: wgpu::BufferUsages::STORAGE
                        | wgpu::BufferUsages::VERTEX
                        | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                }));
            self.global_visible_instance_buffer =
                Some(render_server.device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("visible instances"),
                    size: instance_buffer_size,
                    usage: wgpu::BufferUsages::STORAGE
                        | wgpu::BufferUsages::VERTEX
                        | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                }));
        }
        let indirect_buffer_size = (indirect_commands.len() * 20) as BufferAddress;
        if self.global_indirect_buffer.is_none()
            || self.global_indirect_buffer.as_ref().unwrap().size() < indirect_buffer_size
        {
            self.global_indirect_buffer =
                Some(render_server.device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("global indirect"),
                    size: indirect_buffer_size,
                    usage: wgpu::BufferUsages::STORAGE
                        | wgpu::BufferUsages::INDIRECT
                        | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                }));
        }
        let metadata_buffer_size =
            (mesh_metadatas.len() * mem::size_of::<MeshMetadata>()) as BufferAddress;
        if self.mesh_metadata_buffer.is_none()
            || self.mesh_metadata_buffer.as_ref().unwrap().size() < metadata_buffer_size
        {
            self.mesh_metadata_buffer =
                Some(render_server.device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("mesh metadata"),
                    size: metadata_buffer_size,
                    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                }));
        }

        render_server.queue.write_buffer(
            self.global_instance_buffer.as_ref().unwrap(),
            0,
            bytemuck::cast_slice(&all_instances),
        );
        render_server.queue.write_buffer(
            self.global_indirect_buffer.as_ref().unwrap(),
            0,
            bytemuck::cast_slice(&indirect_commands),
        );
        render_server.queue.write_buffer(
            self.mesh_metadata_buffer.as_ref().unwrap(),
            0,
            bytemuck::cast_slice(&mesh_metadatas),
        );

        // 移除了 cull_bind_group 的预创建，现在由 CullingNode 在运行时动态创建并缓存
        self.draw_counts = vec![indirect_commands.len() as u32];
    }
}
