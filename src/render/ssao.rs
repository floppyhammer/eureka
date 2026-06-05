use crate::math::frustum::Frustum;
use crate::render::camera::{CameraRenderResources, CameraUniform};
use crate::render::mesh::{ExtractedMesh, InstanceRaw, MeshCache, MeshRenderResources};
use crate::render::vertex::{Vertex3d, VertexBuffer};
use crate::render::{create_render_pipeline, RenderContext, Texture, TextureCache, TextureId};
use crate::scene::Bvh;
use glam::{vec3, vec4, Mat4, Vec3};
use wgpu::util::DeviceExt;

pub struct SsaoRenderResources {
    pub normal_texture: TextureId,
    pub ssao_texture: TextureId,
    pub blur_texture: TextureId,
    pub noise_texture: TextureId,

    pub ssao_uniform_buffer: wgpu::Buffer,
    pub ssao_bind_group: wgpu::BindGroup,
    pub blur_bind_group: wgpu::BindGroup,
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct SSAOUniform {
    samples: [[f32; 4]; 64],
}

impl SsaoRenderResources {
    pub fn new(
        render_server: &RenderContext,
        texture_cache: &mut TextureCache,
        camera_resources: &CameraRenderResources,
        initial_depth_texture: TextureId,
    ) -> Self {
        let device = &render_server.device;
        let config = &render_server.surface_config;

        // 1. Textures
        let normal_texture = texture_cache.add(create_color_texture(
            device,
            config.width,
            config.height,
            wgpu::TextureFormat::Rgba16Float,
            "SSAO Normal Texture",
        ));

        let ssao_texture = texture_cache.add(create_color_texture(
            device,
            config.width,
            config.height,
            wgpu::TextureFormat::R8Unorm,
            "SSAO Raw Texture",
        ));

        let blur_texture = texture_cache.add(create_color_texture(
            device,
            config.width,
            config.height,
            wgpu::TextureFormat::R8Unorm,
            "SSAO Blurred Texture",
        ));

        // Noise texture (4x4)
        let mut noise_data = Vec::new();
        let mut seed = 42u32;
        for _ in 0..16 {
            noise_data.push(rand_f32(&mut seed) * 2.0 - 1.0);
            noise_data.push(rand_f32(&mut seed) * 2.0 - 1.0);
            noise_data.push(0.0);
            noise_data.push(0.0);
        }
        let noise_texture = texture_cache.add(create_noise_texture(
            device,
            &render_server.queue,
            &noise_data,
        ));

        // 2. Uniforms
        let mut kernel = [0.0f32; 64 * 4];
        for i in 0..64 {
            let sample = vec3(
                rand_f32(&mut seed) * 2.0 - 1.0,
                rand_f32(&mut seed) * 2.0 - 1.0,
                rand_f32(&mut seed),
            )
            .normalize()
                * rand_f32(&mut seed);

            let mut scale = i as f32 / 64.0;
            scale = lerp(0.1, 1.0, scale * scale);
            let sample = sample * scale;

            kernel[i * 4] = sample.x;
            kernel[i * 4 + 1] = sample.y;
            kernel[i * 4 + 2] = sample.z;
            kernel[i * 4 + 3] = 0.0;
        }

        let ssao_uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("SSAO Uniform Buffer"),
            contents: bytemuck::cast_slice(&kernel),
            usage: wgpu::BufferUsages::UNIFORM,
        });

        // 4. Bind Groups
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let noise_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            ..Default::default()
        });

        let normal_view = &texture_cache.get(normal_texture).unwrap().view;
        let depth_view = &texture_cache.get(initial_depth_texture).unwrap().view;
        let noise_view = &texture_cache.get(noise_texture).unwrap().view;

        // Note: Layouts are temporary here to create bind groups during init.
        // In a full refactor, layouts should be managed by a central cache.
        let ssao_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("SSAO Bind Group Layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry { binding: 0, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None }, count: None },
                    wgpu::BindGroupLayoutEntry { binding: 1, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: true }, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false }, count: None },
                    wgpu::BindGroupLayoutEntry { binding: 2, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering), count: None },
                    wgpu::BindGroupLayoutEntry { binding: 3, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Depth, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false }, count: None },
                    wgpu::BindGroupLayoutEntry { binding: 4, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: false }, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false }, count: None },
                    wgpu::BindGroupLayoutEntry { binding: 5, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering), count: None },
                ],
            });

        let ssao_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &ssao_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: ssao_uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(normal_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(depth_view),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::TextureView(noise_view),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: wgpu::BindingResource::Sampler(&noise_sampler),
                },
            ],
            label: Some("SSAO Bind Group"),
        });

        let blur_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("SSAO Blur Bind Group Layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry { binding: 0, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: true }, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false }, count: None },
                    wgpu::BindGroupLayoutEntry { binding: 1, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering), count: None },
                ],
            });

        let ssao_view = &texture_cache.get(ssao_texture).unwrap().view;
        let blur_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &blur_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(ssao_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
            label: Some("SSAO Blur Bind Group"),
        });

        Self {
            normal_texture,
            ssao_texture,
            blur_texture,
            noise_texture,
            ssao_uniform_buffer,
            ssao_bind_group,
            blur_bind_group,
        }
    }

    pub fn on_resize(
        &mut self,
        device: &wgpu::Device,
        texture_cache: &mut TextureCache,
        width: u32,
        height: u32,
        depth_texture_id: TextureId,
    ) {
        texture_cache.remove(self.normal_texture);
        texture_cache.remove(self.ssao_texture);
        texture_cache.remove(self.blur_texture);

        self.normal_texture = texture_cache.add(create_color_texture(
            device,
            width,
            height,
            wgpu::TextureFormat::Rgba16Float,
            "SSAO Normal Texture",
        ));

        self.ssao_texture = texture_cache.add(create_color_texture(
            device,
            width,
            height,
            wgpu::TextureFormat::R8Unorm,
            "SSAO Raw Texture",
        ));

        self.blur_texture = texture_cache.add(create_color_texture(
            device,
            width,
            height,
            wgpu::TextureFormat::R8Unorm,
            "SSAO Blurred Texture",
        ));

        self.update_bind_groups(device, texture_cache, depth_texture_id);
    }

    pub fn update_bind_groups(
        &mut self,
        device: &wgpu::Device,
        texture_cache: &TextureCache,
        depth_texture_id: TextureId,
    ) {
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let noise_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let normal_view = &texture_cache.get(self.normal_texture).unwrap().view;
        let depth_view = &texture_cache.get(depth_texture_id).unwrap().view;
        let noise_view = &texture_cache.get(self.noise_texture).unwrap().view;

        // Note: Recreating layouts because they aren't stored.
        // This is a sign we need a central BindGroupLayout cache!
        let ssao_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("SSAO Bind Group Layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry { binding: 0, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None }, count: None },
                    wgpu::BindGroupLayoutEntry { binding: 1, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: true }, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false }, count: None },
                    wgpu::BindGroupLayoutEntry { binding: 2, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering), count: None },
                    wgpu::BindGroupLayoutEntry { binding: 3, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Depth, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false }, count: None },
                    wgpu::BindGroupLayoutEntry { binding: 4, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: false }, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false }, count: None },
                    wgpu::BindGroupLayoutEntry { binding: 5, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering), count: None },
                ],
            });

        self.ssao_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &ssao_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.ssao_uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(normal_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(depth_view),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::TextureView(noise_view),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: wgpu::BindingResource::Sampler(&noise_sampler),
                },
            ],
            label: Some("SSAO Bind Group"),
        });

        let blur_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("SSAO Blur Bind Group Layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry { binding: 0, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: true }, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false }, count: None },
                    wgpu::BindGroupLayoutEntry { binding: 1, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering), count: None },
                ],
            });

        let ssao_view = &texture_cache.get(self.ssao_texture).unwrap().view;
        self.blur_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &blur_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(ssao_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
            label: Some("SSAO Blur Bind Group"),
        });
    }

    pub fn render_normal<'a>(
        &'a self,
        render_pass: &mut wgpu::RenderPass<'a>,
        _extracted_meshes: &'a Vec<ExtractedMesh>,
        _mesh_cache: &'a MeshCache,
        mesh_render_resources: &'a MeshRenderResources,
        camera_bind_group: &'a wgpu::BindGroup,
        camera_index: usize,
        _camera_uniform: &CameraUniform,
        _bvh: &'a Bvh,
        pipeline: &'a wgpu::RenderPipeline,
    ) {
        render_pass.set_pipeline(pipeline);
        let offset = camera_index as u32 * CameraUniform::get_uniform_offset_unit();
        render_pass.set_bind_group(0, camera_bind_group, &[offset]);

        if mesh_render_resources.global_indirect_buffer.is_none() {
            return;
        }

        render_pass.set_vertex_buffer(0, mesh_render_resources.mesh_allocator.vertex_buffer.slice(..));
        render_pass.set_vertex_buffer(1, mesh_render_resources.global_visible_instance_buffer.as_ref().unwrap().slice(..));
        render_pass.set_index_buffer(mesh_render_resources.mesh_allocator.index_buffer.slice(..), wgpu::IndexFormat::Uint32);

        if !mesh_render_resources.draw_counts.is_empty() && mesh_render_resources.draw_counts[0] > 0 {
            render_pass.multi_draw_indexed_indirect(
                mesh_render_resources.global_indirect_buffer.as_ref().unwrap(),
                0,
                mesh_render_resources.draw_counts[0],
            );
        }
    }
}

fn create_color_texture(
    device: &wgpu::Device,
    width: u32,
    height: u32,
    format: wgpu::TextureFormat,
    label: &str,
) -> Texture {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        ..Default::default()
    });
    Texture {
        size: (width, height),
        texture,
        view,
        sampler,
        format,
    }
}

fn create_noise_texture(device: &wgpu::Device, queue: &wgpu::Queue, data: &[f32]) -> Texture {
    let size = wgpu::Extent3d {
        width: 4,
        height: 4,
        depth_or_array_layers: 1,
    };
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("SSAO Noise Texture"),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba32Float,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        bytemuck::cast_slice(data),
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(4 * 16),
            rows_per_image: Some(4),
        },
        size,
    );
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        address_mode_u: wgpu::AddressMode::Repeat,
        address_mode_v: wgpu::AddressMode::Repeat,
        ..Default::default()
    });
    Texture {
        size: (4, 4),
        texture,
        view,
        sampler,
        format: wgpu::TextureFormat::Rgba32Float,
    }
}

fn rand_f32(seed: &mut u32) -> f32 {
    *seed = seed.wrapping_mul(1103515245).wrapping_add(12345);
    ((*seed >> 16) & 0x7FFF) as f32 / 32767.0
}

fn lerp(a: f32, b: f32, f: f32) -> f32 {
    a + f * (b - a)
}
