use anyhow::Context;
use anyhow::*;
use cgmath::InnerSpace;
use cgmath::*;
use std::error::Error;
use std::ops::Range;
use std::path::Path;
use tobj::LoadOptions;
use wgpu::util::DeviceExt;

use crate::resource::{material, mesh, texture};
use crate::scene::AsNode;
use crate::{Camera2d, InputEvent, RenderServer, Singletons};
use material::Material3d;
use mesh::{Mesh, Vertex3d};

pub struct Model {
    pub position: cgmath::Vector3<f32>,
    pub rotation: cgmath::Quaternion<f32>,
    pub scale: cgmath::Vector3<f32>,

    // A single model usually contains multiple meshes.
    pub meshes: Vec<Mesh>,

    // Mesh material.
    pub materials: Vec<Material3d>,

    // For instancing.
    instances: Vec<Instance>,
    instance_buffer: wgpu::Buffer,

    // For debugging.
    pub name: String,
}

// [Instance] provides model instancing.
// Drawing thousands of [Model]s can be slow,
// since each object is submitted to the GPU then drawn individually.
pub(crate) struct Instance {
    pub(crate) position: cgmath::Vector3<f32>,
    pub(crate) rotation: cgmath::Quaternion<f32>,
}

impl Instance {
    /// Convert Instance to to InstanceRaw.
    pub(crate) fn to_raw(&self) -> InstanceRaw {
        let model =
            cgmath::Matrix4::from_translation(self.position) * cgmath::Matrix4::from(self.rotation);

        InstanceRaw {
            model: model.into(),
            normal: cgmath::Matrix3::from(self.rotation).into(),
        }
    }
}

/// Used to store Instance in a format that shaders can easily understand.
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct InstanceRaw {
    model: [[f32; 4]; 4],
    // Normal matrix to correct the normal direction.
    normal: [[f32; 3]; 3],
}

impl InstanceRaw {
    pub(crate) fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<InstanceRaw>() as wgpu::BufferAddress,
            // We need to switch from using a step mode of Vertex to Instance
            // This means that our shaders will only change to use the next
            // instance when the shader starts processing a new instance.
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    // While our vertex shader only uses locations 0, and 1 now, in later tutorials we'll
                    // be using 2, 3, and 4, for Vertex. We'll start at slot 5 not conflict with them later
                    shader_location: 5,
                    format: wgpu::VertexFormat::Float32x4,
                },
                // A mat4 takes up 4 vertex slots as it is technically 4 vec4s. We need to define a slot
                // for each vec4. We'll have to reassemble the mat4 in
                // the shader.
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
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 19]>() as wgpu::BufferAddress,
                    shader_location: 10,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 22]>() as wgpu::BufferAddress,
                    shader_location: 11,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        }
    }
}

impl Model {
    /// Load model from a wavefront file (.obj).
    pub fn load<P: AsRef<Path>>(render_server: &RenderServer, path: P) -> Result<Self> {
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
        let mut materials = Vec::new();
        for m in obj_materials {
            // Load diffuse texture.
            let diffuse_texture = match texture::Texture::load(
                device,
                queue,
                containing_folder.join(&m.diffuse_texture),
            ) {
                Ok(i) => i,
                Err(e) => {
                    log::warn!(
                        "Failed to load diffuse texture {:?}: {}",
                        m.diffuse_texture,
                        e
                    );
                    texture::Texture::empty(device, queue, (4, 4))?
                }
            };

            // Load normal texture.
            let normal_texture = match texture::Texture::load(
                device,
                queue,
                containing_folder.join(&m.normal_texture),
            ) {
                Ok(i) => i,
                Err(e) => {
                    log::warn!(
                        "Failed to load normal texture {:?}: {}",
                        m.normal_texture,
                        e
                    );
                    texture::Texture::empty(device, queue, (4, 4))?
                }
            };

            // Create a bind group for the material textures.
            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &render_server.model_texture_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&diffuse_texture.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&diffuse_texture.sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(&normal_texture.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::Sampler(&normal_texture.sampler),
                    },
                ],
                label: None,
            });

            materials.push(Material3d {
                name: m.name,
                diffuse_texture,
                normal_texture,
                bind_group,
            });
        }

        // Handle meshes.
        let mut meshes = Vec::new();
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

                let pos0: cgmath::Vector3<_> = v0.position.into();
                let pos1: cgmath::Vector3<_> = v1.position.into();
                let pos2: cgmath::Vector3<_> = v2.position.into();

                let uv0: cgmath::Vector2<_> = v0.uv.into();
                let uv1: cgmath::Vector2<_> = v1.uv.into();
                let uv2: cgmath::Vector2<_> = v2.uv.into();

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
                    (tangent + cgmath::Vector3::from(vertices[c[0] as usize].tangent)).into();
                vertices[c[1] as usize].tangent =
                    (tangent + cgmath::Vector3::from(vertices[c[1] as usize].tangent)).into();
                vertices[c[2] as usize].tangent =
                    (tangent + cgmath::Vector3::from(vertices[c[2] as usize].tangent)).into();
                vertices[c[0] as usize].bi_tangent =
                    (bi_tangent + cgmath::Vector3::from(vertices[c[0] as usize].bi_tangent)).into();
                vertices[c[1] as usize].bi_tangent =
                    (bi_tangent + cgmath::Vector3::from(vertices[c[1] as usize].bi_tangent)).into();
                vertices[c[2] as usize].bi_tangent =
                    (bi_tangent + cgmath::Vector3::from(vertices[c[2] as usize].bi_tangent)).into();

                // Used to average the tangents/bi-tangents.
                triangles_included[c[0] as usize] += 1;
                triangles_included[c[1] as usize] += 1;
                triangles_included[c[2] as usize] += 1;
            }

            // Average the tangents/bi-tangents.
            for (i, n) in triangles_included.into_iter().enumerate() {
                let denom = 1.0 / n as f32;
                let mut v = &mut vertices[i];
                v.tangent = (cgmath::Vector3::from(v.tangent) * denom)
                    .normalize()
                    .into();
                v.bi_tangent = (cgmath::Vector3::from(v.bi_tangent) * denom)
                    .normalize()
                    .into();
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

            meshes.push(Mesh {
                name: m.name,
                vertex_buffer,
                index_buffer,
                index_count: m.mesh.indices.len() as u32,
                material: m.mesh.material_id.unwrap_or(0),
            });
        }

        // Transform.
        let position = cgmath::Vector3 {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        };
        let rotation = if position.is_zero() {
            // This is needed so an object at (0, 0, 0) won't get scaled to zero
            // as Quaternions can effect scale if they're not created correctly.
            cgmath::Quaternion::from_axis_angle(cgmath::Vector3::unit_z(), cgmath::Deg(0.0))
        } else {
            cgmath::Quaternion::from_axis_angle(position.normalize(), cgmath::Deg(45.0))
        };
        let scale = cgmath::Vector3 {
            x: 1.0,
            y: 1.0,
            z: 1.0,
        };

        // Set instance data. Default number of instances is one.
        let instances = vec![{ Instance { position, rotation } }];

        // Copy data from [Instance] to [InstanceRaw].
        let instance_data = instances.iter().map(Instance::to_raw).collect::<Vec<_>>();

        // Create the instance buffer.
        let instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("model instance buffer"),
            contents: bytemuck::cast_slice(&instance_data),
            usage: wgpu::BufferUsages::VERTEX,
        });

        Ok(Self {
            position,
            rotation,
            scale,
            meshes,
            materials,
            name: "".to_string(),
            instances,
            instance_buffer,
        })
    }

    /// Draw multiple models.
    fn set_instances(&mut self, device: &wgpu::Device) {
        // Instancing.
        const NUM_INSTANCES_PER_ROW: u32 = 1;
        const INSTANCE_DISPLACEMENT: cgmath::Vector3<f32> = cgmath::Vector3::new(
            NUM_INSTANCES_PER_ROW as f32 * 0.5,
            0.0,
            NUM_INSTANCES_PER_ROW as f32 * 0.5,
        );

        const SPACE_BETWEEN: f32 = 3.0;
        let instances = (0..NUM_INSTANCES_PER_ROW)
            .flat_map(|z| {
                (0..NUM_INSTANCES_PER_ROW).map(move |x| {
                    let x = SPACE_BETWEEN * (x as f32 - NUM_INSTANCES_PER_ROW as f32 / 2.0);
                    let z = SPACE_BETWEEN * (z as f32 - NUM_INSTANCES_PER_ROW as f32 / 2.0);

                    let position = cgmath::Vector3 { x, y: 0.0, z };

                    let rotation = if position.is_zero() {
                        // This is needed so an object at (0, 0, 0) won't get scaled to zero
                        // as Quaternions can effect scale if they're not created correctly.
                        cgmath::Quaternion::from_axis_angle(
                            cgmath::Vector3::unit_z(),
                            cgmath::Deg(0.0),
                        )
                    } else {
                        cgmath::Quaternion::from_axis_angle(position.normalize(), cgmath::Deg(45.0))
                    };

                    Instance { position, rotation }
                })
            })
            .collect::<Vec<_>>();

        // Copy data from [Instance] to [InstanceRaw].
        let instance_data = instances.iter().map(Instance::to_raw).collect::<Vec<_>>();

        // Create the instance buffer.
        self.instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("model instance buffer"),
            contents: bytemuck::cast_slice(&instance_data),
            usage: wgpu::BufferUsages::VERTEX,
        });
    }
}

impl AsNode for Model {
    fn input(&mut self, input: &InputEvent) {}

    fn update(
        &mut self,
        queue: &wgpu::Queue,
        dt: f32,
        render_server: &RenderServer,
        singletons: Option<&Singletons>,
    ) {
    }

    fn draw<'a, 'b: 'a>(
        &'b self,
        render_pass: &mut wgpu::RenderPass<'a>,
        render_server: &'b RenderServer,
        singletons: &'b Singletons,
    ) {
        render_pass.set_pipeline(&render_server.model_pipeline);

        // Set vertex buffer for InstanceInput.
        render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));

        render_pass.draw_model_instanced(
            &self,
            0..self.instances.len() as u32,
            &singletons.camera3d.as_ref().unwrap().bind_group,
            &singletons.light.as_ref().unwrap().bind_group,
        );
    }
}

pub trait DrawModel<'a> {
    fn draw_mesh(
        &mut self,
        mesh: &'a Mesh,
        material: &'a Material3d,
        camera_bind_group: &'a wgpu::BindGroup,
        light_bind_group: &'a wgpu::BindGroup,
    );

    fn draw_mesh_instanced(
        &mut self,
        mesh: &'a Mesh,
        material: &'a Material3d,
        instances: Range<u32>,
        camera_bind_group: &'a wgpu::BindGroup,
        light_bind_group: &'a wgpu::BindGroup,
    );

    fn draw_model(
        &mut self,
        model: &'a Model,
        camera_bind_group: &'a wgpu::BindGroup,
        light_bind_group: &'a wgpu::BindGroup,
    );

    fn draw_model_instanced(
        &mut self,
        model: &'a Model,
        instances: Range<u32>,
        camera_bind_group: &'a wgpu::BindGroup,
        light_bind_group: &'a wgpu::BindGroup,
    );
}

/// Rendering a mesh.
impl<'a, 'b> DrawModel<'b> for wgpu::RenderPass<'a>
where
    'b: 'a,
{
    fn draw_mesh(
        &mut self,
        mesh: &'b Mesh,
        material: &'b Material3d,
        camera_bind_group: &'b wgpu::BindGroup,
        light_bind_group: &'b wgpu::BindGroup,
    ) {
        self.draw_mesh_instanced(mesh, material, 0..1, camera_bind_group, light_bind_group);
    }

    /// Draw a single mesh.
    fn draw_mesh_instanced(
        &mut self,
        mesh: &'b Mesh,
        material: &'b Material3d,
        instances: Range<u32>,
        camera_bind_group: &'b wgpu::BindGroup,
        light_bind_group: &'b wgpu::BindGroup,
    ) {
        // Set vertex buffer for VertexInput.
        self.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));

        self.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);

        // Set textures.
        self.set_bind_group(0, &material.bind_group, &[]);

        // Set camera uniform.
        self.set_bind_group(1, camera_bind_group, &[]);

        // Set light uniform.
        self.set_bind_group(2, light_bind_group, &[]);

        self.draw_indexed(0..mesh.index_count, 0, instances);
    }

    /// Draw a model instance.
    fn draw_model(
        &mut self,
        model: &'b Model,
        camera_bind_group: &'b wgpu::BindGroup,
        light_bind_group: &'b wgpu::BindGroup,
    ) {
        self.draw_model_instanced(model, 0..1, camera_bind_group, light_bind_group);
    }

    /// Draw multiple model instances.
    fn draw_model_instanced(
        &mut self,
        model: &'b Model,
        instances: Range<u32>,
        camera_bind_group: &'b wgpu::BindGroup,
        light_bind_group: &'b wgpu::BindGroup,
    ) {
        // Draw every mesh in the model.
        for mesh in &model.meshes {
            // Get material.
            let material = &model.materials[mesh.material];

            self.draw_mesh_instanced(
                mesh,
                material,
                instances.clone(),
                camera_bind_group,
                light_bind_group,
            );
        }
    }
}
