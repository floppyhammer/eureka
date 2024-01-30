use crate::math::transform::Transform3d;
use crate::render::camera::{CameraRenderResources, CameraUniform};
use crate::render::gizmo::GizmoRenderResources;
use crate::render::material::{MaterialCache, MaterialId, MaterialStandard};
use crate::render::shader_maker::ShaderMaker;
use crate::render::vertex::{Vertex2d, Vertex3d, VertexBuffer, VertexSky};
use crate::render::{create_render_pipeline, RenderServer, Texture, TextureCache, TextureId};
use crate::Singletons;
use cgmath::{Deg, InnerSpace, Matrix3, Matrix4, Quaternion, Rotation3, Vector3, Zero};
use lyon::path::Position;
use std::collections::HashMap;
use std::mem;
use std::ops::Range;
use wgpu::util::DeviceExt;
use wgpu::{BufferAddress, Device, SamplerBindingType};
use crate::render::light::LightUniform;

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

/// Shared by 2D/3D meshes.
pub struct Mesh {
    // Mesh name for debugging reason. Not unique.
    pub name: String,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub index_count: u32,
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

        Self {
            name: "default 2d mesh".to_string(),
            vertex_buffer,
            index_buffer,
            index_count: indices.len() as u32,
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

        Self {
            name: "default 3d mesh".to_string(),
            vertex_buffer,
            index_buffer,
            index_count: indices.len() as u32,
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

        Self {
            name: "default skybox mesh".to_string(),
            vertex_buffer,
            index_buffer,
            index_count: indices.len() as u32,
        }
    }
}

/// Minimal data for rendering a mesh.
#[derive(Debug, Copy, Clone)]
pub struct ExtractedMesh {
    pub(crate) transform: Transform3d,
    pub(crate) mesh_id: MeshId,
    pub(crate) material_id: Option<MaterialId>,
}

/// Used to store Instance in a format that shaders can easily understand.
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct InstanceRaw {
    model: [[f32; 4]; 4],
    // Normal matrix to correct the normal direction.
    normal: [[f32; 3]; 3],
}

// [Instance] provides model instancing.
// Drawing thousands of [Model]s can be slow,
// since each object is submitted to the GPU then drawn individually.
pub(crate) struct Instance {
    pub(crate) position: Vector3<f32>,
    pub(crate) rotation: Quaternion<f32>,
}

impl Instance {
    /// Convert Instance to to InstanceRaw.
    pub(crate) fn to_raw(&self) -> InstanceRaw {
        let model = Matrix4::from_translation(self.position) * Matrix4::from(self.rotation);

        InstanceRaw {
            model: model.into(),
            normal: Matrix3::from(self.rotation).into(),
        }
    }
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

pub trait DrawModel<'a> {
    fn draw_mesh(
        &mut self,
        mesh: &'a Mesh,
        material_bind_group: Option<&'a wgpu::BindGroup>,
        camera_bind_group: &'a wgpu::BindGroup,
        light_bind_group: &'a wgpu::BindGroup,
    );

    fn draw_mesh_instanced(
        &mut self,
        mesh: &'a Mesh,
        material_bind_group: Option<&'a wgpu::BindGroup>,
        instances: Range<u32>,
        camera_bind_group: &'a wgpu::BindGroup,
        light_bind_group: &'a wgpu::BindGroup,
    );

    // fn draw_meshes(
    //     &mut self,
    //     singletons: &'a mut Singletons,
    //     meshes: &'a &[Mesh],
    //     materials: &'a &[MaterialId],
    //     camera_bind_group: &'a wgpu::BindGroup,
    //     light_bind_group: &'a wgpu::BindGroup,
    // );
    //
    // fn draw_meshes_instanced(
    //     &mut self,
    //     singletons: &'a mut Singletons,
    //     meshes: &'a &[Mesh],
    //     materials: &'a &[MaterialId],
    //     instances: Range<u32>,
    //     camera_bind_group: &'a wgpu::BindGroup,
    //     light_bind_group: &'a wgpu::BindGroup,
    // );
}

/// Rendering a mesh.
impl<'a, 'b> DrawModel<'b> for wgpu::RenderPass<'a>
where
    'b: 'a,
{
    fn draw_mesh(
        &mut self,
        mesh: &'b Mesh,
        material_bind_group: Option<&'b wgpu::BindGroup>,
        camera_bind_group: &'b wgpu::BindGroup,
        light_bind_group: &'b wgpu::BindGroup,
    ) {
        self.draw_mesh_instanced(
            mesh,
            material_bind_group,
            0..1,
            camera_bind_group,
            light_bind_group,
        );
    }

    /// Draw a single mesh.
    fn draw_mesh_instanced(
        &mut self,
        mesh: &'b Mesh,
        material_bind_group: Option<&'b wgpu::BindGroup>,
        instances: Range<u32>,
        camera_bind_group: &'b wgpu::BindGroup,
        light_bind_group: &'b wgpu::BindGroup,
    ) {
        // Set vertex buffer for VertexInput.
        self.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));

        self.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);

        // Set camera uniform.
        self.set_bind_group(0, camera_bind_group, &[]);

        // Set light uniform.
        self.set_bind_group(1, light_bind_group, &[]);

        // Set textures.
        if (material_bind_group.is_some()) {
            self.set_bind_group(2, material_bind_group.unwrap(), &[]);
        }

        self.draw_indexed(0..mesh.index_count, 0, instances);
    }

    // fn draw_meshes(
    //     &mut self,
    //     singletons: &'b mut Singletons,
    //     meshes: &'b &[Mesh],
    //     materials: &'b &[MaterialId],
    //     camera_bind_group: &'b wgpu::BindGroup,
    //     light_bind_group: &'b wgpu::BindGroup,
    // ) {
    //     self.draw_meshes_instanced(singletons, meshes, materials, 0..1, camera_bind_group, light_bind_group);
    // }

    // fn draw_meshes_instanced(
    //     &mut self,
    //     singletons: &'b mut Singletons,
    //     meshes: &'b [&Mesh],
    //     instances: Range<u32>,
    //     texture_bind_groups: &'b [Option<&wgpu::BindGroup>],
    //     camera_bind_group: &'b wgpu::BindGroup,
    //     light_bind_group: &'b wgpu::BindGroup,
    // ) {
    //     // Draw every mesh in the model.
    //     for i in 0..meshes.len() {
    //         let mesh = &meshes[i];
    //         let material = &materials[i];
    //
    //         self.set_pipeline(singletons.render_server.get_material_3d_pipeline(material));
    //
    //         self.draw_mesh_instanced(
    //             mesh,
    //             material,
    //             instances.clone(),
    //             camera_bind_group,
    //             light_bind_group,
    //         );
    //     }
    // }
}

/// All mesh related resources.
pub struct MeshRenderResources {
    pub(crate) light_bind_group_layout: wgpu::BindGroupLayout,
    pub(crate) light_bind_group: Option<wgpu::BindGroup>,
    pub(crate) light_uniform_buffer: Option<wgpu::Buffer>,

    pub(crate) texture_bind_group_layout_cache: HashMap<u32, wgpu::BindGroupLayout>,
    pub(crate) texture_bind_group_cache: HashMap<MaterialId, wgpu::BindGroup>,

    pub(crate) pipeline_cache: HashMap<u32, wgpu::RenderPipeline>,
    pub material_cache: MaterialCache,

    // For mesh batching.
    pub(crate) instance_cache: HashMap<MeshId, InstanceMetadata>,
}

pub(crate) struct InstanceMetadata {
    pub(crate) buffer: wgpu::Buffer,
    pub(crate) instance_count: u64,
}

impl MeshRenderResources {
    pub(crate) fn new(render_server: &RenderServer) -> Self {
        let light_bind_group_layout =
            render_server
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    entries: &[wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    }],
                    label: Some("mesh light bind group layout"),
                });

        Self {
            light_bind_group_layout,
            light_uniform_buffer: None,
            texture_bind_group_layout_cache: Default::default(),
            light_bind_group: None,
            texture_bind_group_cache: HashMap::new(),

            pipeline_cache: Default::default(),
            material_cache: MaterialCache::new(),
            instance_cache: HashMap::new(),
        }
    }

    pub fn add_texture_bind_group_layout(
        &mut self,
        device: &wgpu::Device,
        material: &MaterialStandard,
    ) {
        let flags = material.get_flags();

        // Try find layout from cache.
        let layout = self.texture_bind_group_layout_cache.get(&flags);

        // Create new layout.
        if layout.is_none() {
            let label = "mesh textures bind group layout";

            let mut bind_group_layout_entries = vec![];
            let mut next_binding = 0;

            // Color texture.
            if material.color_texture.is_some() {
                bind_group_layout_entries.push(wgpu::BindGroupLayoutEntry {
                    binding: next_binding,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                });
                next_binding += 1;

                bind_group_layout_entries.push(wgpu::BindGroupLayoutEntry {
                    binding: next_binding,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler {
                        0: SamplerBindingType::Filtering,
                    },
                    count: None,
                });
                next_binding += 1;
            }

            // Normal texture.
            if material.normal_texture.is_some() {
                bind_group_layout_entries.push(wgpu::BindGroupLayoutEntry {
                    binding: next_binding,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    count: None,
                });
                next_binding += 1;

                bind_group_layout_entries.push(wgpu::BindGroupLayoutEntry {
                    binding: next_binding,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler {
                        0: SamplerBindingType::Filtering,
                    },
                    count: None,
                });
                next_binding += 1;
            }

            let mesh_textures_bind_group_layout =
                device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    entries: bind_group_layout_entries.as_slice(),
                    label: Some(label),
                });

            self.texture_bind_group_layout_cache
                .insert(flags, mesh_textures_bind_group_layout);
        }
    }

    pub fn get_texture_bind_group_layout(
        &self,
        material: &MaterialStandard,
    ) -> &wgpu::BindGroupLayout {
        let flags = material.get_flags();

        self.texture_bind_group_layout_cache.get(&flags).unwrap()
    }

    pub fn prepare_lights(&mut self, render_server: &RenderServer, light_uniform: LightUniform) {
        if self.light_bind_group.is_none() {
            // We'll want to update our lights position, so we use COPY_DST.
            let buffer = render_server.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("light uniform buffer"),
                size: mem::size_of::<LightUniform>() as BufferAddress,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });

            let bind_group = render_server
                .device
                .create_bind_group(&wgpu::BindGroupDescriptor {
                    layout: &self.light_bind_group_layout,
                    entries: &[wgpu::BindGroupEntry {
                        binding: 0,
                        resource: buffer.as_entire_binding(),
                    }],
                    label: None,
                });

            self.light_bind_group = Some(bind_group);
            self.light_uniform_buffer = Some(buffer);
        }

        render_server.queue.write_buffer(
            self.light_uniform_buffer.as_ref().unwrap(),
            0,
            bytemuck::cast_slice(&[light_uniform]),
        );
    }

    /// Prepare texture bind group and its layout for each material.
    pub fn prepare_materials(
        &mut self,
        texture_cache: &TextureCache,
        render_server: &RenderServer,
    ) {
        let mut pairs: Vec<(MaterialId, MaterialStandard)> = vec![];
        for pair in &self.material_cache.storage {
            pairs.push((*pair.0, pair.1.clone()));
        }

        // Prepare texture bind group layouts.
        for pair in &pairs {
            if (pair.1.get_flags() == 0) {
                continue;
            }

            self.add_texture_bind_group_layout(&render_server.device, &pair.1);

            let bind_group_layout = self.get_texture_bind_group_layout(&pair.1);

            let bind_group = self.texture_bind_group_cache.get(&pair.0);
            if (bind_group.is_some()) {
                continue;
            }

            let bind_group_entries = pair.1.get_bind_group_entries(&texture_cache);

            // Create a texture bind group for each material.
            let bind_group = render_server
                .device
                .create_bind_group(&wgpu::BindGroupDescriptor {
                    layout: bind_group_layout,
                    entries: bind_group_entries.as_slice(),
                    label: None,
                });

            self.texture_bind_group_cache.insert(pair.0, bind_group);
        }
    }

    pub fn prepare_pipeline(
        &mut self,
        render_server: &RenderServer,
        shader_maker: &mut ShaderMaker,
        camera_bind_group_layout: &wgpu::BindGroupLayout,
        material_id: Option<MaterialId>,
    ) {
        if (material_id.is_none()) {
            const PLAIN_MATERTIAL_FLAGS: u32 = 0;

            let pipeline = self.pipeline_cache.get(&PLAIN_MATERTIAL_FLAGS);

            // Create new pipeline.
            if pipeline.is_none() {
                let pipeline = {
                    // Set up resource pipeline layout using bind group layouts.
                    let pipeline_layout = render_server.device.create_pipeline_layout(
                        &wgpu::PipelineLayoutDescriptor {
                            label: Some("mesh pipeline layout"),
                            bind_group_layouts: &[
                                camera_bind_group_layout,
                                &self.light_bind_group_layout,
                            ],
                            push_constant_ranges: &[],
                        },
                    );

                    // Shader descriptor, not a shader module yet.
                    let shader = wgpu::ShaderModuleDescriptor {
                        label: Some("standard material shader"),
                        source: shader_maker
                            .make_shader(include_str!("../shaders/mesh.wgsl"), &[])
                            .unwrap(),
                    };

                    create_render_pipeline(
                        &render_server.device,
                        &pipeline_layout,
                        render_server.surface_config.format,
                        Some(Texture::DEPTH_FORMAT),
                        &[Vertex3d::desc(), InstanceRaw::desc()],
                        shader,
                        "standard material pipeline",
                        false,
                        Some(wgpu::Face::Back),
                    )
                };

                self.pipeline_cache.insert(PLAIN_MATERTIAL_FLAGS, pipeline);
            }
        } else {
            let material = self
                .material_cache
                .get(&material_id.unwrap())
                .unwrap()
                .clone();

            let flags = material.get_flags();

            self.add_texture_bind_group_layout(&render_server.device, &material);

            let pipeline = self.pipeline_cache.get(&flags);

            // Create new pipeline.
            if pipeline.is_none() {
                let pipeline = {
                    // Set up resource pipeline layout using bind group layouts.
                    let pipeline_layout = render_server.device.create_pipeline_layout(
                        &wgpu::PipelineLayoutDescriptor {
                            label: Some("mesh pipeline layout"),
                            bind_group_layouts: &[
                                camera_bind_group_layout,
                                &self.light_bind_group_layout,
                                self.get_texture_bind_group_layout(&material),
                            ],
                            push_constant_ranges: &[],
                        },
                    );

                    // Shader descriptor, not a shader module yet.
                    let shader = wgpu::ShaderModuleDescriptor {
                        label: Some("standard material shader"),
                        source: shader_maker
                            .make_shader(
                                include_str!("../shaders/mesh.wgsl"),
                                material.get_shader_defs().as_slice(),
                            )
                            .unwrap(),
                    };

                    create_render_pipeline(
                        &render_server.device,
                        &pipeline_layout,
                        render_server.surface_config.format,
                        Some(Texture::DEPTH_FORMAT),
                        &[Vertex3d::desc(), InstanceRaw::desc()],
                        shader,
                        "standard material pipeline",
                        false,
                        Some(wgpu::Face::Back),
                    )
                };

                self.pipeline_cache.insert(flags, pipeline);
            }
        }
    }

    pub fn get_pipeline(&self, material: &MaterialStandard) -> &wgpu::RenderPipeline {
        let flags = material.get_flags();

        self.pipeline_cache.get(&flags).unwrap()
    }

    /// Draw multiple models.
    pub(crate) fn prepare_instances(
        &mut self,
        render_server: &RenderServer,
        meshes: &Vec<ExtractedMesh>,
    ) {
        for mesh in meshes {
            let metadata = self.instance_cache.get(&mesh.mesh_id);

            if (metadata.is_none()) {
                // Create the instance buffer.
                let instance_buffer = render_server.device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("model instance buffer"),
                    size: mem::size_of::<InstanceRaw>() as BufferAddress,
                    usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                });

                self.instance_cache.insert(
                    mesh.mesh_id,
                    InstanceMetadata {
                        buffer: instance_buffer,
                        instance_count: 1,
                    },
                );
            }

            let metadata = self.instance_cache.get(&mesh.mesh_id).unwrap();

            let mut instances = vec![];

            let transform = &mesh.transform;

            instances.push(Instance {
                position: transform.position,
                rotation: transform.rotation,
            });

            // Copy data from [Instance] to [InstanceRaw].
            let instance_data = instances.iter().map(Instance::to_raw).collect::<Vec<_>>();

            // Update buffer.
            render_server.queue.write_buffer(
                &metadata.buffer,
                0,
                bytemuck::cast_slice(&instance_data[..]),
            );
        }
    }
}

pub(crate) fn prepare_meshes(
    extracted_meshes: &Vec<ExtractedMesh>,
    extracted_lights: &Vec<LightUniform>,
    texture_cache: &TextureCache,
    shader_maker: &mut ShaderMaker,
    mesh_render_resources: &mut MeshRenderResources,
    camera_render_resources: &CameraRenderResources,
    render_server: &RenderServer,
) {
    //
    // // Copy data from [Instance] to [InstanceRaw].
    // let instance_data = instances.iter().map(Instance::to_raw).collect::<Vec<_>>();
    //
    // // Create the instance buffer.
    // let instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
    //     label: Some("model instance buffer"),
    //     contents: bytemuck::cast_slice(&instance_data),
    //     usage: wgpu::BufferUsages::VERTEX,
    // });

    for mesh in extracted_meshes {
        mesh_render_resources.prepare_materials(&texture_cache, render_server);

        mesh_render_resources.prepare_pipeline(
            render_server,
            shader_maker,
            &camera_render_resources.bind_group_layout,
            mesh.material_id,
        );
    }

    for light in extracted_lights {
        mesh_render_resources.prepare_lights(render_server, *light);
    }

    mesh_render_resources.prepare_instances(render_server, &extracted_meshes);
}

pub(crate) fn render_meshes<'a, 'b: 'a>(
    extracted_meshes: &'b Vec<ExtractedMesh>,
    mesh_cache: &'b MeshCache,
    mesh_render_resources: &'b MeshRenderResources,
    camera_render_resources: &'b CameraRenderResources,
    gizmo_render_resources: &'b GizmoRenderResources,
    render_pass: &mut wgpu::RenderPass<'a>,
) {
    if (camera_render_resources.bind_group.is_none()) {
        return;
    }
    if (mesh_render_resources.light_bind_group.is_none()) {
        return;
    }

    let camera_bind_group = camera_render_resources.bind_group.as_ref().unwrap();

    let light_bind_group = mesh_render_resources.light_bind_group.as_ref().unwrap();

    for extracted in extracted_meshes {
        let mut texture_bind_group = None;
        let mut flags = 0;

        if (extracted.material_id.is_some()) {
            let material_id = &extracted.material_id.unwrap();

            texture_bind_group = Some(
                mesh_render_resources
                    .texture_bind_group_cache
                    .get(material_id)
                    .unwrap(),
            );

            let material = mesh_render_resources
                .material_cache
                .get(material_id)
                .unwrap();
            flags = material.get_flags();
        }

        let pipeline = mesh_render_resources.pipeline_cache.get(&flags).unwrap();

        let mesh = mesh_cache.get(extracted.mesh_id).unwrap();

        let instance = mesh_render_resources
            .instance_cache
            .get(&extracted.mesh_id)
            .unwrap();

        render_pass.set_pipeline(pipeline);
        // Set vertex buffer for InstanceInput.
        render_pass.set_vertex_buffer(1, instance.buffer.slice(..));

        // Set vertex buffer for VertexInput.
        render_pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));

        render_pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);

        // FIXME
        // Set camera uniform.
        render_pass.set_bind_group(0, camera_bind_group, &[0]);

        // Set light uniform.
        render_pass.set_bind_group(1, light_bind_group, &[]);

        // Set textures.
        if (texture_bind_group.is_some()) {
            render_pass.set_bind_group(2, texture_bind_group.unwrap(), &[]);
        }

        render_pass.draw_indexed(0..mesh.index_count, 0, 0..1);
    }

    gizmo_render_resources.render(
        render_pass,
        camera_render_resources.bind_group.as_ref().unwrap(),
    );
}
