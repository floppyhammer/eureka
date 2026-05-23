use anyhow::Context;
use anyhow::*;
use glam::{Quat, Vec2, Vec3};
use std::any::Any;
use std::path::Path;
use std::thread;
use std::result::Result::Ok;
use std::time::Instant;
use tobj::LoadOptions;
use wgpu::util::DeviceExt;

use crate::math::aabb::Aabb;
use crate::render::draw_command::DrawCommands;
use crate::render::material::{MaterialCache, MaterialId, MaterialStandard};
use crate::render::vertex::Vertex3d;
use crate::render::{
    ExtractedMesh, Mesh, MeshCache, MeshId, RenderServer, Texture, TextureCache, TextureId, RawTextureData
};
use crate::scene::d3::node_3d::{AsNode3d, Node3d};
use crate::scene::{AsNode, NodeType};

pub struct RawMeshData {
    pub name: String,
    pub vertices: Vec<Vertex3d>,
    pub indices: Vec<u32>,
    pub material_index: Option<usize>,
    pub aabb: Aabb,
}

pub struct RawMaterialData {
    pub name: String,
    pub base_color: [f32; 4],
    pub metallic: f32,
    pub roughness: f32,
    pub color_texture: Option<RawTextureData>,
    pub normal_texture: Option<RawTextureData>,
}

pub struct RawModelData {
    pub meshes: Vec<RawMeshData>,
    pub materials: Vec<RawMaterialData>,
    pub aabb: Aabb,
}

pub struct Model {
    node_3d: Node3d,

    // A single model usually contains multiple meshes.
    pub meshes: Vec<MeshId>,

    // Mesh materials. Same length as the meshes.
    pub materials: Vec<Option<MaterialId>>,

    pub aabb: Aabb,

    // For debugging.
    pub name: String,
}

impl Model {
    /// Pure CPU: Load and parse model data from disk.
    pub fn parse<P: AsRef<Path>>(path: P) -> Result<RawModelData> {
        log::info!("Starting background parse for: {:?}", path.as_ref());

        let (obj_meshes, obj_materials) = tobj::load_obj(
            path.as_ref(),
            &LoadOptions {
                triangulate: true,
                single_index: true,
                ..Default::default()
            },
        )?;

        let obj_materials = obj_materials?;
        let containing_folder = path.as_ref().parent().context("Directory has no parent")?;

        let mut raw_materials = Vec::new();
        for m in obj_materials {
            let color_texture = if let Some(ref tex_path) = m.diffuse_texture {
                Texture::decode_from_disk(containing_folder.join(tex_path)).ok()
            } else {
                None
            };

            let normal_texture = if let Some(ref tex_path) = m.normal_texture {
                Texture::decode_from_disk(containing_folder.join(tex_path)).ok()
            } else {
                None
            };

            raw_materials.push(RawMaterialData {
                name: m.name,
                base_color: [1.0, 1.0, 1.0, 1.0],
                metallic: 0.0,
                roughness: 0.0,
                color_texture,
                normal_texture,
            });
        }

        let mut raw_meshes = Vec::new();
        let mut model_aabb = Aabb::default();
        let mut first = true;

        for m in obj_meshes {
            let mut vertices = Vec::new();
            for i in 0..m.mesh.positions.len() / 3 {
                vertices.push(Vertex3d {
                    position: [
                        m.mesh.positions[i * 3],
                        m.mesh.positions[i * 3 + 1],
                        m.mesh.positions[i * 3 + 2],
                    ],
                    // Flip the vertical component of the texture coordinates.
                    uv: [m.mesh.texcoords[i * 2], 1.0 - m.mesh.texcoords[i * 2 + 1]],
                    normal: [
                        m.mesh.normals[i * 3],
                        m.mesh.normals[i * 3 + 1],
                        m.mesh.normals[i * 3 + 2],
                    ],
                    // We'll calculate these later.
                    tangent: [0.0; 3],
                    bi_tangent: [0.0; 3],
                });
            }

            // Calculate tangents (same logic as before)
            let indices = &m.mesh.indices;
            let mut triangles_included = vec![0; vertices.len()];
            for c in indices.chunks(3) {
                let v0 = vertices[c[0] as usize];
                let v1 = vertices[c[1] as usize];
                let v2 = vertices[c[2] as usize];
                let pos0 = Vec3::from_array(v0.position);
                let pos1 = Vec3::from_array(v1.position);
                let pos2 = Vec3::from_array(v2.position);
                let uv0 = Vec2::from_array(v0.uv);
                let uv1 = Vec2::from_array(v1.uv);
                let uv2 = Vec2::from_array(v2.uv);
                let delta_pos1 = pos1 - pos0;
                let delta_pos2 = pos2 - pos0;
                let delta_uv1 = uv1 - uv0;
                let delta_uv2 = uv2 - uv0;
                let r = 1.0 / (delta_uv1.x * delta_uv2.y - delta_uv1.y * delta_uv2.x);
                let tangent = (delta_pos1 * delta_uv2.y - delta_pos2 * delta_uv1.y) * r;
                let bi_tangent = (delta_pos2 * delta_uv1.x - delta_pos1 * delta_uv2.x) * -r;
                for &idx in c {
                    vertices[idx as usize].tangent = (tangent + Vec3::from_array(vertices[idx as usize].tangent)).to_array();
                    vertices[idx as usize].bi_tangent = (bi_tangent + Vec3::from_array(vertices[idx as usize].bi_tangent)).to_array();
                    triangles_included[idx as usize] += 1;
                }
            }
            for (i, n) in triangles_included.into_iter().enumerate() {
                if n > 0 {
                    let denom = 1.0 / n as f32;
                    vertices[i].tangent = (Vec3::from_array(vertices[i].tangent) * denom).normalize().to_array();
                    vertices[i].bi_tangent = (Vec3::from_array(vertices[i].bi_tangent) * denom).normalize().to_array();
                }
            }

            let aabb = Aabb::from_points(&vertices.iter().map(|v| Vec3::from_slice(&v.position)).collect::<Vec<_>>());
            if first { model_aabb = aabb; first = false; } else { model_aabb = model_aabb.union(&aabb); }

            raw_meshes.push(RawMeshData {
                name: m.name,
                vertices,
                indices: m.mesh.indices,
                material_index: m.mesh.material_id,
                aabb,
            });
        }

        Ok(RawModelData {
            meshes: raw_meshes,
            materials: raw_materials,
            aabb: model_aabb,
        })
    }

    /// GPU Side: Upload raw data to GPU and populate caches.
    pub fn from_raw(
        raw: RawModelData,
        render_server: &RenderServer,
        texture_cache: &mut TextureCache,
        material_cache: &mut MaterialCache,
        mesh_cache: &mut MeshCache,
    ) -> Self {
        let mut material_ids = Vec::new();
        for m in raw.materials {
            let color_texture = m.color_texture.map(|raw_tex| Texture::from_raw(&render_server.device, &render_server.queue, texture_cache, raw_tex));
            let normal_texture = m.normal_texture.map(|raw_tex| Texture::from_raw(&render_server.device, &render_server.queue, texture_cache, raw_tex));

            material_ids.push(material_cache.add(MaterialStandard {
                name: m.name,
                base_color: m.base_color,
                metallic: m.metallic,
                roughness: m.roughness,
                color_texture,
                normal_texture,
                texture_bind_group: None,
                transparent: false,
            }));
        }

        let mut meshes = Vec::new();
        let mut materials = Vec::new();
        for m in raw.meshes {
            let vertex_buffer = render_server.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(&format!("{} Vertex Buffer", m.name)),
                contents: bytemuck::cast_slice(&m.vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });
            let index_buffer = render_server.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(&format!("{} Index Buffer", m.name)),
                contents: bytemuck::cast_slice(&m.indices),
                usage: wgpu::BufferUsages::INDEX,
            });

            let mesh_id = mesh_cache.add(Mesh {
                name: m.name,
                vertex_buffer,
                index_buffer,
                index_count: m.indices.len() as u32,
                aabb: m.aabb,
            });
            meshes.push(mesh_id);
            materials.push(m.material_index.map(|idx| material_ids[idx]));
        }

        Self {
            node_3d: Node3d::default(),
            meshes,
            materials,
            aabb: raw.aabb,
            name: "".to_string(),
        }
    }

    /// Keep old load for compatibility, but implement using new flow.
    pub fn load<P: AsRef<Path>>(
        texture_cache: &mut TextureCache,
        material_cache: &mut MaterialCache,
        mesh_cache: &mut MeshCache,
        render_server: &RenderServer,
        path: P,
    ) -> Result<Self> {
        let raw = Self::parse(path)?;
        Ok(Self::from_raw(raw, render_server, texture_cache, material_cache, mesh_cache))
    }

    pub fn get_world_aabb(&self) -> Aabb {
        self.aabb.transform(&self.node_3d.transform)
    }
}

impl AsNode for Model {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn node_type(&self) -> NodeType {
        NodeType::Model
    }

    fn draw(&self, draw_cmds: &mut DrawCommands) {
        for i in 0..self.meshes.len() {
            let mesh = self.meshes[i];
            let material = self.materials[i];

            let extracted_mesh = ExtractedMesh {
                transform: self.node_3d.transform,
                mesh_id: mesh,
                material_id: material,
            };

            draw_cmds.extracted.meshes.push(extracted_mesh);
        }
    }
}

impl AsNode3d for Model {
    fn get_position(&self) -> Vec3 {
        self.node_3d.transform.position
    }

    fn set_position(&mut self, position: Vec3) {
        self.node_3d.transform.position = position;
    }

    fn get_rotation(&self) -> Quat {
        self.node_3d.transform.rotation
    }

    fn set_rotation(&mut self, rotation: Quat) {
        self.node_3d.transform.rotation = rotation;
    }

    fn get_scale(&self) -> Vec3 {
        self.node_3d.transform.scale
    }

    fn set_scale(&mut self, scale: Vec3) {
        self.node_3d.transform.scale = scale;
    }
}
