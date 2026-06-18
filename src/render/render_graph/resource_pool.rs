use super::resource::{
    BindGroupKey, BufferKey, PooledBuffer, ResourceKey, SamplerKey, TextureKey, VirtualResource,
};
use crate::render::{Texture, NEXT_RESOURCE_ID, NEXT_VIEW_ID};
use std::collections::HashMap;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// 资源池回收阈值：如果资源超过 30 秒未被使用，则从池中释放
const MAX_UNUSED_SECONDS: u64 = 30;

/// 瞬时资源池，用于在帧内复用纹理和缓冲区，并支持多帧并行下的延迟回收。
///
/// 架构设计分为两个阶段：
/// 1. **Pending（待回收队列）**: 处理 GPU 同步。资源在此队列中等待 FIF (Frames In Flight) 周期完成，
///    确保 GPU 已完全处理完引用该资源的任务，实现物理层面的“数据隔离”。
/// 2. **Ready（就绪池）**: 处理逻辑复用。资源完成 FIF 冷却后进入此池，可被立即复用以减少分配开销。
///    在此阶段，资源受“时间淘汰机制”管理，超过阈值未被领用则会被裁剪以释放显存。
#[derive(Default)]
pub struct ResourcePool {
    /// “就绪”纹理池。
    /// 存储已完成 FIF 冷却、可安全复用的纹理。
    /// 元组包含: (纹理对象, 进入池子的时间戳)。
    /// 淘汰策略：基于时间 (MAX_UNUSED_SECONDS) 进行裁剪。
    textures: HashMap<TextureKey, Vec<(Texture, Instant)>>,

    /// “待回收”纹理队列（FIF 同步层）。
    /// 存储刚被节点释放，但 GPU 可能仍在读取/写入的纹理。
    /// 元组包含: (纹理对象, 资源键, 释放时的帧号)。
    /// 处理逻辑：只有 current_frame >= release_frame + frames_in_flight 时才转移到就绪池。
    pending_textures: Vec<(Texture, TextureKey, u64)>,

    /// “就绪”缓冲区池。
    /// 存储已完成 FIF 冷却、可安全复用的缓冲区。
    /// 包含对齐处理，旨在提高不同规格请求间的复用命中率。
    buffers: HashMap<BufferKey, Vec<(PooledBuffer, Instant)>>,

    /// “待回收”缓冲区队列（FIF 同步层）。
    /// 确保在多帧并行环境下，旧帧的缓冲区数据不会被新帧提前覆盖。
    pending_buffers: Vec<(PooledBuffer, BufferKey, u64)>,

    /// 统计信息：当前池中管理（含待回收）的所有缓冲区的总内存（字节）
    total_buffer_memory: u64,
    /// 统计信息：当前池中管理（含待回收）的所有纹理的总内存（字节）
    total_texture_memory: u64,

    /// 采样器永久缓存。
    /// 由于采样器状态有限且不可变，通常不进行裁剪，直接以其规格作为键进行全局复用。
    sampler_cache: HashMap<SamplerKey, wgpu::Sampler>,

    /// 帧内 BindGroup 缓存。
    /// 仅在当前帧有效，每帧开始时清空。通过资源 ID 组合快速查找，避免重复创建 BindGroup。
    bind_group_cache: HashMap<BindGroupKey, wgpu::BindGroup>,

    /// 持久化资源（Persistent）。
    /// 不参与瞬时复用逻辑，通常由用户手动管理其生命周期，常用于 SwapChain 或长期存在的 G-Buffer。
    persistent_resources: HashMap<String, (VirtualResource, ResourceKey)>,
}

impl ResourcePool {
    pub fn update(&mut self, current_frame: u64, frames_in_flight: u64) {
        let now = Instant::now();

        // 1. 回收待处理纹理到“就绪”池
        let mut i = 0;
        while i < self.pending_textures.len() {
            if current_frame >= self.pending_textures[i].2 + frames_in_flight {
                let (texture, key, _) = self.pending_textures.remove(i);
                self.textures
                    .entry(key)
                    .or_default()
                    .push((texture, now));
            } else {
                i += 1;
            }
        }

        // 2. 回收待处理缓冲区到“就绪”池
        let mut i = 0;
        while i < self.pending_buffers.len() {
            if current_frame >= self.pending_buffers[i].2 + frames_in_flight {
                let (buffer, key, _) = self.pending_buffers.remove(i);
                self.buffers
                    .entry(key)
                    .or_default()
                    .push((buffer, now));
            } else {
                i += 1;
            }
        }

        // 3. 资源裁剪 (Trimming)：清理长时间不使用的资源，每 128 帧执行一次
        if current_frame % 128 == 0 {
            self.trim_unused_resources();
        }
    }

    fn trim_unused_resources(&mut self) {
        let now = Instant::now();
        let mut trimmed_textures = 0;
        let mut freed_texture_mem = 0;
        let mut trimmed_buffers = 0;
        let mut freed_buffer_mem = 0;

        // 裁剪纹理
        for (key, list) in self.textures.iter_mut() {
            let old_len = list.len();
            list.retain(|(_, last_used)| now.duration_since(*last_used).as_secs() < MAX_UNUSED_SECONDS);
            let removed_count = old_len - list.len();
            if removed_count > 0 {
                let bytes = Self::estimate_texture_size(key) * removed_count as u64;
                freed_texture_mem += bytes;
                trimmed_textures += removed_count;
            }
        }

        if trimmed_textures > 0 {
            self.total_texture_memory = self.total_texture_memory.saturating_sub(freed_texture_mem);
            log::info!(
                "Trimmed {} unused textures, freed {:.3} MB. Current texture memory: {:.2} MB",
                trimmed_textures,
                freed_texture_mem as f64 / 1024.0 / 1024.0,
                self.total_texture_memory as f64 / 1024.0 / 1024.0
            );
        }

        // 裁剪缓冲区
        for (key, list) in self.buffers.iter_mut() {
            let old_len = list.len();
            list.retain(|(_, last_used)| now.duration_since(*last_used).as_secs() < MAX_UNUSED_SECONDS);
            let removed_count = old_len - list.len();
            if removed_count > 0 {
                let bytes = key.size * removed_count as u64;
                freed_buffer_mem += bytes;
                trimmed_buffers += removed_count;
            }
        }

        if trimmed_buffers > 0 {
            self.total_buffer_memory = self.total_buffer_memory.saturating_sub(freed_buffer_mem);
            log::info!(
                "Trimmed {} unused buffers, freed {:.3} MB. Current buffer memory: {:.2} MB",
                trimmed_buffers,
                freed_buffer_mem as f64 / 1024.0 / 1024.0,
                self.total_buffer_memory as f64 / 1024.0 / 1024.0
            );
        }
    }

    pub fn clear_bind_group_cache(&mut self) {
        self.bind_group_cache.clear();
    }

    // --- 纹理管理 ---

    pub fn acquire_texture(&mut self, device: &wgpu::Device, key: TextureKey) -> Texture {
        self.acquire_texture_internal(device, key, None)
    }

    pub fn acquire_persistent_texture(
        &mut self,
        device: &wgpu::Device,
        name: &str,
        key: TextureKey,
    ) -> Texture {
        if let Some((VirtualResource::Texture(t), old_key)) = self.persistent_resources.get(name) {
            if let ResourceKey::Texture(old_texture_key) = old_key {
                if old_texture_key != &key {
                    panic!(
                        "Resource Conflict: Persistent texture '{}' requested with different keys!\nOld: {:?}\nNew: {:?}",
                        name, old_texture_key, key
                    );
                }
            }
            return t.clone();
        }
        let t = self.acquire_texture_internal(device, key, Some(name));
        self.persistent_resources.insert(
            name.to_string(),
            (
                VirtualResource::Texture(t.clone()),
                ResourceKey::Texture(key),
            ),
        );
        t
    }

    fn acquire_texture_internal(
        &mut self,
        device: &wgpu::Device,
        key: TextureKey,
        persistent_name: Option<&str>,
    ) -> Texture {
        if persistent_name.is_none() {
            if let Some(textures) = self.textures.get_mut(&key) {
                if let Some((texture, _)) = textures.pop() {
                    return texture;
                }
            }
        }

        let label = persistent_name.unwrap_or("transient_texture");

        let wgpu_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d {
                width: key.width,
                height: key.height,
                depth_or_array_layers: key.layers,
            },
            mip_level_count: key.mip_levels,
            sample_count: 1,
            dimension: key.dimension,
            format: key.format.unwrap(),
            usage: key.usage,
            view_formats: &[],
        });

        let view = wgpu_texture.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(match key.dimension {
                wgpu::TextureDimension::D1 => wgpu::TextureViewDimension::D1,
                wgpu::TextureDimension::D2 => {
                    if key.layers > 1 {
                        wgpu::TextureViewDimension::D2Array
                    } else {
                        wgpu::TextureViewDimension::D2
                    }
                }
                wgpu::TextureDimension::D3 => wgpu::TextureViewDimension::D3,
            }),
            ..Default::default()
        });

        let estimated_size = Self::estimate_texture_size(&key);
        self.total_texture_memory += estimated_size;

        log::info!(
            "Allocated new texture [{}] (size: {}x{}, format: {:?}, estimated: {:.2} MB). Total texture memory: {:.2} MB",
            label,
            key.width,
            key.height,
            key.format,
            estimated_size as f64 / 1024.0 / 1024.0,
            self.total_texture_memory as f64 / 1024.0 / 1024.0
        );

        Texture {
            size: (key.width, key.height),
            texture: wgpu_texture,
            view,
            format: key.format.unwrap(),
            id: NEXT_RESOURCE_ID.fetch_add(1, Ordering::Relaxed),
            view_id: NEXT_VIEW_ID.fetch_add(1, Ordering::Relaxed),
            view_cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn estimate_texture_size(key: &TextureKey) -> u64 {
        let format = key.format.unwrap_or(wgpu::TextureFormat::Rgba8Unorm);
        let bpp = match format {
            wgpu::TextureFormat::R8Unorm => 1,
            wgpu::TextureFormat::Rg8Unorm => 2,
            wgpu::TextureFormat::Rgba8Unorm | wgpu::TextureFormat::Rgba8UnormSrgb => 4,
            wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb => 4,
            wgpu::TextureFormat::Rgba16Float => 8,
            wgpu::TextureFormat::R32Float => 4,
            wgpu::TextureFormat::Rg32Float => 8,
            wgpu::TextureFormat::Rgba32Float => 16,
            wgpu::TextureFormat::Depth32Float => 4,
            wgpu::TextureFormat::Depth24Plus => 4,
            wgpu::TextureFormat::Depth24PlusStencil8 => 4,
            _ => 4,
        };
        (key.width * key.height * key.layers) as u64 * bpp
    }

    pub fn release_texture_deferred(&mut self, key: TextureKey, texture: Texture, frame_id: u64) {
        self.pending_textures.push((texture, key, frame_id));
    }

    // --- 缓冲区管理 ---

    pub fn acquire_buffer(
        &mut self,
        device: &wgpu::Device,
        key: BufferKey,
    ) -> (PooledBuffer, BufferKey) {
        self.acquire_buffer_internal(device, key, None)
    }

    pub fn acquire_persistent_buffer(
        &mut self,
        device: &wgpu::Device,
        name: &str,
        key: BufferKey,
    ) -> PooledBuffer {
        if let Some((VirtualResource::Buffer(b), old_key)) = self.persistent_resources.get(name) {
            if let ResourceKey::Buffer(old_buffer_key) = old_key {
                if old_buffer_key != &key {
                    panic!(
                        "Resource Conflict: Persistent buffer '{}' requested with different keys!\nOld: {:?}\nNew: {:?}",
                        name, old_buffer_key, key
                    );
                }
            }
            return b.clone();
        }
        let (b, actual_key) = self.acquire_buffer_internal(device, key, Some(name));
        self.persistent_resources.insert(
            name.to_string(),
            (
                VirtualResource::Buffer(b.clone()),
                ResourceKey::Buffer(actual_key),
            ),
        );
        b
    }

    fn acquire_buffer_internal(
        &mut self,
        device: &wgpu::Device,
        key: BufferKey,
        persistent_name: Option<&str>,
    ) -> (PooledBuffer, BufferKey) {
        let mut search_key = key;
        // 对尺寸进行向上对齐以提高资源复用率
        search_key.size = if search_key.size <= 4096 {
            // 小缓冲区按 256 字节对齐 (通常满足 Uniform Buffer 对齐要求)
            (search_key.size + 255) & !255
        } else if search_key.size <= 1024 * 1024 {
            // 中等缓冲区按 64KB 对齐
            (search_key.size + 65535) & !65535
        } else {
            // 大缓冲区按 1MB 对齐
            (search_key.size + 1024 * 1024 - 1) & !(1024 * 1024 - 1)
        };

        if persistent_name.is_none() {
            // 尝试寻找一个大小足够且用法兼容的现有缓冲区
            let mut found_key = None;
            for (pool_key, buffers) in &mut self.buffers {
                if !buffers.is_empty()
                    && pool_key.size >= search_key.size
                    && pool_key.usage.contains(search_key.usage)
                {
                    found_key = Some(*pool_key);
                    break;
                }
            }

            if let Some(k) = found_key {
                let (buffer, _) = self.buffers.get_mut(&k).unwrap().pop().unwrap();
                return (buffer, k);
            }
        }

        let label = persistent_name.unwrap_or("transient_buffer");

        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            size: search_key.size,
            usage: search_key.usage | wgpu::BufferUsages::COPY_DST, // 强制包含 COPY_DST 方便写入
            mapped_at_creation: false,
        });

        self.total_buffer_memory += search_key.size;
        log::info!(
            "Allocated new buffer [{}] (size: {} bytes). Total buffer memory: {:.2} MB",
            label,
            search_key.size,
            self.total_buffer_memory as f64 / 1024.0 / 1024.0
        );

        (
            PooledBuffer {
                buffer,
                id: NEXT_RESOURCE_ID.fetch_add(1, Ordering::Relaxed),
            },
            search_key,
        )
    }

    pub fn release_buffer_deferred(&mut self, key: BufferKey, buffer: PooledBuffer, frame_id: u64) {
        self.pending_buffers.push((buffer, key, frame_id));
    }

    // --- 采样器管理 ---

    pub fn acquire_sampler(&mut self, device: &wgpu::Device, key: SamplerKey) -> wgpu::Sampler {
        self.sampler_cache
            .entry(key)
            .or_insert_with(|| {
                device.create_sampler(&wgpu::SamplerDescriptor {
                    address_mode_u: key.address_mode_u,
                    address_mode_v: key.address_mode_v,
                    address_mode_w: key.address_mode_w,
                    mag_filter: key.mag_filter,
                    min_filter: key.min_filter,
                    mipmap_filter: key.mipmap_filter,
                    compare: key.compare,
                    lod_min_clamp: key.lod_min_clamp,
                    lod_max_clamp: key.lod_max_clamp,
                    ..Default::default()
                })
            })
            .clone()
    }

    // --- BindGroup 管理 ---

    pub fn get_or_create_bind_group<F>(
        &mut self,
        layout_name: &str,
        resource_ids: Vec<u64>,
        creator: F,
    ) -> wgpu::BindGroup
    where
        F: FnOnce() -> wgpu::BindGroup,
    {
        let key = BindGroupKey {
            layout_name: layout_name.to_string(),
            resource_ids,
        };
        self.bind_group_cache
            .entry(key)
            .or_insert_with(creator)
            .clone()
    }

    pub fn get_bind_group(&self, layout_name: &str, resource_ids: Vec<u64>) -> &wgpu::BindGroup {
        let key = BindGroupKey {
            layout_name: layout_name.to_string(),
            resource_ids,
        };
        self.bind_group_cache
            .get(&key)
            .expect("Trying to get a bind group that is not created earlier in the graph!")
    }
}
