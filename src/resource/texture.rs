use anyhow::*;
use image::{DynamicImage, GenericImageView, ImageBuffer, Rgb};
use std::path::Path;
use std::time::Instant;
use cgmath::Point2;

use crate::resource::{Material2d, Mesh};
use crate::RenderServer;

/// [`Texture`] is a wrapper over wgpu texture, view and sampler.
/// It only stores data and is not responsible for drawing.

pub struct Texture {
    // Actual data.
    pub texture: wgpu::Texture,
    // Thin wrapper over texture.
    pub view: wgpu::TextureView,
    // Defines how to sample the texture.
    pub sampler: wgpu::Sampler,
    pub size: Point2<u32>,
    pub format: wgpu::TextureFormat,
}

impl Texture {
    pub fn load<P: AsRef<Path>>(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        path: P,
    ) -> Result<Self> {
        // Needed to appease the borrow checker.
        let path_copy = path.as_ref().to_path_buf();
        let label = path_copy.to_str();

        let img = image::open(path).context("Invalid image path")?;

        Self::from_image(device, queue, &img, label)
    }

    pub fn empty(device: &wgpu::Device, queue: &wgpu::Queue, size: (u32, u32)) -> Result<Self> {
        let mut data: Vec<u8> = Vec::new();
        data.reserve((size.0 * size.1 * 4) as usize);
        for _ in 0..(size.0 * size.1) {
            data.extend([222u8, 222, 222, 0]);
        }

        let mut image = DynamicImage::ImageRgba8(image::ImageBuffer::from_raw(size.0, size.1, data).unwrap());

        Self::from_image(device, queue, &image, Some("Empty image"))
    }

    /// Create texture from bytes.
    pub fn from_bytes(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bytes: &[u8],
        format: wgpu::TextureFormat,
        label: &str,
        y_flip: bool,
    ) -> Result<Self> {
        let img = image::load_from_memory(bytes)?;

        // Flip image.
        if y_flip {
            image::imageops::flip_vertical(&img);
        }

        Self::from_image(device, queue, &img, Some(label))
    }

    /// Create texture from image.
    pub fn from_image(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        img: &DynamicImage,
        label: Option<&str>,
    ) -> Result<Self> {
        // Image size.
        let dimensions = img.dimensions();

        let size = wgpu::Extent3d {
            width: dimensions.0,
            height: dimensions.1,
            depth_or_array_layers: 1,
        };

        let format = match img {
            DynamicImage::ImageLuma8(_) => {
                wgpu::TextureFormat::R8Unorm
            }
            DynamicImage::ImageRgb8(_) => {
                wgpu::TextureFormat::Rgba8UnormSrgb
            }
            DynamicImage::ImageRgba8(_) => {
                wgpu::TextureFormat::Rgba8UnormSrgb
            }
            _ => {
                panic!("Unsupported image format!");
            }
        };

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label,
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        });

        let img_copy_texture = wgpu::ImageCopyTexture {
            aspect: wgpu::TextureAspect::All,
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
        };

        // Write image data to texture.
        match img {
            DynamicImage::ImageLuma8(gray) => {
                queue.write_texture(
                    img_copy_texture,
                    &gray,
                    wgpu::ImageDataLayout {
                        offset: 0,
                        bytes_per_row: std::num::NonZeroU32::new(1 * dimensions.0),
                        rows_per_image: std::num::NonZeroU32::new(dimensions.1),
                    },
                    size,
                );
            }
            DynamicImage::ImageRgb8(_) => {
                // Make a rgba8 copy.
                let rgba = img.to_rgba8();

                queue.write_texture(
                    img_copy_texture,
                    &rgba,
                    wgpu::ImageDataLayout {
                        offset: 0,
                        bytes_per_row: std::num::NonZeroU32::new(4 * dimensions.0),
                        rows_per_image: std::num::NonZeroU32::new(dimensions.1),
                    },
                    size,
                );
            }
            DynamicImage::ImageRgba8(rgba) => {
                queue.write_texture(
                    img_copy_texture,
                    &rgba,
                    wgpu::ImageDataLayout {
                        offset: 0,
                        bytes_per_row: std::num::NonZeroU32::new(4 * dimensions.0),
                        rows_per_image: std::num::NonZeroU32::new(dimensions.1),
                    },
                    size,
                );
            }
            _ => {
                panic!("Unsupported image format!");
            }
        };

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        Ok(Self {
            texture,
            view,
            sampler,
            format,
            size: Point2::new(size.width, size.height),
        })
    }

    pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

    pub fn create_depth_texture(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
        label: &str,
    ) -> Self {
        // Our depth texture needs to be the same size as our screen if we want things to resource correctly.
        let size = wgpu::Extent3d {
            width: config.width,
            height: config.height,
            depth_or_array_layers: 1,
        };

        // Since we are rendering to this texture, we need to add the RENDER_ATTACHMENT flag to it.
        let desc = wgpu::TextureDescriptor {
            label: Some(label),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: Self::DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        };

        let texture = device.create_texture(&desc);

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            compare: Some(wgpu::CompareFunction::LessEqual),
            lod_min_clamp: -100.0,
            lod_max_clamp: 100.0,
            ..Default::default()
        });

        Self {
            texture,
            view,
            sampler,
            format: Self::DEPTH_FORMAT,
            size: Point2::new(size.width, size.height),
        }
    }

    /// Set a new sampler for this texture.
    pub fn set_sampler(&mut self, new_sampler: wgpu::Sampler) {
        self.sampler = new_sampler;
    }
}

pub struct CubemapTexture {
    // Actual data.
    pub texture: wgpu::Texture,
    // Thin wrapper over texture.
    pub view: wgpu::TextureView,
    // Defines how to sample the texture.
    pub sampler: wgpu::Sampler,
}

impl CubemapTexture {
    pub fn load<P: AsRef<Path>>(render_server: &RenderServer, path: P) -> Result<Self> {
        let now = Instant::now();

        // Needed to appease the borrow checker.
        let path_copy = path.as_ref().to_path_buf();
        let label = path_copy.to_str();

        let img = image::open(path).context("Invalid image path")?;

        let texture = Self::from_image(&render_server.device, &render_server.queue, &img, label);

        let elapsed_time = now.elapsed();
        log::info!("Loading cubemap texture took {} milliseconds", elapsed_time.as_millis());

        texture
    }

    /// Create texture from image.
    pub fn from_image(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        img: &image::DynamicImage,
        label: Option<&str>,
    ) -> Result<Self> {
        // Make a rgba8 copy.
        let rgba = img.to_rgba8();

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
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
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
                bytes_per_row: std::num::NonZeroU32::new(4 * size.width),
                rows_per_image: std::num::NonZeroU32::new(size.height),
            },
            size,
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some("cubemap texture view"),
            format: Some(wgpu::TextureFormat::Rgba8UnormSrgb),
            dimension: Some(wgpu::TextureViewDimension::Cube),
            aspect: wgpu::TextureAspect::default(),
            base_mip_level: 0,
            mip_level_count: std::num::NonZeroU32::new(1),
            base_array_layer: 0,
            array_layer_count: std::num::NonZeroU32::new(6),
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

        Ok(Self {
            texture,
            view,
            sampler,
        })
    }
}
