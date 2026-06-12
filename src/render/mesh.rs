use crate::math::aabb::Aabb;
use crate::math::transform::Transform3d;
use crate::render::material::{MaterialCache, MaterialId, MaterialStandard};
use crate::render::mesh_allocator::MeshAllocator;
use crate::render::shader_maker::ShaderMaker;
use crate::render::sprite::ExtractedSprite2d;
use crate::render::{RenderContext, TextureCache, TextureId};
use glam::{Mat3, Mat4, Quat, Vec3};
use std::collections::HashMap;
use std::mem;
use wgpu::BufferAddress;
use crate::render::render_backend::RenderBackend;

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
pub(crate) struct MeshMetadata {
    pub(crate) aabb_min: [f32; 4],
    pub(crate) aabb_max: [f32; 4],
    pub(crate) base_instance: u32,
    pub(crate) instance_count: u32,
    pub(crate) _pad: [u32; 2],
}

pub struct MeshRenderResources {
    // pub(crate) dummy_2d_view: wgpu::TextureView,
    // pub(crate) dummy_cube_view: wgpu::TextureView,
    // pub(crate) dummy_sampler: wgpu::Sampler,
    pub(crate) bindless_bind_group_layout: wgpu::BindGroupLayout,
    pub(crate) bindless_bind_group: Option<wgpu::BindGroup>,

    pub(crate) materials_storage_buffer: Option<wgpu::Buffer>,
    
    pub(crate) material_cache: MaterialCache,
    pub(crate) mesh_allocator: MeshAllocator,

    /// Mesh
    pub(crate) global_instance_buffer: Option<wgpu::Buffer>,
    pub(crate) mesh_metadata_buffer: Option<wgpu::Buffer>,
    pub(crate) mesh_id_to_index: HashMap<MeshId, u32>,
    pub(crate) draw_counts: Vec<u32>,
    pub(crate) mesh_infos: Vec<MeshInstanceInfo>,
}

#[derive(Clone, Copy)]
pub(crate) struct MeshInstanceInfo {
    pub(crate) mesh_id: MeshId,
    pub(crate) base_instance: u32,
    pub(crate) instance_count: u32,
}

// impl MeshRenderResources {
//     pub(crate) fn new(render_server: &RenderContext) -> Self {
// 
//         Self {
//             bindless_bind_group_layout,
//             bindless_bind_group: None,
//             materials_storage_buffer: None,
//             material_cache: MaterialCache::new(),
//             mesh_allocator: MeshAllocator::new(&render_server.device),
//             global_instance_buffer: None,
//             mesh_metadata_buffer: None,
//             mesh_id_to_index: HashMap::new(),
//             draw_counts: Vec::new(),
//             mesh_infos: Vec::new(),
//             indirect_commands: vec![],
//         }
//     }
// 
// 
// }
