use crate::render::camera::{CameraRenderResources, CameraUniform};
use crate::render::gizmo::GizmoRenderResources;
use crate::render::shader_maker::ShaderMaker;
use crate::render::vertex::{Vertex3d, VertexBuffer};
use crate::render::{create_render_pipeline, ExtractedMesh, InstanceRaw, MeshCache, MeshRenderResources, RenderServer, Texture, TextureCache, TextureId};
use crate::scene::OPENGL_TO_WGPU_MATRIX;
use cgmath::{EuclideanSpace, InnerSpace, Matrix4, Point3, SquareMatrix, Vector3};
use std::mem;
use wgpu::BufferAddress;

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct PointLightUniform {
    pub(crate) position: [f32; 3],
    pub(crate) strength: f32,
    pub(crate) color: [f32; 3],
    pub(crate) constant: f32,
    pub(crate) linear: f32,
    pub(crate) quadratic: f32,
    pub(crate) _pad0: f32,
    pub(crate) _pad1: f32,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct DirectionalLightUniform {
    pub(crate) direction: [f32; 3],
    pub(crate) strength: f32,
    pub(crate) color: [f32; 3],
    pub(crate) distance: f32,
}

#[derive(Debug, Default, Clone)]
pub(crate) struct ExtractedLights {
    pub(crate) point_lights: Vec<PointLightUniform>,
    pub(crate) directional_light: Option<DirectionalLightUniform>,
}

const MAX_POINT_LIGHTS: usize = 10;
pub(crate) const NUM_CASCADES: usize = 3;

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct LightUniform {
    pub(crate) ambient_color: [f32; 3],
    pub(crate) ambient_strength: f32,
    pub(crate) directional_light: DirectionalLightUniform,
    pub(crate) point_lights: [PointLightUniform; MAX_POINT_LIGHTS],
    pub(crate) point_light_count: u32,
    pub(crate) _pad: [u32; 3],
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct CascadeUniform {
    pub(crate) view_proj: [[[f32; 4]; 4]; NUM_CASCADES],
    pub(crate) splits: [f32; 4],
}

pub(crate) struct LightRenderResources {
    pub(crate) shadow_map: Option<TextureId>,
    pub(crate) pipeline: Option<wgpu::RenderPipeline>,
    pub(crate) shadow_camera_bind_group: Option<wgpu::BindGroup>,
    pub(crate) shadow_camera_buffer: Option<wgpu::Buffer>,
    pub(crate) cascade_uniform_buffer: Option<wgpu::Buffer>,
}

impl LightRenderResources {
    pub(crate) fn new() -> Self {
        Self {
            shadow_map: None,
            pipeline: None,
            shadow_camera_bind_group: None,
            shadow_camera_buffer: None,
            cascade_uniform_buffer: None,
        }
    }

    pub fn prepare_pipeline(
        &mut self,
        render_server: &RenderServer,
        camera_render_resources: &CameraRenderResources,
    ) {
        if self.pipeline.is_some() {
            return;
        }

        let pipeline_layout =
            render_server
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("shadow pipeline layout"),
                    bind_group_layouts: &[&camera_render_resources.bind_group_layout],
                    push_constant_ranges: &[],
                });

        let shader = wgpu::ShaderModuleDescriptor {
            label: Some("shadow shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/shadow.wgsl").into()),
        };

        let pipeline = create_render_pipeline(
            &render_server.device,
            &pipeline_layout,
            None,
            Some(Texture::DEPTH_FORMAT),
            &[Vertex3d::desc(), InstanceRaw::desc()],
            shader,
            "shadow pipeline",
            false,
            Some(wgpu::Face::Back),
        );

        self.pipeline = Some(pipeline);
    }
}

pub(crate) fn prepare_shadow(
    extracted_lights: &ExtractedLights,
    main_camera: Option<&CameraUniform>,
    render_server: &RenderServer,
    texture_cache: &mut TextureCache,
    render_resources: &mut LightRenderResources,
    camera_render_resources: &CameraRenderResources,
) {
    if render_resources.cascade_uniform_buffer.is_none() {
        let buffer = render_server.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("cascade uniform buffer"),
            size: mem::size_of::<CascadeUniform>() as BufferAddress,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        render_resources.cascade_uniform_buffer = Some(buffer);
    }

    let offset_unit = CameraUniform::get_uniform_offset_unit();
    let shadow_camera_buffer_size = offset_unit * (NUM_CASCADES as u32);

    if render_resources.shadow_camera_buffer.is_none() {
        let buffer = render_server.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("shadow camera buffer"),
            size: shadow_camera_buffer_size as BufferAddress,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group = render_server
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &camera_render_resources.bind_group_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &buffer,
                        offset: 0,
                        size: Some(wgpu::BufferSize::new(mem::size_of::<CameraUniform>() as u64).unwrap()),
                    }),
                }],
                label: Some("shadow camera bind group"),
            });

        render_resources.shadow_camera_bind_group = Some(bind_group);
        render_resources.shadow_camera_buffer = Some(buffer);
    }

    if let (Some(directional_light), Some(camera)) = (&extracted_lights.directional_light, main_camera) {
        let light_dir = Vector3::from(directional_light.direction).normalize();

        // 视锥体分割距离
        let near = 0.1;
        let far = 100.0;
        let cascade_splits = [near, 10.0, 35.0, far];

        let view_mat = Matrix4::from(camera.view);
        let proj_mat = Matrix4::from(camera.proj);
        let inv_cam = (proj_mat * view_mat).invert().expect("Main camera matrix should be invertible");

        let mut camera_uniforms = Vec::new();
        let mut cascade_uniform = CascadeUniform::default();
        cascade_uniform.splits = [cascade_splits[1], cascade_splits[2], cascade_splits[3], 0.0];

        for i in 0..NUM_CASCADES {
            let split_near = cascade_splits[i];
            let split_far = cascade_splits[i + 1];

            // WGPU NDC 空间 Z 是 0.0 到 1.0
            let corners = [
                cgmath::Point3::new(-1.0, 1.0, 0.0),
                cgmath::Point3::new(1.0, 1.0, 0.0),
                cgmath::Point3::new(1.0, -1.0, 0.0),
                cgmath::Point3::new(-1.0, -1.0, 0.0),
                cgmath::Point3::new(-1.0, 1.0, 1.0),
                cgmath::Point3::new(1.0, 1.0, 1.0),
                cgmath::Point3::new(1.0, -1.0, 1.0),
                cgmath::Point3::new(-1.0, -1.0, 1.0),
            ];

            let mut world_corners = [cgmath::Point3::origin(); 8];
            for j in 0..8 {
                let pt = inv_cam * corners[j].to_vec().extend(1.0);
                world_corners[j] = cgmath::Point3::from_vec(pt.truncate() / pt.w);
            }

            // 修正级联裁剪：根据分割距离重新计算世界坐标
            // 线性插值虽然不完全准确，但对于正交光足够。更好的做法是重投影深度。
            for j in 0..4 {
                let dir = world_corners[j + 4] - world_corners[j];
                world_corners[j + 4] = world_corners[j] + dir * (split_far / far);
                world_corners[j] = world_corners[j] + dir * (split_near / far);
            }

            // 稳定化级联：计算包围球中心
            let mut center = cgmath::Vector3::new(0.0, 0.0, 0.0);
            for j in 0..8 {
                center += world_corners[j].to_vec();
            }
            center /= 8.0;

            // 计算包围球半径
            let mut radius = 0.0f32;
            for j in 0..8 {
                let distance = (world_corners[j] - Point3::from_vec(center)).magnitude();
                radius = radius.max(distance);
            }
            radius = (radius * 1.1).ceil(); // 稍微扩大并取整以稳定像素

            // 灯光相机观察矩阵：将眼睛退后足够远，以防遮挡物被切
            let light_view = Matrix4::look_at_rh(
                cgmath::Point3::from_vec(center - light_dir * radius * 2.0),
                cgmath::Point3::from_vec(center),
                Vector3::unit_y(),
            );

            // 使用包围球半径创建对称的正交矩阵
            let light_proj = cgmath::ortho(-radius, radius, -radius, radius, 0.0, radius * 4.0);
            let view_proj = OPENGL_TO_WGPU_MATRIX * light_proj * light_view;

            camera_uniforms.push(CameraUniform {
                view_position: [center.x, center.y, center.z, 1.0],
                view: light_view.into(),
                proj: light_proj.into(),
                view_proj: view_proj.into(),
            });

            cascade_uniform.view_proj[i] = view_proj.into();
        }

        // 写入缓冲区逻辑保持不变
        let mut shadow_camera_data = vec![0u8; shadow_camera_buffer_size as usize];
        for i in 0..NUM_CASCADES {
            let bytes = bytemuck::bytes_of(&camera_uniforms[i]);
            let offset = i * offset_unit as usize;
            shadow_camera_data[offset..offset + bytes.len()].copy_from_slice(bytes);
        }
        render_server.queue.write_buffer(
            render_resources.shadow_camera_buffer.as_ref().unwrap(),
            0,
            &shadow_camera_data,
        );

        render_server.queue.write_buffer(
            render_resources.cascade_uniform_buffer.as_ref().unwrap(),
            0,
            bytemuck::bytes_of(&cascade_uniform),
        );
    }

    if render_resources.shadow_map.is_none() {
        let depth_texture = Texture::create_depth_texture_with_size(
            &render_server.device,
            texture_cache,
            2048,
            2048,
            NUM_CASCADES as u32,
            Some("shadow map array"),
        );
        render_resources.shadow_map = Some(depth_texture);
    }

    render_resources.prepare_pipeline(render_server, camera_render_resources);
}

pub(crate) fn render_shadow(
    encoder: &mut wgpu::CommandEncoder,
    texture_cache: &TextureCache,
    render_resources: &LightRenderResources,
    extracted_meshes: &Vec<ExtractedMesh>,
    mesh_cache: &MeshCache,
    mesh_render_resources: &MeshRenderResources,
) {
    if render_resources.shadow_map.is_none() || render_resources.pipeline.is_none() {
        return;
    }

    let shadow_map_id = render_resources.shadow_map.unwrap();
    let shadow_map = texture_cache.get(shadow_map_id).unwrap();

    let offset_unit = CameraUniform::get_uniform_offset_unit();

    for i in 0..NUM_CASCADES {
        let cascade_view = shadow_map.texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some("shadow cascade view"),
            format: Some(Texture::DEPTH_FORMAT),
            dimension: Some(wgpu::TextureViewDimension::D2),
            usage: Some(wgpu::TextureUsages::RENDER_ATTACHMENT),
            aspect: wgpu::TextureAspect::DepthOnly,
            base_mip_level: 0,
            mip_level_count: None,
            base_array_layer: i as u32,
            array_layer_count: Some(1),
        });

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("shadow render pass"),
            color_attachments: &[],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &cascade_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        if let Some(bind_group) = &render_resources.shadow_camera_bind_group {
            let dynamic_offset = (i as u32) * offset_unit;
            render_pass.set_bind_group(0, bind_group, &[dynamic_offset]);
        }

        for extracted in extracted_meshes {
            let pipeline = render_resources.pipeline.as_ref().unwrap();
            let mesh = mesh_cache.get(extracted.mesh_id).unwrap();
            let instance = mesh_render_resources
                .instance_cache
                .get(&extracted.mesh_id)
                .unwrap();

            render_pass.set_pipeline(pipeline);
            render_pass.set_vertex_buffer(1, instance.buffer.slice(..));
            render_pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
            render_pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            render_pass.draw_indexed(0..mesh.index_count, 0, 0..1);
        }
    }
}
