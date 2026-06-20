use crate::animation::property::PropertyProvider;
use crate::core::Singletons;
use crate::math::aabb::Aabb;
use crate::math::transform::Transform3d;
use crate::render::material::{MaterialCache, MaterialId, MaterialStandard};
use crate::render::mesh_allocator::MeshAllocator;
use crate::render::vertex::Vertex3d;
use crate::render::{
    Mesh, MeshCache, MeshId, RawTextureData, RenderContext, Texture, TextureCache,
};
use anyhow::Context;
use anyhow::*;
use glam::{Mat4, Quat, Vec2, Vec3};
use std::any::Any;
use std::path::{Path, PathBuf};
use std::result::Result::Ok;
use tobj::LoadOptions;

#[derive(Clone)]
pub struct RawMeshData {
    pub name: String,
    pub vertices: Vec<Vertex3d>,
    pub indices: Vec<u32>,
    pub material_index: Option<usize>,
    pub aabb: Aabb,
    pub local_transform: Transform3d,
}

#[derive(Clone)]
pub struct RawMaterialData {
    pub name: String,
    pub base_color: [f32; 4],
    pub emissive: [f32; 3],
    pub emissive_strength: f32,
    pub metallic: f32,
    pub roughness: f32,
    pub color_texture: Option<RawTextureData>,
    pub normal_texture: Option<RawTextureData>,
    pub metallic_roughness_texture: Option<RawTextureData>,
    pub occlusion_texture: Option<RawTextureData>,
    pub emissive_texture: Option<RawTextureData>,
    pub transparent: bool,
    pub alpha_cutoff: f32,
    pub alpha_mode: AlphaMode,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlphaMode {
    Opaque,
    Mask,
    Blend,
}

#[derive(Clone)]
pub struct RawModelData {
    pub meshes: Vec<RawMeshData>,
    pub materials: Vec<RawMaterialData>,
    pub aabb: Aabb,
}

pub struct Model {
    pub meshes: Vec<MeshId>,
    pub materials: Vec<Option<MaterialId>>,
    pub mesh_transforms: Vec<Transform3d>,
    pub aabb: Aabb,
}

/// 标记一个实体正在等待模型资产加载
pub struct AssetPending(pub PathBuf);

impl Model {
    /// Create a placeholder model that will be populated later.
    pub fn empty() -> Self {
        Self {
            meshes: Vec::new(),
            materials: Vec::new(),
            mesh_transforms: Vec::new(),
            aabb: Aabb::default(),
        }
    }

    /// Pure CPU: Load and parse model data from disk.
    pub fn parse<P: AsRef<Path>>(path: P) -> Result<RawModelData> {
        let extension = path
            .as_ref()
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();

        match extension.as_str() {
            "obj" => Self::parse_obj(path),
            "gltf" | "glb" => Self::parse_gltf(path),
            _ => Err(anyhow!("Unsupported model extension: {}", extension)),
        }
    }

    fn parse_obj<P: AsRef<Path>>(path: P) -> Result<RawModelData> {
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
                emissive: [0.0, 0.0, 0.0],
                emissive_strength: 1.0,
                metallic: 0.0,
                roughness: 1.0,
                color_texture,
                normal_texture,
                metallic_roughness_texture: None,
                occlusion_texture: None,
                emissive_texture: None,
                transparent: false,
                alpha_cutoff: 0.5,
                alpha_mode: AlphaMode::Opaque,
            });
        }

        let mut raw_meshes = Vec::new();
        let mut model_aabb = Aabb::default();
        let mut first = true;

        for m in obj_meshes {
            let mut vertices = Vec::with_capacity(m.mesh.positions.len() / 3);
            for i in 0..m.mesh.positions.len() / 3 {
                vertices.push(Vertex3d {
                    position: [
                        m.mesh.positions[i * 3],
                        m.mesh.positions[i * 3 + 1],
                        m.mesh.positions[i * 3 + 2],
                    ],
                    uv: if !m.mesh.texcoords.is_empty() {
                        [m.mesh.texcoords[i * 2], 1.0 - m.mesh.texcoords[i * 2 + 1]]
                    } else {
                        [0.0, 0.0]
                    },
                    normal: if !m.mesh.normals.is_empty() {
                        [
                            m.mesh.normals[i * 3],
                            m.mesh.normals[i * 3 + 1],
                            m.mesh.normals[i * 3 + 2],
                        ]
                    } else {
                        [0.0, 1.0, 0.0]
                    },
                    tangent: [0.0; 3],
                    bi_tangent: [0.0; 3],
                });
            }

            // Calculate tangents...
            let indices = &m.mesh.indices;
            for c in indices.chunks(3) {
                let v0_idx = c[0] as usize;
                let v1_idx = c[1] as usize;
                let v2_idx = c[2] as usize;

                let pos0 = Vec3::from_array(vertices[v0_idx].position);
                let pos1 = Vec3::from_array(vertices[v1_idx].position);
                let pos2 = Vec3::from_array(vertices[v2_idx].position);
                let uv0 = Vec2::from_array(vertices[v0_idx].uv);
                let uv1 = Vec2::from_array(vertices[v1_idx].uv);
                let uv2 = Vec2::from_array(vertices[v2_idx].uv);

                let delta_pos1 = pos1 - pos0;
                let delta_pos2 = pos2 - pos0;
                let delta_uv1 = uv1 - uv0;
                let delta_uv2 = uv2 - uv0;

                let denom = delta_uv1.x * delta_uv2.y - delta_uv1.y * delta_uv2.x;
                if denom.abs() > f32::EPSILON {
                    let r = 1.0 / denom;
                    let tangent = (delta_pos1 * delta_uv2.y - delta_pos2 * delta_uv1.y) * r;
                    let bi_tangent = (delta_pos2 * delta_uv1.x - delta_pos1 * delta_uv2.x) * -r;

                    let v0_t = Vec3::from_array(vertices[v0_idx].tangent) + tangent;
                    let v1_t = Vec3::from_array(vertices[v1_idx].tangent) + tangent;
                    let v2_t = Vec3::from_array(vertices[v2_idx].tangent) + tangent;
                    vertices[v0_idx].tangent = v0_t.to_array();
                    vertices[v1_idx].tangent = v1_t.to_array();
                    vertices[v2_idx].tangent = v2_t.to_array();

                    let v0_bt = Vec3::from_array(vertices[v0_idx].bi_tangent) + bi_tangent;
                    let v1_bt = Vec3::from_array(vertices[v1_idx].bi_tangent) + bi_tangent;
                    let v2_bt = Vec3::from_array(vertices[v2_idx].bi_tangent) + bi_tangent;
                    vertices[v0_idx].bi_tangent = v0_bt.to_array();
                    vertices[v1_idx].bi_tangent = v1_bt.to_array();
                    vertices[v2_idx].bi_tangent = v2_bt.to_array();
                }
            }

            for v in &mut vertices {
                let t = Vec3::from_array(v.tangent);
                if t.length_squared() > f32::EPSILON {
                    v.tangent = t.normalize().to_array();
                }
                let bt = Vec3::from_array(v.bi_tangent);
                if bt.length_squared() > f32::EPSILON {
                    v.bi_tangent = bt.normalize().to_array();
                }
            }

            let aabb = Aabb::from_vertices(&vertices);
            if first {
                model_aabb = aabb;
                first = false;
            } else {
                model_aabb = model_aabb.union(&aabb);
            }

            raw_meshes.push(RawMeshData {
                name: m.name,
                vertices,
                indices: m.mesh.indices,
                material_index: m.mesh.material_id,
                aabb,
                local_transform: Transform3d::default(),
            });
        }

        Ok(RawModelData {
            meshes: raw_meshes,
            materials: raw_materials,
            aabb: model_aabb,
        })
    }

    fn parse_gltf<P: AsRef<Path>>(path: P) -> Result<RawModelData> {
        log::info!("Starting background parse for glTF: {:?}", path.as_ref());
        let (document, buffers, images) = gltf::import(path.as_ref())?;

        let mut raw_materials = Vec::new();
        for m in document.materials() {
            let pbr = m.pbr_metallic_roughness();
            let base_color_factor = pbr.base_color_factor();

            let color_texture = pbr.base_color_texture().map(|t| {
                let img = &images[t.texture().source().index()];
                let (data, format) = match img.format {
                    gltf::image::Format::R8G8B8 => {
                        let mut rgba =
                            Vec::with_capacity(img.width as usize * img.height as usize * 4);
                        for chunk in img.pixels.chunks_exact(3) {
                            rgba.push(chunk[0]);
                            rgba.push(chunk[1]);
                            rgba.push(chunk[2]);
                            rgba.push(255);
                        }
                        (rgba, wgpu::TextureFormat::Rgba8UnormSrgb)
                    }
                    gltf::image::Format::R8G8B8A8 => {
                        (img.pixels.clone(), wgpu::TextureFormat::Rgba8UnormSrgb)
                    }
                    _ => (img.pixels.clone(), wgpu::TextureFormat::Rgba8UnormSrgb),
                };

                RawTextureData {
                    name: format!("{}_color", m.name().unwrap_or("")),
                    pixels: data,
                    width: img.width,
                    height: img.height,
                    format,
                }
            });

            let normal_texture = m.normal_texture().map(|t| {
                let img = &images[t.texture().source().index()];
                let (data, format) = match img.format {
                    gltf::image::Format::R8G8B8 => {
                        let mut rgba =
                            Vec::with_capacity(img.width as usize * img.height as usize * 4);
                        for chunk in img.pixels.chunks_exact(3) {
                            rgba.push(chunk[0]);
                            rgba.push(chunk[1]);
                            rgba.push(chunk[2]);
                            rgba.push(255);
                        }
                        (rgba, wgpu::TextureFormat::Rgba8Unorm)
                    }
                    gltf::image::Format::R8G8B8A8 => {
                        (img.pixels.clone(), wgpu::TextureFormat::Rgba8Unorm)
                    }
                    _ => (img.pixels.clone(), wgpu::TextureFormat::Rgba8Unorm),
                };
                RawTextureData {
                    name: format!("{}_normal", m.name().unwrap_or("")),
                    pixels: data,
                    width: img.width,
                    height: img.height,
                    format,
                }
            });

            let metallic_roughness_texture = m
                .pbr_metallic_roughness()
                .metallic_roughness_texture()
                .map(|t| {
                    let img = &images[t.texture().source().index()];
                    let (data, format) = match img.format {
                        gltf::image::Format::R8G8B8 => {
                            let mut rgba =
                                Vec::with_capacity(img.width as usize * img.height as usize * 4);
                            for chunk in img.pixels.chunks_exact(3) {
                                rgba.push(chunk[0]);
                                rgba.push(chunk[1]);
                                rgba.push(chunk[2]);
                                rgba.push(255);
                            }
                            (rgba, wgpu::TextureFormat::Rgba8Unorm)
                        }
                        gltf::image::Format::R8G8B8A8 => {
                            (img.pixels.clone(), wgpu::TextureFormat::Rgba8Unorm)
                        }
                        _ => (img.pixels.clone(), wgpu::TextureFormat::Rgba8Unorm),
                    };
                    RawTextureData {
                        name: format!("{}_metallic_roughness", m.name().unwrap_or("")),
                        pixels: data,
                        width: img.width,
                        height: img.height,
                        format,
                    }
                });

            let occlusion_texture = m.occlusion_texture().map(|t| {
                let img = &images[t.texture().source().index()];
                let (data, format) = match img.format {
                    gltf::image::Format::R8G8B8 => {
                        let mut rgba =
                            Vec::with_capacity(img.width as usize * img.height as usize * 4);
                        for chunk in img.pixels.chunks_exact(3) {
                            rgba.push(chunk[0]);
                            rgba.push(chunk[1]);
                            rgba.push(chunk[2]);
                            rgba.push(255);
                        }
                        (rgba, wgpu::TextureFormat::Rgba8Unorm)
                    }
                    gltf::image::Format::R8G8B8A8 => {
                        (img.pixels.clone(), wgpu::TextureFormat::Rgba8Unorm)
                    }
                    _ => (img.pixels.clone(), wgpu::TextureFormat::Rgba8Unorm),
                };
                RawTextureData {
                    name: format!("{}_occlusion", m.name().unwrap_or("")),
                    pixels: data,
                    width: img.width,
                    height: img.height,
                    format,
                }
            });

            let emissive_texture = m.emissive_texture().map(|t| {
                let img = &images[t.texture().source().index()];
                let (data, format) = match img.format {
                    gltf::image::Format::R8G8B8 => {
                        let mut rgba =
                            Vec::with_capacity(img.width as usize * img.height as usize * 4);
                        for chunk in img.pixels.chunks_exact(3) {
                            rgba.push(chunk[0]);
                            rgba.push(chunk[1]);
                            rgba.push(chunk[2]);
                            rgba.push(255);
                        }
                        (rgba, wgpu::TextureFormat::Rgba8UnormSrgb)
                    }
                    gltf::image::Format::R8G8B8A8 => {
                        (img.pixels.clone(), wgpu::TextureFormat::Rgba8UnormSrgb)
                    }
                    _ => (img.pixels.clone(), wgpu::TextureFormat::Rgba8UnormSrgb),
                };
                RawTextureData {
                    name: format!("{}_emissive", m.name().unwrap_or("")),
                    pixels: data,
                    width: img.width,
                    height: img.height,
                    format,
                }
            });

            let alpha_mode = m.alpha_mode();
            let alpha_cutoff = m.alpha_cutoff().unwrap_or(0.5);

            let is_transparent = match alpha_mode {
                gltf::material::AlphaMode::Blend => true,
                gltf::material::AlphaMode::Mask => false,
                gltf::material::AlphaMode::Opaque => base_color_factor[3] < 1.0,
            };

            let alpha_mode_enum = match alpha_mode {
                gltf::material::AlphaMode::Blend => AlphaMode::Blend,
                gltf::material::AlphaMode::Mask => AlphaMode::Mask,
                gltf::material::AlphaMode::Opaque => AlphaMode::Opaque,
            };

            raw_materials.push(RawMaterialData {
                name: m.name().unwrap_or("").to_string(),
                base_color: base_color_factor,
                emissive: m.emissive_factor(),
                emissive_strength: 1.0, // Default for glTF
                metallic: pbr.metallic_factor(),
                roughness: pbr.roughness_factor(),
                color_texture,
                normal_texture,
                metallic_roughness_texture,
                occlusion_texture,
                emissive_texture,
                transparent: is_transparent,
                alpha_cutoff,
                alpha_mode: alpha_mode_enum,
            });
        }

        let default_material_index = raw_materials.len();
        raw_materials.push(RawMaterialData {
            name: "Default glTF Material".to_string(),
            base_color: [1.0, 1.0, 1.0, 1.0],
            emissive: [0.0, 0.0, 0.0],
            emissive_strength: 1.0,
            metallic: 0.0,
            roughness: 1.0,
            color_texture: None,
            normal_texture: None,
            metallic_roughness_texture: None,
            occlusion_texture: None,
            emissive_texture: None,
            transparent: false,
            alpha_cutoff: 0.5,
            alpha_mode: AlphaMode::Opaque,
        });

        let mut raw_meshes = Vec::new();
        let mut model_aabb = Aabb::default();
        let mut first = true;

        for scene in document.scenes() {
            for node in scene.nodes() {
                Self::process_node(
                    &node,
                    &buffers,
                    &images,
                    &mut raw_meshes,
                    &mut model_aabb,
                    &mut first,
                    Mat4::IDENTITY,
                    default_material_index,
                );
            }
        }

        Ok(RawModelData {
            meshes: raw_meshes,
            materials: raw_materials,
            aabb: model_aabb,
        })
    }

    fn process_node(
        node: &gltf::Node,
        buffers: &[gltf::buffer::Data],
        images: &[gltf::image::Data],
        raw_meshes: &mut Vec<RawMeshData>,
        model_aabb: &mut Aabb,
        first: &mut bool,
        parent_transform: Mat4,
        default_material_index: usize,
    ) {
        let (translation, rotation, scale) = node.transform().decomposed();
        let local_mat = Mat4::from_scale_rotation_translation(
            Vec3::from_array(scale),
            Quat::from_array(rotation),
            Vec3::from_array(translation),
        );
        let world_mat = parent_transform * local_mat;

        if let Some(mesh) = node.mesh() {
            for primitive in mesh.primitives() {
                let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));

                let mut vertices = Vec::new();
                if let Some(positions) = reader.read_positions() {
                    let mut normals = reader.read_normals();
                    let mut tex_coords = reader.read_tex_coords(0).map(|v| v.into_f32());
                    let mut tangents = reader.read_tangents();

                    for pos in positions {
                        let normal = normals
                            .as_mut()
                            .and_then(|n| n.next())
                            .unwrap_or([0.0, 1.0, 0.0]);
                        let uv = tex_coords
                            .as_mut()
                            .and_then(|tc| tc.next())
                            .unwrap_or([0.0, 0.0]);
                        let tangent = tangents
                            .as_mut()
                            .and_then(|t| t.next())
                            .unwrap_or([1.0, 0.0, 0.0, 1.0]);

                        let tangent_vec3 = [tangent[0], tangent[1], tangent[2]];
                        let bitangent = Vec3::from_array(normal)
                            .cross(Vec3::from_array(tangent_vec3))
                            * tangent[3];

                        vertices.push(Vertex3d {
                            position: pos,
                            normal,
                            uv,
                            tangent: tangent_vec3,
                            bi_tangent: bitangent.to_array(),
                        });
                    }
                }

                let indices = reader
                    .read_indices()
                    .map(|indices| match indices {
                        gltf::mesh::util::ReadIndices::U8(it) => it.map(|x| x as u32).collect(),
                        gltf::mesh::util::ReadIndices::U16(it) => it.map(|x| x as u32).collect(),
                        gltf::mesh::util::ReadIndices::U32(it) => it.collect(),
                    })
                    .unwrap_or_else(|| (0..vertices.len() as u32).collect());

                let aabb = Aabb::from_vertices(&vertices);

                let (s, r, t) = world_mat.to_scale_rotation_translation();
                let transform = Transform3d {
                    position: t,
                    rotation: r,
                    scale: s,
                };

                let world_aabb = aabb.transform(&transform);
                if *first {
                    *model_aabb = world_aabb;
                    *first = false;
                } else {
                    *model_aabb = model_aabb.union(&world_aabb);
                }

                raw_meshes.push(RawMeshData {
                    name: mesh.name().unwrap_or("").to_string(),
                    vertices,
                    indices,
                    material_index: Some(
                        primitive
                            .material()
                            .index()
                            .unwrap_or(default_material_index),
                    ),
                    aabb,
                    local_transform: transform,
                });
            }
        }

        for child in node.children() {
            Self::process_node(
                &child,
                buffers,
                images,
                raw_meshes,
                model_aabb,
                first,
                world_mat,
                default_material_index,
            );
        }
    }

    /// Fill this model with actual GPU data.
    pub fn finalize(
        &mut self,
        raw: RawModelData,
        render_server: &RenderContext,
        global_texture_cache: &mut TextureCache,
        global_material_cache: &mut MaterialCache,
        global_mesh_cache: &mut MeshCache,
        global_mesh_allocator: &mut MeshAllocator,
    ) {
        let mut material_ids = Vec::new();

        for m in raw.materials {
            let color_texture = m.color_texture.map(|raw_tex| {
                Texture::from_raw(
                    &render_server.device,
                    &render_server.queue,
                    global_texture_cache,
                    raw_tex,
                )
            });
            let normal_texture = m.normal_texture.map(|raw_tex| {
                Texture::from_raw(
                    &render_server.device,
                    &render_server.queue,
                    global_texture_cache,
                    raw_tex,
                )
            });
            let metallic_roughness_texture = m.metallic_roughness_texture.map(|raw_tex| {
                Texture::from_raw(
                    &render_server.device,
                    &render_server.queue,
                    global_texture_cache,
                    raw_tex,
                )
            });
            let occlusion_texture = m.occlusion_texture.map(|raw_tex| {
                Texture::from_raw(
                    &render_server.device,
                    &render_server.queue,
                    global_texture_cache,
                    raw_tex,
                )
            });
            let emissive_texture = m.emissive_texture.map(|raw_tex| {
                Texture::from_raw(
                    &render_server.device,
                    &render_server.queue,
                    global_texture_cache,
                    raw_tex,
                )
            });

            let alpha_mode = match m.alpha_mode {
                AlphaMode::Opaque => crate::render::material::AlphaMode::Opaque,
                AlphaMode::Mask => crate::render::material::AlphaMode::Mask,
                AlphaMode::Blend => crate::render::material::AlphaMode::Blend,
            };

            material_ids.push(global_material_cache.add(MaterialStandard {
                name: m.name,
                base_color: m.base_color,
                emissive: m.emissive,
                emissive_strength: m.emissive_strength,
                metallic: m.metallic,
                roughness: m.roughness,
                color_texture,
                normal_texture,
                metallic_roughness_texture,
                occlusion_texture,
                emissive_texture,
                transparent: m.transparent,
                alpha_cutoff: m.alpha_cutoff,
                alpha_mode,
            }));
        }

        for m in raw.meshes {
            let (v_offset, i_offset) =
                global_mesh_allocator.allocate(&render_server.queue, &m.vertices, &m.indices);

            let mesh_id = global_mesh_cache.add(Mesh::new(
                &m.name,
                v_offset,
                i_offset,
                m.indices.len() as u32,
                m.aabb,
            ));
            self.meshes.push(mesh_id);
            self.materials
                .push(m.material_index.map(|idx| material_ids[idx]));
            self.mesh_transforms.push(m.local_transform);
        }

        self.aabb = raw.aabb;
    }

    pub fn get_world_aabb(&self, transform: &Transform3d) -> Aabb {
        self.aabb.transform(transform)
    }

    pub fn from_raw(
        raw: RawModelData,
        rs: &RenderContext,
        tc: &mut TextureCache,
        mc: &mut MaterialCache,
        msc: &mut MeshCache,
        ma: &mut MeshAllocator,
    ) -> Self {
        let mut model = Self::empty();
        model.finalize(raw, rs, tc, mc, msc, ma);
        model
    }

    pub fn from_primitive(
        primitive: crate::scene::d3::primitive::MeshPrimitive,
        rs: &RenderContext,
        tc: &mut TextureCache,
        mc: &mut MaterialCache,
        msc: &mut MeshCache,
        ma: &mut MeshAllocator,
    ) -> Self {
        let raw = primitive.generate_raw_data();
        Self::from_raw(raw, rs, tc, mc, msc, ma)
    }

    pub fn load<P: AsRef<Path>>(
        tc: &mut TextureCache,
        mc: &mut MaterialCache,
        msc: &mut MeshCache,
        ma: &mut crate::render::mesh_allocator::MeshAllocator,
        rs: &RenderContext,
        path: P,
    ) -> Result<Self> {
        let raw = Self::parse(path)?;
        Ok(Self::from_raw(raw, rs, tc, mc, msc, ma))
    }
}
