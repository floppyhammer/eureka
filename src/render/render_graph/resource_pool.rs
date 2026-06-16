use super::resource::{BindGroupKey, BufferKey, PooledBuffer, SamplerKey, TextureKey};
use crate::render::{Texture, NEXT_TEXTURE_ID, NEXT_VIEW_ID};
use std::collections::HashMap;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};

/// 瞬时资源池，用于在帧内复用纹理和缓冲区，并支持多帧并行下的延迟回收
#[derive(Default)]
pub struct ResourcePool {
    /// 跨帧纹理池（支持 FIF 数据隔离）
    textures: HashMap<TextureKey, Vec<Texture>>, // “就绪”池
    pending_textures: Vec<(Texture, TextureKey, u64)>, // “待回收”队列

    /// 跨帧缓冲区池（支持 FIF 数据隔离）
    buffers: HashMap<BufferKey, Vec<PooledBuffer>>,
    pending_buffers: Vec<(PooledBuffer, BufferKey, u64)>,

    /// 统计信息：当前池中管理的所有缓冲区的总内存（字节）
    total_buffer_memory: u64,
    /// 统计信息：当前池中管理的所有纹理的总内存（字节）
    total_texture_memory: u64,

    /// 采样器永久缓存，采样器通常数量有限且不可变，直接缓存即可
    sampler_cache: HashMap<SamplerKey, wgpu::Sampler>,

    /// 帧内 BindGroup 缓存，每帧清空
    bind_group_cache: HashMap<BindGroupKey, wgpu::BindGroup>,
}

impl ResourcePool {
    pub fn update(&mut self, current_frame: u64, frames_in_flight: u64) {
        // 1. 回收纹理
        /*
        1. 检查 pending_textures：遍历这个队列。
        2. 判断安全期：如果某个纹理的 frame_count 已经距离现在超过了 MAX_FRAMES_IN_FLIGHT（通常是 2 或 3 帧），说明 GPU 肯定已经用完它了。
        3. 转移：将这个纹理从 pending_textures 移动到 textures 池中。
        4. 真正可用：从此，这个纹理才能再次被 acquire_texture 捡走。
        */
        let mut i = 0;
        while i < self.pending_textures.len() {
            if current_frame >= self.pending_textures[i].2 + frames_in_flight {
                let (texture, key, _) = self.pending_textures.remove(i);
                self.textures.entry(key).or_default().push(texture);
            } else {
                i += 1;
            }
        }

        // 2. 回收缓冲区
        let mut i = 0;
        while i < self.pending_buffers.len() {
            if current_frame >= self.pending_buffers[i].2 + frames_in_flight {
                let (buffer, key, _) = self.pending_buffers.remove(i);
                self.buffers.entry(key).or_default().push(buffer);
            } else {
                i += 1;
            }
        }
    }

    pub fn clear_bind_group_cache(&mut self) {
        self.bind_group_cache.clear();
    }

    // --- 纹理管理 ---

    pub fn acquire_texture(&mut self, device: &wgpu::Device, key: TextureKey) -> Texture {
        if let Some(textures) = self.textures.get_mut(&key) {
            if let Some(texture) = textures.pop() {
                return texture;
            }
        }

        let wgpu_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("transient_texture"),
            size: wgpu::Extent3d {
                width: key.width,
                height: key.height,
                depth_or_array_layers: key.layers,
            },
            mip_level_count: 1,
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

        let format = key.format.unwrap();
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
            wgpu::TextureFormat::Depth24Plus => 4, // 估算
            wgpu::TextureFormat::Depth24PlusStencil8 => 4, // 估算
            _ => 4,                                // 默认
        };

        let estimated_size = (key.width * key.height * key.layers) as u64 * bpp;
        self.total_texture_memory += estimated_size;

        log::info!(
            "Allocated new texture (size: {}x{}, format: {:?}, estimated: {:.2} MB). Total texture memory: {:.2} MB",
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
            id: NEXT_TEXTURE_ID.fetch_add(1, Ordering::Relaxed),
            view_id: NEXT_VIEW_ID.fetch_add(1, Ordering::Relaxed),
            view_cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// 调用后，资源并不会立即进入 textures 池。相反，它会被塞进 pending_textures，
    /// 并打上一个“时间戳”（当前的 frame_count）
    pub fn release_texture_deferred(&mut self, key: TextureKey, texture: Texture, frame_id: u64) {
        self.pending_textures.push((texture, key, frame_id));
    }

    // --- 缓冲区管理 ---

    pub fn acquire_buffer(
        &mut self,
        device: &wgpu::Device,
        key: BufferKey,
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
            let buffer = self.buffers.get_mut(&k).unwrap().pop().unwrap();
            return (buffer, k);
        }

        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("transient_buffer"),
            size: search_key.size,
            usage: search_key.usage | wgpu::BufferUsages::COPY_DST, // 强制包含 COPY_DST 方便写入
            mapped_at_creation: false,
        });

        self.total_buffer_memory += search_key.size;
        log::info!(
            "Allocated new buffer (size: {} bytes). Total buffer memory: {:.2} MB",
            search_key.size,
            self.total_buffer_memory as f64 / 1024.0 / 1024.0
        );

        (
            PooledBuffer {
                buffer,
                id: NEXT_TEXTURE_ID.fetch_add(1, Ordering::Relaxed),
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

    pub fn release_sampler_deferred(
        &mut self,
        _key: SamplerKey,
        _sampler: wgpu::Sampler,
        _frame_id: u64,
    ) {
        // 采样器现在由 sampler_cache 永久管理，不再需要延迟释放逻辑
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
