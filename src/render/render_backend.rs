use crate::render::material::{MaterialCache, MaterialId, MaterialStandard, MaterialUniform};
use crate::render::mesh_allocator::MeshAllocator;
use crate::render::render_graph::{
    AntiAliasingMode, RenderGraph, RenderGraphConfig, SsaoConfig, SsgiConfig, SsrConfig,
    VolumetricConfig,
};
use crate::render::render_world::Extracted;
use crate::render::sky::{prepare_sky, SkyImportedResources};
use crate::render::sprite::ExtractedSprite2d;
use crate::render::{
    ExtractedMesh, Instance, InstanceRaw, MeshCache, MeshId, MeshInstanceInfo, MeshMetadata,
    RenderContext, TextureCache, TextureId,
};
use crate::scene::Bvh;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use crate::render::egui_debug::EguiBackendRenderer;

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
    // Opaque Mesh instances (GPU Culling Path) -------------------
    pub(crate) all_instances: Vec<InstanceRaw>,
    pub(crate) mesh_id_to_index: HashMap<MeshId, u32>,
    pub(crate) draw_counts: Vec<u32>,
    pub(crate) mesh_infos: Vec<MeshInstanceInfo>,
    pub(crate) mesh_metadatas: Vec<MeshMetadata>,
    pub(crate) indirect_commands: Vec<wgpu::util::DrawIndexedIndirectArgs>,
    pub(crate) instance_buffer_size: usize,
    pub(crate) indirect_buffer_size: usize,
    // ---------------------------
    // Transparent Mesh instances (CPU Sorted Path) --------------
    pub(crate) sorted_transparent_instances: Vec<InstanceRaw>,
    pub(crate) transparent_draw_batches: Vec<TransparentBatch>,
    // ---------------------------
}

pub struct TransparentBatch {
    pub mesh_id: MeshId,
    pub instance_range: std::ops::Range<u32>,
}

pub enum RenderCommand {
    Render(Extracted, Vec<egui::ClippedPrimitive>, egui::TexturesDelta, f32),
    Resize(u32, u32),
    UpdateConfig(RenderGraphConfig),
}

/// 运行在独立线程的渲染后端
pub struct RenderBackend {
    pub(crate) surface: wgpu::Surface<'static>,
    pub(crate) egui_backend_renderer: Option<EguiBackendRenderer>,
    /// 当前启用的渲染图配置
    pub(crate) active_config: RenderGraphConfig,
    /// 渲染图，各帧共享
    pub(crate) render_graph: RenderGraph,
    // 保存一些各帧共享的资源
    pub(crate) dummy_2d_texture: Arc<wgpu::Texture>,
    pub(crate) dummy_2d_view: wgpu::TextureView,
    pub(crate) dummy_cube_texture: Arc<wgpu::Texture>,
    pub(crate) dummy_cube_view: wgpu::TextureView,
    pub(crate) dummy_3d_texture: Arc<wgpu::Texture>,
    pub(crate) dummy_3d_view: wgpu::TextureView,
    // 共享缓存
    pub(crate) imported_texture_cache: Arc<RwLock<TextureCache>>,
    pub(crate) imported_material_cache: Arc<RwLock<MaterialCache>>,
    pub(crate) sky_imported_resources: SkyImportedResources,
    pub(crate) imported_mesh_cache: Arc<RwLock<MeshCache>>,
    pub(crate) imported_mesh_allocator: Arc<RwLock<MeshAllocator>>,
    /// 持久化存在，各帧共享
    bind_group_layouts: HashMap<String, wgpu::BindGroupLayout>,
    pipeline_layouts: HashMap<String, wgpu::PipelineLayout>,

    // GPU Profiling (Multi-buffered)
    timestamp_query_set: Option<wgpu::QuerySet>,
    timestamp_resolve_buffer: Option<wgpu::Buffer>,
    timestamp_destination_buffers: Vec<wgpu::Buffer>,
    timestamp_mapped_flags: Vec<Arc<std::sync::atomic::AtomicBool>>,
    timestamp_active: Vec<bool>, // 新增：追踪缓冲区是否正在被 GPU 或 CPU 使用
    current_timestamp_index: usize,
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
        let surface_format = render_server.surface_config.format;
        let egui_backend_renderer = Some(EguiBackendRenderer::new(&render_server.device, surface_format));
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

        let dummy_3d_texture = render_server
            .device
            .create_texture(&wgpu::TextureDescriptor {
                label: Some("dummy 3d texture"),
                size: wgpu::Extent3d {
                    width: 1,
                    height: 1,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D3,
                format: wgpu::TextureFormat::Rgba16Float,
                usage: wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });

        let dummy_2d_view = dummy_2d_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let dummy_cube_view = dummy_cube_texture.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::Cube),
            ..Default::default()
        });
        let dummy_3d_view = dummy_3d_texture.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::D3),
            ..Default::default()
        });

        let active_config = RenderGraphConfig::default();
        let render_graph = RenderGraph::build_from_config(active_config.clone());

        // --- GPU Profiling Setup ---
        let mut timestamp_query_set = None;
        let mut timestamp_resolve_buffer = None;
        let mut timestamp_destination_buffers = Vec::new();
        let mut timestamp_mapped_flags = Vec::new();
        let mut timestamp_active = Vec::new();

        let has_basic_query = render_server
            .device
            .features()
            .contains(wgpu::Features::TIMESTAMP_QUERY);
        let has_encoder_query = render_server
            .device
            .features()
            .contains(wgpu::Features::TIMESTAMP_QUERY_INSIDE_ENCODERS);

        if has_basic_query && has_encoder_query {
            log::info!("GPU Profiling: Enabled (using hardware timestamps)");
            timestamp_query_set = Some(render_server.device.create_query_set(
                &wgpu::QuerySetDescriptor {
                    label: Some("timestamp query set"),
                    count: 2,
                    ty: wgpu::QueryType::Timestamp,
                },
            ));

            timestamp_resolve_buffer =
                Some(render_server.device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("timestamp resolve buffer"),
                    size: 16,
                    usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
                    mapped_at_creation: false,
                }));

            for i in 0..render_server.frames_in_flight {
                timestamp_destination_buffers.push(render_server.device.create_buffer(
                    &wgpu::BufferDescriptor {
                        label: Some(&format!("timestamp destination buffer {}", i)),
                        size: 16,
                        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                        mapped_at_creation: false,
                    },
                ));
                timestamp_mapped_flags.push(Arc::new(std::sync::atomic::AtomicBool::new(false)));
                timestamp_active.push(false);
            }
        } else {
            log::warn!("GPU Profiling: Disabled (hardware doesn't support TIMESTAMP_QUERY_INSIDE_ENCODERS)");
        }

        Self {
            surface,
            egui_backend_renderer,
            active_config,
            dummy_2d_texture: Arc::new(dummy_2d_texture),
            dummy_2d_view,
            dummy_cube_texture: Arc::new(dummy_cube_texture),
            dummy_cube_view,
            dummy_3d_texture: Arc::new(dummy_3d_texture),
            dummy_3d_view,
            render_graph,
            sky_imported_resources,
            imported_texture_cache,
            imported_material_cache,
            imported_mesh_cache,
            imported_mesh_allocator,
            bind_group_layouts: Default::default(),
            pipeline_layouts: Default::default(),
            timestamp_query_set,
            timestamp_resolve_buffer,
            timestamp_destination_buffers,
            timestamp_mapped_flags,
            timestamp_active,
            current_timestamp_index: 0,
        }
    }

    /// 动态更新渲染图配置。若配置发生变更，则按新配置重新组装拓扑结构。
    pub fn set_render_config(&mut self, config: RenderGraphConfig) {
        if self.active_config != config {
            log::info!("Rebuilding RenderGraph with updated config: {:?}", config);
            self.render_graph = RenderGraph::build_from_config(config.clone());
            self.active_config = config;
        }
    }

    pub fn run(
        &mut self,
        render_context: &RenderContext,
        mut extracted: Extracted,
        tris: Vec<egui::ClippedPrimitive>,
        textures_delta: egui::TexturesDelta,
        scale_factor: f32,
    ) {
        if let Some(renderer) = &mut self.egui_backend_renderer {
            renderer.receive_textures_delta(textures_delta);
        }

        let cpu_render_start = std::time::Instant::now();

        // 动态根据 extracted / camera 状态同步渲染图配置
        let has_volumetric = extracted.cameras.uniforms.iter().any(|u| u.volumetric_enabled != 0);
        let has_ssr = extracted.cameras.uniforms.iter().any(|u| u.ssr_enabled != 0);
        let has_ssgi = extracted.cameras.uniforms.iter().any(|u| u.ssgi_enabled != 0);

        let new_config = RenderGraphConfig {
            shadow: self.active_config.shadow.clone(),
            ssao: SsaoConfig {
                enabled: extracted.ssao_enabled,
                ..self.active_config.ssao
            },
            ssgi: SsgiConfig {
                enabled: has_ssgi,
                ..self.active_config.ssgi
            },
            ssr: SsrConfig {
                enabled: has_ssr,
                ..self.active_config.ssr
            },
            volumetric: VolumetricConfig {
                enabled: has_volumetric,
                ..self.active_config.volumetric
            },
            bloom: self.active_config.bloom.clone(),
            aa_mode: if extracted.taa_enabled {
                AntiAliasingMode::Taa
            } else if extracted.fxaa_enabled {
                AntiAliasingMode::Fxaa
            } else {
                AntiAliasingMode::None
            },
            tonemapping: self.active_config.tonemapping.clone(),
        };

        self.set_render_config(new_config);

        // 处理旧数据并释放缓冲区
        self.process_timestamps(render_context);

        let surface_texture = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(texture) => texture,
            wgpu::CurrentSurfaceTexture::Suboptimal(texture) => {
                // 不要在这里调用 configure，因为 texture 还没有被释放。
                // 次优状态下依然可以渲染，配置留给专门的 Resize 指令即可。
                texture
            }
            wgpu::CurrentSurfaceTexture::Lost | wgpu::CurrentSurfaceTexture::Outdated => {
                self.surface.configure(&render_context.device, &render_context.surface_config);
                return;
            }
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                return;
            }
            wgpu::CurrentSurfaceTexture::Validation => {
                log::error!("WGPU Surface Validation Error");
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

        // 3. Render Egui Debug UI
        let egui_cmd_bufs = if let Some(renderer) = &mut self.egui_backend_renderer {
            Some(renderer.render(
                &render_context.device,
                &render_context.queue,
                &final_output_view,
                (surface_texture.texture.width(), surface_texture.texture.height()),
                scale_factor,
                &tris,
            ))
        } else {
            None
        };

        // 记录 CPU Render Time (仅包含命令录制和数据准备，不包含 GPU 等待)
        render_context.render_cpu_time.store(
            cpu_render_start.elapsed().as_nanos() as u64,
            std::sync::atomic::Ordering::Relaxed,
        );

        // --- 提交渲染工作和时间戳解析 ---
        let mut submission = Vec::new();
        let mut timestamp_recorded = false;

        if let (Some(query_set), Some(resolve_buf)) =
            (&self.timestamp_query_set, &self.timestamp_resolve_buffer)
        {
            // 关键修复：只有当缓冲区不处于 Active (映射中) 时才使用它
            if !self.timestamp_active[self.current_timestamp_index] {
                // 1. 创建起始时间戳
                let mut start_encoder =
                    render_context
                        .device
                        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                            label: Some("GPU Start Timer"),
                        });
                start_encoder.write_timestamp(query_set, 0);
                submission.push(start_encoder.finish());

                // 2. 加入主渲染任务
                submission.push(cmd_buf);
                if let Some(mut egui_bufs) = egui_cmd_bufs {
                    submission.append(&mut egui_bufs);
                }

                // 3. 创建结束时间戳并解析
                let mut end_encoder =
                    render_context
                        .device
                        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                            label: Some("GPU End Timer"),
                        });
                end_encoder.write_timestamp(query_set, 1);

                let dest_buf = &self.timestamp_destination_buffers[self.current_timestamp_index];
                end_encoder.resolve_query_set(query_set, 0..2, resolve_buf, 0);
                end_encoder.copy_buffer_to_buffer(resolve_buf, 0, dest_buf, 0, 16);
                submission.push(end_encoder.finish());

                self.timestamp_active[self.current_timestamp_index] = true;
                timestamp_recorded = true;
            } else {
                submission.push(cmd_buf);
                if let Some(mut egui_bufs) = egui_cmd_bufs {
                    submission.append(&mut egui_bufs);
                }
            }
        } else {
            submission.push(cmd_buf);
            if let Some(mut egui_bufs) = egui_cmd_bufs {
                submission.append(&mut egui_bufs);
            }
        }

        render_context.queue.submit(submission);
        surface_texture.present();

        if timestamp_recorded {
            let dest_buf = &self.timestamp_destination_buffers[self.current_timestamp_index];
            let flag = self.timestamp_mapped_flags[self.current_timestamp_index].clone();

            dest_buf
                .slice(..)
                .map_async(wgpu::MapMode::Read, move |result| {
                    if result.is_ok() {
                        flag.store(true, std::sync::atomic::Ordering::Release);
                    }
                });

            self.current_timestamp_index =
                (self.current_timestamp_index + 1) % self.timestamp_destination_buffers.len();
        }

        // 必须调用 poll(Poll) 来推进异步映射的进度，但这不会阻塞线程
        let _ = render_context.device.poll(wgpu::PollType::Poll);
    }

    fn process_timestamps(&mut self, render_context: &RenderContext) {
        // 检查所有缓冲区，看看哪个已经准备好读取了
        for (i, buffer) in self.timestamp_destination_buffers.iter().enumerate() {
            let flag = &self.timestamp_mapped_flags[i];

            if flag.load(std::sync::atomic::Ordering::Acquire) {
                let slice = buffer.slice(..);
                {
                    let data = slice.get_mapped_range();
                    let timestamps: &[u64] = bytemuck::cast_slice(&data[..]);
                    if timestamps.len() >= 2 {
                        let diff = timestamps[1].wrapping_sub(timestamps[0]);
                        let period = render_context.queue.get_timestamp_period();
                        let gpu_nanos = diff as f64 * period as f64;
                        render_context
                            .gpu_time
                            .store(gpu_nanos as u64, std::sync::atomic::Ordering::Relaxed);
                    }
                }
                buffer.unmap();
                flag.store(false, std::sync::atomic::Ordering::Release);
                self.timestamp_active[i] = false; // 释放标志，该缓冲区现在可以再次被 GPU 使用
            }
        }
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
                    material_cache.get(&material_id).map_or(false, |m| m.transparent)
                } else {
                    false
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

        // 1. Prepare Opaque Instances (GPU Culling Path)
        let (
            all_instances,
            mesh_id_to_index,
            draw_counts,
            mesh_infos,
            mesh_metadatas,
            indirect_commands,
            instance_buffer_size,
            indirect_buffer_size,
        ) = if !opaque_meshes.is_empty() {
            self.prepare_instances(&opaque_meshes, &mesh_cache, &material_index_map)
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

        // 2. Prepare Transparent Instances (CPU Sorted Path + Simple Frustum Culling)
        let mut sorted_transparent_instances = Vec::new();
        let mut transparent_draw_batches: Vec<TransparentBatch> = Vec::new();

        if !transparent_meshes.is_empty() {
            // A. 获取主相机视角和视锥体
            let (view_pos, frustum) = extracted
                .cameras
                .uniforms
                .iter()
                .enumerate()
                .find(|(i, _)| extracted.cameras.types[*i] == crate::render::camera::CameraType::D3)
                .map(|(_, u)| {
                    let pos = glam::Vec3::from_slice(&u.view_position[0..3]);
                    let vp = glam::Mat4::from_cols_array_2d(&u.view_proj);
                    (pos, crate::math::frustum::Frustum::from_view_proj(vp))
                })
                .unwrap_or((
                    glam::Vec3::ZERO,
                    crate::math::frustum::Frustum::from_view_proj(glam::Mat4::IDENTITY),
                ));

            // B. 直接线性过滤可见物体 (不需要 BVH)
            let mut visible_transparent: Vec<_> = transparent_meshes
                .iter()
                .filter(|mesh| {
                    if let Some(m) = mesh_cache.get(mesh.mesh_id) {
                        frustum.intersects_aabb(&m.aabb.transform(&mesh.transform))
                    } else {
                        false
                    }
                })
                .cloned()
                .collect();

            // C. 按从远到近排序
            visible_transparent.sort_by(|a, b| {
                let dist_a = a.transform.position.distance_squared(view_pos);
                let dist_b = b.transform.position.distance_squared(view_pos);
                dist_b
                    .partial_cmp(&dist_a)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });

            // D. 生成实例数据
            for mesh in visible_transparent {
                let material_idx = mesh
                    .material_id
                    .and_then(|id| material_index_map.get(&id))
                    .cloned()
                    .unwrap_or(0);
                let instance = crate::render::mesh::Instance {
                    position: mesh.transform.position,
                    scale: mesh.transform.scale,
                    rotation: mesh.transform.rotation,
                    material_idx,
                }
                .to_raw();

                let current_idx = sorted_transparent_instances.len() as u32;
                sorted_transparent_instances.push(instance);

                if let Some(last) = transparent_draw_batches.last_mut() {
                    if last.mesh_id == mesh.mesh_id {
                        last.instance_range.end += 1;
                        continue;
                    }
                }

                transparent_draw_batches.push(TransparentBatch {
                    mesh_id: mesh.mesh_id,
                    instance_range: current_idx..current_idx + 1,
                });
            }
        }

        drop(mesh_cache);

        // 6.5 Re-include MASKED transparent meshes for SSAO (normal pre-pass)
        let mut ssao_meshes = opaque_meshes.clone();
        // ... (SSAO 逻辑保持不变)
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
            sorted_transparent_instances,
            transparent_draw_batches,
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
                material.emissive_texture,
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
                _pad0: 0,
                _pad1: 0,
                cone_axis_cutoff: [0.0, 0.0, 0.0, -1.0],
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
