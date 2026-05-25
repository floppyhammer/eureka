use crate::math::frustum::Frustum;
use crate::render::camera::{CameraRenderResources, CameraUniform};
use crate::render::vertex::{Vertex3d, VertexBuffer};
use crate::render::{
    create_render_pipeline, ExtractedMesh, InstanceRaw, MeshCache, MeshRenderResources,
    RenderServer, Texture, TextureCache, TextureId,
};
use crate::scene::Bvh;
use glam::{Mat4, Vec3};
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
    pub(crate) shadow_near: f32,
    pub(crate) shadow_far: f32,
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

pub(crate) const MAX_POINT_LIGHTS: usize = 4;
pub(crate) const NUM_CASCADES: usize = 3;

const POINT_SHADOW_FACES: [(Vec3, Vec3); 6] = [
    // 每一个面的 (Target, Up) 必须严格对应
    (Vec3::new(1.0, 0.0, 0.0), Vec3::new(0.0, -1.0, 0.0)), // +X
    (Vec3::new(-1.0, 0.0, 0.0), Vec3::new(0.0, -1.0, 0.0)), // -X
    (Vec3::new(0.0, 1.0, 0.0), Vec3::new(0.0, 0.0, 1.0)),  // +Y (注意 Up 是 +Z)
    (Vec3::new(0.0, -1.0, 0.0), Vec3::new(0.0, 0.0, -1.0)), // -Y (注意 Up 是 -Z)
    (Vec3::new(0.0, 0.0, 1.0), Vec3::new(0.0, -1.0, 0.0)), // +Z
    (Vec3::new(0.0, 0.0, -1.0), Vec3::new(0.0, -1.0, 0.0)), // -Z
];

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
    pub(crate) directional_shadow_map: Option<TextureId>,
    pub(crate) pipeline: Option<wgpu::RenderPipeline>,
    pub(crate) directional_shadow_camera_bind_group: Option<wgpu::BindGroup>,
    pub(crate) directional_shadow_camera_buffer: Option<wgpu::Buffer>,
    pub(crate) cascade_uniform_buffer: Option<wgpu::Buffer>,
    pub(crate) cascade_view_projs: [Mat4; NUM_CASCADES],

    pub(crate) point_shadow_map: Option<TextureId>,
    pub(crate) point_shadow_camera_buffer: Option<wgpu::Buffer>,
    pub(crate) point_shadow_camera_bind_group: Option<wgpu::BindGroup>,
    pub(crate) point_shadow_view_projs: Vec<Mat4>,
}

impl LightRenderResources {
    pub(crate) fn new() -> Self {
        Self {
            directional_shadow_map: None,
            pipeline: None,
            directional_shadow_camera_bind_group: None,
            directional_shadow_camera_buffer: None,
            cascade_uniform_buffer: None,
            cascade_view_projs: [Mat4::IDENTITY; NUM_CASCADES],
            point_shadow_map: None,
            point_shadow_camera_buffer: None,
            point_shadow_camera_bind_group: None,
            point_shadow_view_projs: vec![],
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
            Some(wgpu::Face::Front),
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
            size: size_of::<CascadeUniform>() as BufferAddress,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        render_resources.cascade_uniform_buffer = Some(buffer);
    }

    let offset_unit = CameraUniform::get_uniform_offset_unit();
    let shadow_camera_buffer_size = offset_unit * (NUM_CASCADES as u32);

    if render_resources.directional_shadow_camera_buffer.is_none() {
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
                        size: Some(
                            wgpu::BufferSize::new(size_of::<CameraUniform>() as u64).unwrap(),
                        ),
                    }),
                }],
                label: Some("shadow camera bind group"),
            });

        render_resources.directional_shadow_camera_bind_group = Some(bind_group);
        render_resources.directional_shadow_camera_buffer = Some(buffer);
    }

    // Directional shadow logic
    if let (Some(directional_light), Some(camera)) =
        (&extracted_lights.directional_light, main_camera)
    {
        let light_dir = Vec3::from_array(directional_light.direction).normalize();

        // 视锥体分割距离
        let near = 0.1;
        let far = 100.0;
        let cascade_splits = [near, 10.0, 35.0, far];

        let view_mat = Mat4::from_cols_array_2d(&camera.view);
        let proj_mat = Mat4::from_cols_array_2d(&camera.proj);
        let inv_cam = (proj_mat * view_mat).inverse();

        let mut camera_uniforms = Vec::new();
        let mut cascade_uniform = CascadeUniform::default();
        cascade_uniform.splits = [cascade_splits[1], cascade_splits[2], cascade_splits[3], 0.0];

        for i in 0..NUM_CASCADES {
            let split_near = cascade_splits[i];
            let split_far = cascade_splits[i + 1];

            // WGPU NDC 空间 Z 是 0.0 到 1.0
            let corners = [
                Vec3::new(-1.0, 1.0, 0.0),
                Vec3::new(1.0, 1.0, 0.0),
                Vec3::new(1.0, -1.0, 0.0),
                Vec3::new(-1.0, -1.0, 0.0),
                Vec3::new(-1.0, 1.0, 1.0),
                Vec3::new(1.0, 1.0, 1.0),
                Vec3::new(1.0, -1.0, 1.0),
                Vec3::new(-1.0, -1.0, 1.0),
            ];

            let mut world_corners = [Vec3::ZERO; 8];
            for j in 0..8 {
                let pt = inv_cam.project_point3(corners[j]);
                world_corners[j] = pt;
            }

            // 修正级联裁剪：根据分割距离重新计算世界坐标
            for j in 0..4 {
                let dir = world_corners[j + 4] - world_corners[j];
                world_corners[j + 4] = world_corners[j] + dir * (split_far / far);
                world_corners[j] = world_corners[j] + dir * (split_near / far);
            }

            // 稳定化级联：计算包围球中心
            let mut center = Vec3::ZERO;
            for j in 0..8 {
                center += world_corners[j];
            }
            center /= 8.0;

            // 稳定化级联：计算包围球半径
            let mut radius = 0.0f32;
            for j in 0..8 {
                let distance = (world_corners[j] - center).length();
                radius = radius.max(distance);
            }
            radius = (radius * 1.1).ceil(); // 稍微扩大并取整以稳定像素

            // 灯光相机观察矩阵：将眼睛退后足够远，以防遮挡物被切
            // 增加对垂直灯光方向的处理，防止 look_at_rh 产生 NaN
            let mut light_up = Vec3::Y;
            if light_dir.dot(light_up).abs() > 0.99 {
                light_up = Vec3::Z;
            }

            let light_view = Mat4::look_at_rh(center - light_dir * radius * 2.0, center, light_up);

            // glam::Mat4::orthographic_rh maps Z to [0, 1]
            let light_proj =
                Mat4::orthographic_rh(-radius, radius, -radius, radius, 0.0, radius * 4.0);
            let view_proj = light_proj * light_view;

            camera_uniforms.push(CameraUniform {
                view_position: [center.x, center.y, center.z, 1.0],
                view: light_view.to_cols_array_2d(),
                proj: light_proj.to_cols_array_2d(),
                view_proj: view_proj.to_cols_array_2d(),
                inv_proj: Mat4::IDENTITY.to_cols_array_2d(),
                ssao_enabled: 0,
                _pad: [0; 3],
            });

            cascade_uniform.view_proj[i] = view_proj.to_cols_array_2d();
            render_resources.cascade_view_projs[i] = view_proj;
        }

        // 写入缓冲区逻辑保持不变
        let mut shadow_camera_data = vec![0u8; shadow_camera_buffer_size as usize];
        for i in 0..NUM_CASCADES {
            let bytes = bytemuck::bytes_of(&camera_uniforms[i]);
            let offset = i * offset_unit as usize;
            shadow_camera_data[offset..offset + bytes.len()].copy_from_slice(bytes);
        }
        render_server.queue.write_buffer(
            render_resources
                .directional_shadow_camera_buffer
                .as_ref()
                .unwrap(),
            0,
            &shadow_camera_data,
        );

        render_server.queue.write_buffer(
            render_resources.cascade_uniform_buffer.as_ref().unwrap(),
            0,
            bytemuck::bytes_of(&cascade_uniform),
        );
    }

    // Point shadow logic
    let point_shadow_camera_buffer_size = offset_unit * (MAX_POINT_LIGHTS * 6) as u32;
    if render_resources.point_shadow_camera_buffer.is_none() {
        let buffer = render_server.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("point shadow camera buffer"),
            size: point_shadow_camera_buffer_size as BufferAddress,
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
                        size: Some(
                            wgpu::BufferSize::new(size_of::<CameraUniform>() as u64).unwrap(),
                        ),
                    }),
                }],
                label: Some("point shadow camera bind group"),
            });

        render_resources.point_shadow_camera_bind_group = Some(bind_group);
        render_resources.point_shadow_camera_buffer = Some(buffer);
    }

    let mut point_camera_uniforms = vec![CameraUniform::default(); MAX_POINT_LIGHTS * 6];
    render_resources.point_shadow_view_projs.clear();

    for (i, light) in extracted_lights.point_lights.iter().enumerate() {
        if i >= MAX_POINT_LIGHTS {
            break;
        }
        let light_pos = Vec3::from_array(light.position);
        let point_light_proj = wgpu_perspective(light.shadow_near, light.shadow_far);

        for face in 0..6 {
            let (target, up) = POINT_SHADOW_FACES[face];
            let light_view = Mat4::look_at_rh(light_pos, light_pos + target, up);
            let view_proj = point_light_proj * light_view;

            point_camera_uniforms[i * 6 + face] = CameraUniform {
                view_position: [light_pos.x, light_pos.y, light_pos.z, 1.0],
                view: light_view.to_cols_array_2d(),
                proj: point_light_proj.to_cols_array_2d(),
                view_proj: view_proj.to_cols_array_2d(),
                inv_proj: Mat4::IDENTITY.to_cols_array_2d(),
                ssao_enabled: 0,
                _pad: [0; 3],
            };
            render_resources.point_shadow_view_projs.push(view_proj);
        }
    }

    let mut point_shadow_camera_data = vec![0u8; point_shadow_camera_buffer_size as usize];
    for i in 0..(MAX_POINT_LIGHTS * 6) {
        let bytes = bytemuck::bytes_of(&point_camera_uniforms[i]);
        let offset = i * (offset_unit as usize);
        point_shadow_camera_data[offset..offset + bytes.len()].copy_from_slice(bytes);
    }
    render_server.queue.write_buffer(
        render_resources
            .point_shadow_camera_buffer
            .as_ref()
            .unwrap(),
        0,
        &point_shadow_camera_data,
    );

    if render_resources.directional_shadow_map.is_none() {
        let depth_texture = Texture::create_depth_texture_with_size(
            &render_server.device,
            texture_cache,
            2048,
            2048,
            NUM_CASCADES as u32,
            false,
            Some("directional shadow map array"),
        );
        render_resources.directional_shadow_map = Some(depth_texture);
    }

    if render_resources.point_shadow_map.is_none() {
        let depth_texture = Texture::create_depth_texture_with_size(
            &render_server.device,
            texture_cache,
            512,
            512,
            (MAX_POINT_LIGHTS * 6) as u32,
            true,
            Some("point shadow map cube array"),
        );
        render_resources.point_shadow_map = Some(depth_texture);
    }

    render_resources.prepare_pipeline(render_server, camera_render_resources);
}

pub(crate) fn render_shadow(
    encoder: &mut wgpu::CommandEncoder,
    texture_cache: &TextureCache,
    render_resources: &LightRenderResources,
    extracted_lights: &ExtractedLights,
    extracted_meshes: &Vec<ExtractedMesh>,
    mesh_cache: &MeshCache,
    mesh_render_resources: &MeshRenderResources,
    bvh: &Bvh,
) {
    if render_resources.pipeline.is_none() {
        return;
    }

    let pipeline = render_resources.pipeline.as_ref().unwrap();
    let offset_unit = CameraUniform::get_uniform_offset_unit();

    // Directional Shadow
    if let Some(shadow_map_id) = render_resources.directional_shadow_map {
        let shadow_map = texture_cache.get(shadow_map_id).unwrap();

        for i in 0..NUM_CASCADES {
            let cascade_view = shadow_map
                .texture
                .create_view(&wgpu::TextureViewDescriptor {
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
                label: Some("directional shadow render pass"),
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

            if let Some(bind_group) = &render_resources.directional_shadow_camera_bind_group {
                let dynamic_offset = (i as u32) * offset_unit;
                render_pass.set_bind_group(0, bind_group, &[dynamic_offset]);
            }

            let frustum = Frustum::from_view_proj(render_resources.cascade_view_projs[i]);

            let mut visible_indices = Vec::new();
            if bvh.root.is_some() {
                bvh.query(&frustum, &mut visible_indices);
            } else {
                visible_indices = (0..extracted_meshes.len()).collect();
            }

            for idx in visible_indices {
                let extracted = &extracted_meshes[idx];
                let mesh = mesh_cache.get(extracted.mesh_id).unwrap();

                // Frustum culling
                if bvh.root.is_none() {
                    let world_aabb = mesh.aabb.transform(&extracted.transform);
                    if !frustum.intersects_aabb(&world_aabb) {
                        continue;
                    }
                }

                let instance = mesh_render_resources
                    .instance_cache
                    .get(&extracted.mesh_id)
                    .unwrap();

                let instance_offset = *mesh_render_resources.instance_offsets.get(&idx).unwrap();

                render_pass.set_pipeline(pipeline);
                render_pass.set_vertex_buffer(1, instance.buffer.slice(..));
                render_pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                render_pass
                    .set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                render_pass.draw_indexed(0..mesh.index_count, 0, instance_offset..instance_offset + 1);
            }
        }
    }

    // Point Shadow
    if let Some(point_shadow_map_id) = render_resources.point_shadow_map {
        let shadow_map = texture_cache.get(point_shadow_map_id).unwrap();

        for i in 0..(extracted_lights.point_lights.len() * 6) {
            if i >= MAX_POINT_LIGHTS * 6 {
                break;
            }

            let face_view = shadow_map
                .texture
                .create_view(&wgpu::TextureViewDescriptor {
                    label: Some("point shadow face view"),
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
                label: Some("point shadow render pass"),
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &face_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            if let Some(bind_group) = &render_resources.point_shadow_camera_bind_group {
                let dynamic_offset = (i as u32) * offset_unit;
                render_pass.set_bind_group(0, bind_group, &[dynamic_offset]);
            }

            let frustum = Frustum::from_view_proj(render_resources.point_shadow_view_projs[i]);

            // BVH frustum culling
            let mut visible_indices = Vec::new();
            if bvh.root.is_some() {
                bvh.query(&frustum, &mut visible_indices);
            } else {
                visible_indices = (0..extracted_meshes.len()).collect();
            }

            for idx in visible_indices {
                let extracted = &extracted_meshes[idx];
                let mesh = mesh_cache.get(extracted.mesh_id).unwrap();

                // Simple frustum culling (if BVH is not available)
                if bvh.root.is_none() {
                    let world_aabb = mesh.aabb.transform(&extracted.transform);
                    if !frustum.intersects_aabb(&world_aabb) {
                        continue;
                    }
                }

                let instance = mesh_render_resources
                    .instance_cache
                    .get(&extracted.mesh_id)
                    .unwrap();

                let instance_offset = *mesh_render_resources.instance_offsets.get(&idx).unwrap();

                render_pass.set_pipeline(pipeline);
                render_pass.set_vertex_buffer(1, instance.buffer.slice(..));
                render_pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                render_pass
                    .set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                render_pass.draw_indexed(0..mesh.index_count, 0, instance_offset..instance_offset + 1);
            }
        }
    }
}

fn wgpu_perspective(near: f32, far: f32) -> Mat4 {
    // glam::Mat4::perspective_rh maps Z to [0, 1]
    Mat4::perspective_rh(90.0f32.to_radians(), 1.0f32, near, far)
}
