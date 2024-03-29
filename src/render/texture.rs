use crate::render::render_server::RenderServer;
use anyhow::*;
use image::{DynamicImage, GenericImageView, ImageBuffer, Rgb};
use std::collections::HashMap;
use std::path::Path;
use std::time::Instant;
use uuid;
use wgpu::Extent3d;

pub struct Texture {
    pub(crate) size: (u32, u32),
    // Actual data.
    pub texture: wgpu::Texture,
    // Thin wrapper over texture.
    pub view: wgpu::TextureView,
    // Defines how to sample the texture.
    pub sampler: wgpu::Sampler,
    pub format: wgpu::TextureFormat,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TextureId(uuid::Uuid);

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

        let img_copy_texture = wgpu::ImageCopyTexture {
            aspect: wgpu::TextureAspect::All,
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
        };

        // Write image data to texture.
        queue.write_texture(
            img_copy_texture,
            data,
            wgpu::ImageDataLayout {
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

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let texture = Texture {
            size,
            texture,
            view,
            sampler,
            format,
        };

        Ok(cache.add(texture))
    }

    pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

    pub fn create_depth_texture(
        device: &wgpu::Device,
        cache: &mut TextureCache,
        config: &wgpu::SurfaceConfiguration,
        label: Option<&str>,
    ) -> TextureId {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label,
            size: wgpu::Extent3d {
                width: config.width,
                height: config.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: Self::DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            compare: Some(wgpu::CompareFunction::LessEqual),
            lod_min_clamp: 0.0,
            lod_max_clamp: 100.0,
            ..Default::default()
        });

        let texture = Texture {
            size: (config.width, config.height),
            texture,
            view,
            sampler,
            format: Self::DEPTH_FORMAT,
        };

        cache.add(texture)
    }

    /// Set a new sampler for this texture.
    pub fn set_sampler(&mut self, new_sampler: wgpu::Sampler) {
        self.sampler = new_sampler;
    }

    pub fn load_cube<P: AsRef<Path>>(
        render_server: &RenderServer,
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

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label,
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        // Write image data to texture.
        queue.write_texture(
            wgpu::ImageCopyTexture {
                aspect: wgpu::TextureAspect::All,
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            &rgba,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * size.width),
                rows_per_image: Some(size.height),
            },
            size,
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some("cubemap texture view"),
            format: Some(format),
            dimension: Some(wgpu::TextureViewDimension::Cube),
            aspect: wgpu::TextureAspect::default(),
            base_mip_level: 0,
            mip_level_count: Some(1),
            base_array_layer: 0,
            array_layer_count: Some(6),
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let texture = Self {
            size: (0, 0),
            texture,
            view,
            sampler,
            format,
        };

        Ok(cache.add(texture))
    }
}
