use crate::render::material::{MaterialCache, MaterialId, MaterialStandard, MaterialUniform};
use crate::render::mesh_allocator::MeshAllocator;
use crate::render::render_graph::RenderGraph;
use crate::render::render_world::{Extracted, RenderWorld};
use crate::render::sky::{prepare_sky, SkyImportedResources};
use crate::render::sprite::ExtractedSprite2d;
use crate::render::{
    ExtractedMesh, Instance, InstanceRaw, MeshCache, MeshId, MeshInstanceInfo, MeshMetadata,
    RenderContext, TextureCache, TextureId,
};
use crate::scene::Bvh;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

pub struct PreparedFrame {
    pub(crate) extracted: Extracted,
    // Prepare materials ---------
    pub(crate) texture_index_map: HashMap<TextureId, u32>,
    pub(crate) material_index_map: HashMap<MaterialId, u32>,
    pub(crate) material_uniforms: Vec<MaterialUniform>,
    pub(crate) bindless_texture_ids: Vec<TextureId>,
    // ---------------------------
    // Mesh -------------------
    pub(crate) opaque_meshes: Vec<ExtractedMesh>,
    pub(crate) transparent_meshes: Vec<ExtractedMesh>,
    pub(crate) bvh: Bvh,
    // ---------------------------
    // Mesh instances -------------------
    pub(crate) all_instances: Vec<InstanceRaw>,
    pub(crate) mesh_id_to_index: HashMap<MeshId, u32>,
    pub(crate) draw_counts: Vec<u32>,
    pub(crate) mesh_infos: Vec<MeshInstanceInfo>,
    pub(crate) mesh_metadatas: Vec<MeshMetadata>,
    pub(crate) indirect_commands: Vec<wgpu::util::DrawIndexedIndirectArgs>,
    pub(crate) instance_buffer_size: usize,
    pub(crate) indirect_buffer_size: usize,
    // ---------------------------
}

pub enum RenderCommand {
    Render(Extracted),
    Resize(u32, u32),
}

/// 运行在独立线程的渲染后端
pub struct RenderBackend {
    pub(crate) surface: wgpu::Surface<'static>,
    /// 渲染图，各帧共享
    pub(crate) render_graph: RenderGraph,
    // 保存一些各帧共享的资源
    pub(crate) dummy_2d_texture: Arc<wgpu::Texture>,
    pub(crate) dummy_2d_view: wgpu::TextureView,
    pub(crate) dummy_cube_texture: Arc<wgpu::Texture>,
    pub(crate) dummy_cube_view: wgpu::TextureView,
    // 共享缓存
    pub(crate) imported_texture_cache: Arc<RwLock<TextureCache>>,
    pub(crate) imported_material_cache: Arc<RwLock<MaterialCache>>,
    pub(crate) sky_imported_resources: SkyImportedResources,
    pub(crate) imported_mesh_cache: Arc<RwLock<MeshCache>>,
    pub(crate) imported_mesh_allocator: Arc<RwLock<MeshAllocator>>,
    /// 持久化存在，各帧共享
    bind_group_layouts: HashMap<String, wgpu::BindGroupLayout>,
    pipeline_layouts: HashMap<String, wgpu::PipelineLayout>,
}

impl RenderBackend {
    pub fn new(
        render_server: &RenderContext,
        surface: wgpu::Surface<'static>,
        imported_texture_cache: Arc<RwLock<TextureCache>>,
        imported_mesh_cache: Arc<RwLock<MeshCache>>,
        imported_material_cache: Arc<RwLock<MaterialCache>>,
        imported_mesh_allocator: Arc<RwLock<MeshAllocator>>,
    ) -> Self {
        let sky_imported_resources = SkyImportedResources::new();
        
        let dummy_2d_texture = render_server
            .device
            .create_texture(&wgpu::TextureDescriptor {
                label: Some("dummy 2d texture"),
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
        
        let dummy_cube_texture = render_server
            .device
            .create_texture(&wgpu::TextureDescriptor {
                label: Some("dummy cube texture"),
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

        let dummy_2d_view = dummy_2d_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let dummy_cube_view = dummy_cube_texture.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::Cube),
            ..Default::default()
        });

        let mut render_graph = RenderGraph::new();
        render_graph.setup_standard_nodes();

        Self {
            surface,
            dummy_2d_texture: Arc::new(dummy_2d_texture),
            dummy_2d_view,
            dummy_cube_texture: Arc::new(dummy_cube_texture),
            dummy_cube_view,
            render_graph,
            sky_imported_resources,
            imported_texture_cache,
            imported_material_cache,
            imported_mesh_cache,
            imported_mesh_allocator,
            bind_group_layouts: Default::default(),
            pipeline_layouts: Default::default(),
        }
    }

    pub fn run(
        &mut self,
        render_context: &RenderContext,
        mut extracted: Extracted,
    ) {
        let surface_texture = match self.surface.get_current_texture() {
            Ok(texture) => texture,
            Err(e) => {
                eprintln!("Failed to acquire next surface texture: {:?}", e);
                return;
            }
        };

        let final_output_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        // 1. Prepare frame data
        let prepared_frame = self.prepare(render_context, &mut extracted);

        // 2. Run graph and record render commands
        let mut graph = std::mem::take(&mut self.render_graph);
        let cmd_buf = graph.run(render_context, self, &prepared_frame, &final_output_view);
        self.render_graph = graph;

        // 3. Submit render commands
        render_context.queue.submit(std::iter::once(cmd_buf));

        // 4. Present (and do v-sync)
        surface_texture.present();
    }

    fn prepare(
        &mut self,
        render_server: &RenderContext,
        extracted: &mut Extracted,
    ) -> PreparedFrame {
        self.setup_layouts(render_server);

        // 3. Prepare Bindless Materials (Includes all 2D textures)
        let (texture_index_map, material_index_map, material_uniforms, bindless_texture_ids) =
            self.prepare_materials(&extracted.sprites);

        // Separate opaque and transparent meshes
        let mut opaque_meshes = Vec::new();
        let mut transparent_meshes = Vec::new();

        {
            let material_cache = self.imported_material_cache.read().unwrap();
            for mesh in &extracted.meshes {
                let is_transparent = if let Some(material_id) = mesh.material_id {
                    if let Some(material) = material_cache.get(&material_id) {
                        material.transparent || mesh.transparent
                    } else {
                        mesh.transparent
                    }
                } else {
                    mesh.transparent
                };

                if is_transparent {
                    transparent_meshes.push(*mesh);
                } else {
                    opaque_meshes.push(*mesh);
                }
            }
        }

        // Prepare 3D mesh BVH (only for opaque meshes)
        let mesh_cache = self.imported_mesh_cache.read().unwrap();
        let opaque_bvh = if !opaque_meshes.is_empty() {
            let bvh_objects: Vec<_> = opaque_meshes
                .iter()
                .enumerate()
                .filter_map(|(i, ext)| {
                    mesh_cache
                        .get(ext.mesh_id)
                        .map(|mesh| (mesh.aabb.transform(&ext.transform), i))
                })
                .collect();

            Bvh::build(bvh_objects)
        } else {
            Bvh::default()
        };

        // Prepare 3D meshes & lights (combine opaque and transparent for instance preparation)
        let all_meshes: Vec<_> = opaque_meshes
            .iter()
            .chain(transparent_meshes.iter())
            .cloned()
            .collect();

        let (
            all_instances,
            mesh_id_to_index,
            draw_counts,
            mesh_infos,
            mesh_metadatas,
            indirect_commands,
            instance_buffer_size,
            indirect_buffer_size,
        ) = if !all_meshes.is_empty() {
            self.prepare_instances(&all_meshes, &mesh_cache, &material_index_map)
        } else {
            (
                vec![],
                Default::default(),
                vec![],
                vec![],
                vec![],
                vec![],
                0,
                0,
            )
        };
        // Drop mesh_cache here
        drop(mesh_cache);

        // 6.5 Re-include MASKED transparent meshes for SSAO (normal pre-pass)
        let mut ssao_meshes = opaque_meshes.clone();
        {
            let material_cache = self.imported_material_cache.read().unwrap();
            for mesh in &transparent_meshes {
                if let Some(mat_id) = mesh.material_id {
                    if let Some(mat) = material_cache.get(&mat_id) {
                        if mat.alpha_mode == crate::render::material::AlphaMode::Mask {
                            ssao_meshes.push(*mesh);
                        }
                    }
                }
            }
        }

        // 准备天空盒的永驻资源
        if let Some(sky) = &extracted.sky {
            prepare_sky(
                &mut self.sky_imported_resources,
                render_server,
                &sky.texture,
            );
        }

        PreparedFrame {
            extracted: extracted.clone(),
            texture_index_map,
            material_index_map,
            material_uniforms,
            bindless_texture_ids,
            opaque_meshes: ssao_meshes,
            transparent_meshes,
            bvh: opaque_bvh,
            all_instances,
            mesh_id_to_index,
            draw_counts,
            mesh_infos,
            mesh_metadatas,
            indirect_commands,
            instance_buffer_size,
            indirect_buffer_size,
        }
    }

    pub fn prepare_materials(
        &mut self,
        extracted_sprites_2d: &Vec<ExtractedSprite2d>,
    ) -> (
        HashMap<TextureId, u32>,
        HashMap<MaterialId, u32>,
        Vec<MaterialUniform>,
        Vec<TextureId>,
    ) {
        let imported_texture_cache = self.imported_texture_cache.read().unwrap();
        let imported_material_cache = self.imported_material_cache.read().unwrap();

        // 关键：我们不再每一帧都重建 Map。
        // 我们只在必要时清空并重建，或者采用增量方式。
        // 为了目前最稳妥的修复闪烁，我们先清空但确保 3D 材质的顺序是绝对固定的。
        let mut texture_index_map: HashMap<TextureId, u32> = HashMap::new();

        let mut bindless_texture_ids = Vec::new();

        // 1. 搜集材质纹理 (这部分顺序通过 sorted_materials 保证绝对固定)
        let mut sorted_materials: Vec<_> = imported_material_cache.storage.iter().collect();

        sorted_materials.sort_by(|(id1, _), (id2, _)| id1.0.cmp(&id2.0));

        for (_, material) in &sorted_materials {
            for id in [
                material.color_texture,
                material.normal_texture,
                material.metallic_roughness_texture,
                material.occlusion_texture,
            ]
            .into_iter()
            .flatten()
            {
                if !texture_index_map.contains_key(&id) {
                    if let Some(_texture) = imported_texture_cache.get(id) {
                        texture_index_map.insert(id, bindless_texture_ids.len() as u32);
                        bindless_texture_ids.push(id);
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
            if !texture_index_map.contains_key(&id) {
                if let Some(_texture) = imported_texture_cache.get(id) {
                    texture_index_map.insert(id, bindless_texture_ids.len() as u32);
                    bindless_texture_ids.push(id);
                }
            }
        }

        let mut material_index_map: HashMap<MaterialId, u32> = HashMap::new();

        // 准备材质 uniforms
        let mut material_uniforms = Vec::new();

        for (id, material) in &sorted_materials {
            material_index_map.insert(**id, material_uniforms.len() as u32);
            material_uniforms.push(material.to_uniform(&texture_index_map));
        }

        // 没有材质，推入一个 dummy 材质
        if material_uniforms.is_empty() {
            material_uniforms.push(MaterialStandard::new("dummy").to_uniform(&HashMap::new()));
        }

        (
            texture_index_map,
            material_index_map,
            material_uniforms,
            bindless_texture_ids,
        )
    }

    // 准备网格
    pub(crate) fn prepare_instances(
        &self,
        extracted_meshes: &Vec<ExtractedMesh>,
        mesh_cache: &MeshCache,
        material_index_map: &HashMap<MaterialId, u32>,
    ) -> (
        Vec<InstanceRaw>,
        HashMap<MeshId, u32>,
        Vec<u32>,
        Vec<MeshInstanceInfo>,
        Vec<MeshMetadata>,
        Vec<wgpu::util::DrawIndexedIndirectArgs>,
        usize,
        usize,
    ) {
        let mut grouped_instances: HashMap<MeshId, Vec<InstanceRaw>> = HashMap::new();

        for mesh in extracted_meshes {
            let material_idx = mesh
                .material_id
                .and_then(|id| material_index_map.get(&id))
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

        let mut mesh_id_to_index: HashMap<MeshId, u32> = HashMap::new();
        let mut mesh_infos: Vec<MeshInstanceInfo> = Vec::new();

        let mut current_base_instance = 0u32;
        let mut sorted_meshes: Vec<_> = grouped_instances.keys().cloned().collect();
        sorted_meshes.sort_by_key(|id| id.0);

        for mesh_id in sorted_meshes {
            let instances = &grouped_instances[&mesh_id];
            let mesh = mesh_cache.get(mesh_id).unwrap();

            mesh_id_to_index.insert(mesh_id, mesh_metadatas.len() as u32);

            mesh_infos.push(MeshInstanceInfo {
                mesh_id,
                base_instance: current_base_instance,
                instance_count: instances.len() as u32,
            });

            mesh_metadatas.push(crate::render::mesh::MeshMetadata {
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

        let instance_buffer_size = all_instances.len() * size_of::<InstanceRaw>();
        let indirect_buffer_size = indirect_commands.len() * 20;
        let draw_counts = vec![indirect_commands.len() as u32];

        (
            all_instances,
            mesh_id_to_index,
            draw_counts,
            mesh_infos,
            mesh_metadatas,
            indirect_commands,
            instance_buffer_size,
            indirect_buffer_size,
        )
    }

    pub(crate) fn setup_layouts(&mut self, render_context: &RenderContext) {
        if self
            .get_bind_group_layout("bindless_bind_group_layout")
            .is_some()
        {
            return;
        }

        let bindless_bind_group_layout =
            render_context
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

        self.add_bind_group_layout("bindless_bind_group_layout", bindless_bind_group_layout);
    }

    // 固定资源存取

    pub fn add_bind_group_layout(
        &mut self,
        name: impl Into<String>,
        layout: wgpu::BindGroupLayout,
    ) {
        self.bind_group_layouts.insert(name.into(), layout);
    }

    pub fn get_bind_group_layout(&self, name: &str) -> Option<&wgpu::BindGroupLayout> {
        self.bind_group_layouts.get(name)
    }

    pub fn add_pipeline_layout(&mut self, name: impl Into<String>, layout: wgpu::PipelineLayout) {
        self.pipeline_layouts.insert(name.into(), layout);
    }

    pub fn get_pipeline_layout(&self, name: &str) -> Option<&wgpu::PipelineLayout> {
        self.pipeline_layouts.get(name)
    }
}
