use crate::render::RenderContext;
use anyhow::*;
use image::{DynamicImage, GenericImageView, ImageBuffer};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use uuid;
use wgpu::Extent3d;

pub static NEXT_RESOURCE_ID: AtomicU64 = AtomicU64::new(1);
pub static NEXT_VIEW_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ViewKey {
    pub format: Option<wgpu::TextureFormat>,
    pub dimension: Option<wgpu::TextureViewDimension>,
    pub aspect: wgpu::TextureAspect,
    pub base_mip_level: u32,
    pub mip_level_count: Option<u32>,
    pub base_array_layer: u32,
    pub array_layer_count: Option<u32>,
}

impl From<&wgpu::TextureViewDescriptor<'_>> for ViewKey {
    fn from(desc: &wgpu::TextureViewDescriptor<'_>) -> Self {
        Self {
            format: desc.format,
            dimension: desc.dimension,
            aspect: desc.aspect,
            base_mip_level: desc.base_mip_level,
            mip_level_count: desc.mip_level_count,
            base_array_layer: desc.base_array_layer,
            array_layer_count: desc.array_layer_count,
        }
    }
}

#[derive(Clone)]
pub struct RawTextureData {
    pub name: String,
    pub pixels: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub format: wgpu::TextureFormat,
}

#[derive(Clone)]
pub struct RawCubeTextureData {
    pub name: String,
    pub pixels: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub format: wgpu::TextureFormat,
}

#[derive(Clone)]
pub struct Texture {
    pub(crate) size: (u32, u32),
    // Actual data.
    pub texture: wgpu::Texture,
    // Default view
    pub view: wgpu::TextureView,
    pub format: wgpu::TextureFormat,
    /// 唯一标识，用于缓存优化
    pub id: u64,
    pub view_id: u64,
    pub view_cache: Arc<Mutex<HashMap<ViewKey, (wgpu::TextureView, u64)>>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TextureId(uuid::Uuid);

/// Imported texture cache, not managed by ResourcePool.
pub struct TextureCache {
    pub(crate) storage: HashMap<TextureId, Texture>,
}

impl TextureCache {
    pub(crate) fn new() -> Self {
        Self {
            storage: HashMap::new(),
        }
    }

    pub(crate) fn add(&mut self, texture: Texture) -> TextureId {
        let id = TextureId(uuid::Uuid::new_v4());
        self.storage.insert(id, texture);
        id
    }

    pub(crate) fn get(&self, texture_id: TextureId) -> Option<&Texture> {
        self.storage.get(&texture_id)
    }

    pub(crate) fn get_mut(&mut self, texture_id: TextureId) -> Option<&mut Texture> {
        self.storage.get_mut(&texture_id)
    }

    pub(crate) fn remove(&mut self, texture_id: TextureId) {
        self.storage.remove(&texture_id);
    }
}

impl Texture {
    pub fn count_mips(width: u32, height: u32) -> u32 {
        (width.max(height) as f32).log2().floor() as u32 + 1
    }

    pub fn generate_mipmaps(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        texture: &wgpu::Texture,
        format: wgpu::TextureFormat,
        mip_count: u32,
        layer_count: u32,
    ) {
        if mip_count <= 1 {
            return;
        }

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Mipmap Generation Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/blit.wgsl").into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Mipmap Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Mipmap Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Mipmap Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Mipmap Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Mipmap Generation Encoder"),
        });

        for layer in 0..layer_count {
            for target_mip in 1..mip_count {
                let src_view = texture.create_view(&wgpu::TextureViewDescriptor {
                    label: Some("Mipmap Source View"),
                    format: Some(format),
                    dimension: Some(wgpu::TextureViewDimension::D2),
                    base_mip_level: target_mip - 1,
                    mip_level_count: Some(1),
                    base_array_layer: layer,
                    array_layer_count: Some(1),
                    usage: Some(wgpu::TextureUsages::TEXTURE_BINDING),
                    aspect: wgpu::TextureAspect::All,
                });

                let dst_view = texture.create_view(&wgpu::TextureViewDescriptor {
                    label: Some("Mipmap Destination View"),
                    format: Some(format),
                    dimension: Some(wgpu::TextureViewDimension::D2),
                    base_mip_level: target_mip,
                    mip_level_count: Some(1),
                    base_array_layer: layer,
                    array_layer_count: Some(1),
                    usage: Some(wgpu::TextureUsages::RENDER_ATTACHMENT),
                    aspect: wgpu::TextureAspect::All,
                });

                let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("Mipmap Bind Group"),
                    layout: &bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(&src_view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::Sampler(&sampler),
                        },
                    ],
                });

                {
                    let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("Mipmap Render Pass"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &dst_view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                                store: wgpu::StoreOp::Store,
                            },
                            depth_slice: None,
                        })],
                        depth_stencil_attachment: None,
                        timestamp_writes: None,
                        occlusion_query_set: None,
                    });

                    rpass.set_pipeline(&pipeline);
                    rpass.set_bind_group(0, &bind_group, &[]);
                    rpass.draw(0..3, 0..1);
                }
            }
        }

        queue.submit(Some(encoder.finish()));
    }

    pub fn load<P: AsRef<Path>>(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        cache: &mut TextureCache,
        path: P,
    ) -> Result<TextureId> {
        // Needed to appease the borrow checker.
        let path_copy = path.as_ref().to_path_buf();
        let label = path_copy.to_str();

        let img = image::open(path).context("Invalid image path")?;

        Self::from_image(device, queue, cache, &img, label)
    }

    pub fn empty(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        cache: &mut TextureCache,
        size: (u32, u32),
    ) -> Result<TextureId> {
        let mut data: Vec<u8> = Vec::new();
        data.reserve((size.0 * size.1 * 4) as usize);
        for _ in 0..(size.0 * size.1) {
            data.extend([222u8, 222, 222, 0]);
        }

        let image = DynamicImage::ImageRgba8(ImageBuffer::from_raw(size.0, size.1, data).unwrap());

        Self::from_image(device, queue, cache, &image, Some("empty image"))
    }

    /// Create texture from bytes.
    pub fn from_bytes(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        cache: &mut TextureCache,
        bytes: &[u8],
        label: &str,
        y_flip: bool,
    ) -> Result<TextureId> {
        let img = image::load_from_memory(bytes)?;

        // Flip image.
        if y_flip {
            image::imageops::flip_vertical(&img);
        }

        Self::from_image(device, queue, cache, &img, Some(label))
    }

    /// Create texture from image.
    pub fn from_image(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        cache: &mut TextureCache,
        img: &DynamicImage,
        label: Option<&str>,
    ) -> Result<TextureId> {
        // Image size.
        let size = img.dimensions();

        let data: &[u8];
        let bytes_per_row;
        let format;
        let rgba_converted_from_rgb;

        match img {
            DynamicImage::ImageLuma8(gray) => {
                data = &gray;
                bytes_per_row = 1 * size.0;
                format = wgpu::TextureFormat::R8Unorm;
            }
            DynamicImage::ImageRgb8(_) => {
                rgba_converted_from_rgb = img.to_rgba8();
                data = &rgba_converted_from_rgb;
                bytes_per_row = 4 * size.0;
                format = wgpu::TextureFormat::Rgba8UnormSrgb;
            }
            DynamicImage::ImageRgba8(rgba) => {
                data = rgba;
                bytes_per_row = 4 * size.0;
                format = wgpu::TextureFormat::Rgba8UnormSrgb;
            }
            _ => {
                panic!("Unsupported image format!");
            }
        }

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label,
            size: wgpu::Extent3d {
                width: size.0,
                height: size.1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });

        let img_copy_texture = wgpu::TexelCopyTextureInfo {
            aspect: wgpu::TextureAspect::All,
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
        };

        // Write image data to texture.
        queue.write_texture(
            img_copy_texture,
            data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(bytes_per_row),
                rows_per_image: Some(size.1),
            },
            Extent3d {
                width: size.0,
                height: size.1,
                depth_or_array_layers: 1,
            },
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let texture = Texture {
            size,
            texture,
            view,
            format,
            id: NEXT_RESOURCE_ID.fetch_add(1, Ordering::Relaxed),
            view_id: NEXT_VIEW_ID.fetch_add(1, Ordering::Relaxed),
            view_cache: Arc::new(Mutex::new(HashMap::new())),
        };

        Ok(cache.add(texture))
    }

    pub fn create_depth_texture_with_size(
        device: &wgpu::Device,
        cache: &mut TextureCache,
        width: u32,
        height: u32,
        layers: u32,
        cube_view: bool,
        label: Option<&str>,
    ) -> TextureId {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label,
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: layers,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: Self::DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some("depth texture view"),
            format: Some(Self::DEPTH_FORMAT),
            dimension: if cube_view {
                if layers > 6 {
                    Some(wgpu::TextureViewDimension::CubeArray)
                } else {
                    Some(wgpu::TextureViewDimension::Cube)
                }
            } else {
                if layers > 1 {
                    Some(wgpu::TextureViewDimension::D2Array)
                } else {
                    Some(wgpu::TextureViewDimension::D2)
                }
            },
            usage: Some(
                wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
            ),
            aspect: wgpu::TextureAspect::DepthOnly,
            base_mip_level: 0,
            mip_level_count: None,
            base_array_layer: 0,
            array_layer_count: Some(layers),
        });

        let texture = Texture {
            size: (width, height),
            texture,
            view,
            format: Self::DEPTH_FORMAT,
            id: NEXT_RESOURCE_ID.fetch_add(1, Ordering::Relaxed),
            view_id: NEXT_VIEW_ID.fetch_add(1, Ordering::Relaxed),
            view_cache: Arc::new(Mutex::new(HashMap::new())),
        };

        cache.add(texture)
    }

    pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

    pub fn get_view(&self, desc: &wgpu::TextureViewDescriptor) -> (wgpu::TextureView, u64) {
        let key = ViewKey::from(desc);
        let mut cache = self.view_cache.lock().unwrap();

        if let Some(entry) = cache.get(&key) {
            return (entry.0.clone(), entry.1);
        }

        let view = self.texture.create_view(desc);

        // 生成派生 ID：结合物理 ID 和描述符哈希，确保全局唯一且稳定
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.id.hash(&mut hasher);
        key.hash(&mut hasher);
        let derived_id = hasher.finish();

        cache.insert(key, (view.clone(), derived_id));
        (view, derived_id)
    }

    pub fn create_depth_texture(
        device: &wgpu::Device,
        cache: &mut TextureCache,
        config: &wgpu::SurfaceConfiguration,
        label: Option<&str>,
    ) -> TextureId {
        Self::create_depth_texture_with_size(
            device,
            cache,
            config.width,
            config.height,
            1,
            false,
            label,
        )
    }

    pub fn decode_from_disk<P: AsRef<Path>>(path: P) -> Result<RawTextureData> {
        let path_ref = path.as_ref();
        let name = path_ref
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        let img = image::open(path_ref).context("Invalid image path")?;
        let (width, height) = img.dimensions();

        let pixels;
        let format;

        match img {
            DynamicImage::ImageLuma8(gray) => {
                pixels = gray.into_raw();
                format = wgpu::TextureFormat::R8Unorm;
            }
            _ => {
                pixels = img.to_rgba8().into_raw();
                format = wgpu::TextureFormat::Rgba8UnormSrgb;
            }
        }

        Ok(RawTextureData {
            name,
            pixels,
            width,
            height,
            format,
        })
    }

    pub fn from_raw(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        cache: &mut TextureCache,
        raw: RawTextureData,
    ) -> TextureId {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(&raw.name),
            size: wgpu::Extent3d {
                width: raw.width,
                height: raw.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: raw.format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });

        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                aspect: wgpu::TextureAspect::All,
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            &raw.pixels,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(raw.pixels.len() as u32 / raw.height),
                rows_per_image: Some(raw.height),
            },
            Extent3d {
                width: raw.width,
                height: raw.height,
                depth_or_array_layers: 1,
            },
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        cache.add(Texture {
            size: (raw.width, raw.height),
            texture,
            view,
            format: raw.format,
            id: NEXT_RESOURCE_ID.fetch_add(1, Ordering::Relaxed),
            view_id: NEXT_VIEW_ID.fetch_add(1, Ordering::Relaxed),
            view_cache: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    pub fn decode_cube_from_disk<P: AsRef<Path>>(path: P) -> Result<RawCubeTextureData> {
        let path_ref = path.as_ref();
        let name = path_ref
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        let img = image::open(path_ref).context("Invalid image path")?;
        let dimensions = img.dimensions();
        let rgba = img.to_rgba8();

        Ok(RawCubeTextureData {
            name,
            pixels: rgba.into_raw(),
            width: dimensions.0,
            height: dimensions.1,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
        })
    }

    pub fn from_raw_cube(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        cache: &mut TextureCache,
        raw: RawCubeTextureData,
    ) -> TextureId {
        let size = wgpu::Extent3d {
            width: raw.width,
            height: raw.height / 6,
            depth_or_array_layers: 6,
        };

        let mip_level_count = Self::count_mips(size.width, size.height);

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(&raw.name),
            size,
            mip_level_count,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: raw.format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });

        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                aspect: wgpu::TextureAspect::All,
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            &raw.pixels,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * size.width),
                rows_per_image: Some(size.height),
            },
            size,
        );

        Self::generate_mipmaps(device, queue, &texture, raw.format, mip_level_count, 6);

        let view = texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some("cubemap texture view"),
            format: Some(raw.format),
            dimension: Some(wgpu::TextureViewDimension::Cube),
            usage: None,
            aspect: wgpu::TextureAspect::default(),
            base_mip_level: 0,
            mip_level_count: Some(mip_level_count),
            base_array_layer: 0,
            array_layer_count: Some(6),
        });

        cache.add(Texture {
            size: (size.width, size.height),
            texture,
            view,
            format: raw.format,
            id: NEXT_RESOURCE_ID.fetch_add(1, Ordering::Relaxed),
            view_id: NEXT_VIEW_ID.fetch_add(1, Ordering::Relaxed),
            view_cache: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    pub fn load_cube<P: AsRef<Path>>(
        render_server: &RenderContext,
        cache: &mut TextureCache,
        path: P,
    ) -> Result<TextureId> {
        // Needed to appease the borrow checker.
        let path_copy = path.as_ref().to_path_buf();
        let label = path_copy.to_str();

        let img = image::open(path).context("Invalid image path")?;

        Self::from_cube_image(
            &render_server.device,
            &render_server.queue,
            cache,
            &img,
            label,
        )
    }

    /// Create texture from image.
    pub fn from_cube_image(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        cache: &mut TextureCache,
        img: &image::DynamicImage,
        label: Option<&str>,
    ) -> Result<TextureId> {
        // Make a rgba8 copy.
        let rgba = img.to_rgba8();

        let format = wgpu::TextureFormat::Rgba8UnormSrgb;

        // Image size.
        let dimensions = img.dimensions();

        assert_eq!(dimensions.1 % 6, 0, "Skybox texture has invalid size!");

        let size = wgpu::Extent3d {
            width: dimensions.0,
            height: dimensions.1 / 6,
            depth_or_array_layers: 6,
        };

        let mip_level_count = Self::count_mips(size.width, size.height);

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label,
            size,
            mip_level_count,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });

        // Write image data to texture.
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                aspect: wgpu::TextureAspect::All,
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            &rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * size.width),
                rows_per_image: Some(size.height),
            },
            size,
        );

        Self::generate_mipmaps(device, queue, &texture, format, mip_level_count, 6);

        let view = texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some("cubemap texture view"),
            format: Some(format),
            dimension: Some(wgpu::TextureViewDimension::Cube),
            usage: None,
            aspect: wgpu::TextureAspect::default(),
            base_mip_level: 0,
            mip_level_count: Some(mip_level_count),
            base_array_layer: 0,
            array_layer_count: Some(6),
        });

        let texture = Self {
            size: (size.width, size.height),
            texture,
            view,
            format,
            id: NEXT_RESOURCE_ID.fetch_add(1, Ordering::Relaxed),
            view_id: NEXT_VIEW_ID.fetch_add(1, Ordering::Relaxed),
            view_cache: Arc::new(Mutex::new(HashMap::new())),
        };

        Ok(cache.add(texture))
    }
}
