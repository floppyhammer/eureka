use anyhow::Context;
use anyhow::*;
use cgmath::*;
use std::any::Any;
use std::path::Path;
use std::result::Result::Ok;
use std::time::Instant;
use tobj::LoadOptions;
use wgpu::util::DeviceExt;

use crate::math::transform::Transform3d;
use crate::render::draw_command::DrawCommands;
use crate::render::material::{MaterialCache, MaterialId, MaterialStandard};
use crate::render::vertex::Vertex3d;
use crate::render::{
    ExtractedMesh, Instance, Mesh, MeshCache, MeshId, RenderServer, Texture, TextureCache,
};
use crate::scene::{AsNode, NodeType};

pub struct Model {
    pub transform: Transform3d,

    // A single model usually contains multiple meshes.
    pub meshes: Vec<MeshId>,

    // Mesh materials. Same length as the meshes.
    pub materials: Vec<Option<MaterialId>>,

    // // For instancing.
    // instances: Vec<Instance>,
    // instance_buffer: wgpu::Buffer,

    // For debugging.
    pub name: String,
}

impl Model {
    /// Load model from a wavefront file (.obj).
    pub fn load<P: AsRef<Path>>(
        texture_cache: &mut TextureCache,
        material_cache: &mut MaterialCache,
        mesh_cache: &mut MeshCache,
        render_server: &RenderServer,
        path: P,
    ) -> Result<Self> {
        let now = Instant::now();

        let device = &render_server.device;
        let queue = &render_server.queue;

        let (obj_meshes, obj_materials) = tobj::load_obj(
            path.as_ref(),
            &LoadOptions {
                triangulate: true,
                single_index: true,
                ..Default::default()
            },
        )?;

        // Unwrap Result.
        let obj_materials = obj_materials?;

        // We're assuming that the texture files are stored in the same folder as the obj file.
        let containing_folder = path.as_ref().parent().context("Directory has no parent")?;

        // Handle materials.
        let mut local_materials = Vec::new();

        for m in obj_materials {
            // Load color texture.
            let mut color_texture = None;

            if m.diffuse_texture.is_some() {
                color_texture = match Texture::load(
                    device,
                    queue,
                    texture_cache,
                    containing_folder.join(&m.diffuse_texture.clone().unwrap()),
                ) {
                    Ok(i) => Some(i),
                    Err(e) => {
                        log::warn!(
                            "Failed to load diffuse texture {:?}: {}",
                            m.diffuse_texture.clone().unwrap(),
                            e
                        );
                        None
                    }
                };
            }

            // Load normal texture.
            let mut normal_texture = None;

            if m.normal_texture.is_some() {
                normal_texture = match Texture::load(
                    device,
                    queue,
                    texture_cache,
                    containing_folder.join(&m.normal_texture.clone().unwrap()),
                ) {
                    Ok(i) => Some(i),
                    Err(e) => {
                        log::warn!(
                            "Failed to load normal texture {:?}: {}",
                            m.normal_texture.clone().unwrap(),
                            e
                        );
                        None
                    }
                };
            }

            let material = MaterialStandard {
                name: m.name,
                color_texture,
                normal_texture,
                texture_bind_group: None,
                transparent: false,
            };

            local_materials.push(material_cache.add(material));
        }

        // Handle meshes.
        let mut meshes = Vec::new();
        let mut materials = Vec::new();

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
                    // Cf. https://vulkan-tutorial.com/Loading_models
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

            let indices = &m.mesh.indices;
            let mut triangles_included = (0..vertices.len()).collect::<Vec<_>>();

            // Calculate tangents and bi-tangets. We're going to
            // use the triangles, so we need to loop through the
            // indices in chunks of 3.
            for c in indices.chunks(3) {
                let v0 = vertices[c[0] as usize];
                let v1 = vertices[c[1] as usize];
                let v2 = vertices[c[2] as usize];

                let pos0: Vector3<_> = v0.position.into();
                let pos1: Vector3<_> = v1.position.into();
                let pos2: Vector3<_> = v2.position.into();

                let uv0: Vector2<_> = v0.uv.into();
                let uv1: Vector2<_> = v1.uv.into();
                let uv2: Vector2<_> = v2.uv.into();

                // Calculate the edges of the triangle.
                let delta_pos1 = pos1 - pos0;
                let delta_pos2 = pos2 - pos0;

                // This will give us a direction to calculate the
                // tangent and bi-tangent.
                let delta_uv1 = uv1 - uv0;
                let delta_uv2 = uv2 - uv0;

                // Solving the following system of equations will
                // give us the tangent and bi-tangent.
                //     delta_pos1 = delta_uv1.x * T + delta_uv1.y * B
                //     delta_pos2 = delta_uv2.x * T + delta_uv2.y * B
                // Luckily, the place I found this equation provided the solution!
                let r = 1.0 / (delta_uv1.x * delta_uv2.y - delta_uv1.y * delta_uv2.x);
                let tangent = (delta_pos1 * delta_uv2.y - delta_pos2 * delta_uv1.y) * r;
                // We flip the bi-tangent to enable right-handed normal
                // maps with wgpu texture coordinate system.
                let bi_tangent = (delta_pos2 * delta_uv1.x - delta_pos1 * delta_uv2.x) * -r;

                // We'll use the same tangent/bi-tangent for each vertex in the triangle.
                vertices[c[0] as usize].tangent =
                    (tangent + Vector3::from(vertices[c[0] as usize].tangent)).into();
                vertices[c[1] as usize].tangent =
                    (tangent + Vector3::from(vertices[c[1] as usize].tangent)).into();
                vertices[c[2] as usize].tangent =
                    (tangent + Vector3::from(vertices[c[2] as usize].tangent)).into();
                vertices[c[0] as usize].bi_tangent =
                    (bi_tangent + Vector3::from(vertices[c[0] as usize].bi_tangent)).into();
                vertices[c[1] as usize].bi_tangent =
                    (bi_tangent + Vector3::from(vertices[c[1] as usize].bi_tangent)).into();
                vertices[c[2] as usize].bi_tangent =
                    (bi_tangent + Vector3::from(vertices[c[2] as usize].bi_tangent)).into();

                // Used to average the tangents/bi-tangents.
                triangles_included[c[0] as usize] += 1;
                triangles_included[c[1] as usize] += 1;
                triangles_included[c[2] as usize] += 1;
            }

            // Average the tangents/bi-tangents.
            for (i, n) in triangles_included.into_iter().enumerate() {
                let denom = 1.0 / n as f32;
                let mut v = &mut vertices[i];
                v.tangent = (Vector3::from(v.tangent) * denom).normalize().into();
                v.bi_tangent = (Vector3::from(v.bi_tangent) * denom).normalize().into();
            }

            let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(&format!("{:?} Vertex Buffer", path.as_ref())),
                contents: bytemuck::cast_slice(&vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });

            let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(&format!("{:?} Index Buffer", path.as_ref())),
                contents: bytemuck::cast_slice(&m.mesh.indices),
                usage: wgpu::BufferUsages::INDEX,
            });

            let mesh = Mesh {
                name: m.name,
                vertex_buffer,
                index_buffer,
                index_count: m.mesh.indices.len() as u32,
            };

            meshes.push(mesh_cache.add(mesh));

            // Prepare a material id for each mesh.
            if (m.mesh.material_id.is_some()) {
                materials.push(Some(local_materials[m.mesh.material_id.unwrap()]));
            } else {
                materials.push(None);
            }
        }

        // Set instance data. Default number of instances is one.
        // let instances = vec![{ Instance { position, rotation } }];

        let elapsed_time = now.elapsed();
        log::info!(
            "Loading model took {} milliseconds",
            elapsed_time.as_millis()
        );

        Ok(Self {
            transform: Transform3d::default(),
            meshes,
            materials,
            name: "".to_string(),
            // instances,
        })
    }
}

impl AsNode for Model {
    fn node_type(&self) -> NodeType {
        NodeType::Model
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn draw(&self, draw_cmds: &mut DrawCommands) {
        for i in 0..self.meshes.len() {
            let mesh = self.meshes[i];
            let material = self.materials[i];

            let extracted_mesh = ExtractedMesh {
                transform: self.transform,
                mesh_id: mesh,
                material_id: material,
            };

            draw_cmds.extracted.meshes.push(extracted_mesh);
        }

        // // Set vertex buffer for InstanceInput.
        // render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
        //
        // render_pass.draw_model_instanced(
        //     singletons,
        //     &self,
        //     0..self.instances.len() as u32,
        //     &camera_info.bind_group.unwrap(),
        //     &camera_info.bind_group.unwrap(), // FIXME
        // );
    }
}
