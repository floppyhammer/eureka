use crate::math::frustum::Frustum;
use crate::render::camera::{CameraType, CameraUniform};
use crate::render::light::{CascadeUniform, MAX_POINT_LIGHTS, NUM_CASCADES};
use crate::render::render_graph::{standard_resources, BufferKey, FrameContext, Node};
use crate::render::vertex::{Vertex3d, VertexBuffer};
use crate::render::{create_render_pipeline, InstanceRaw, Texture};
use glam::{Mat4, Vec3};
use std::any::Any;
use wgpu::BufferAddress;

pub struct ShadowNode {
    pipeline: Option<wgpu::RenderPipeline>,
}

impl Default for ShadowNode {
    fn default() -> Self {
        Self { pipeline: None }
    }
}

impl Node for ShadowNode {
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn node_resources(&self) -> crate::render::render_graph::resource::NodeResources {
        use crate::render::light::{MAX_POINT_LIGHTS, NUM_CASCADES};
        use crate::render::render_graph::resource::{ResourceSpec, TextureKey};
        use crate::render::render_graph::standard_resources;
        use crate::render::Texture;

        crate::render::render_graph::resource::NodeResources::new()
            .output(
                standard_resources::directional_shadow_map(),
                ResourceSpec::Texture(TextureKey {
                    width: 2048,
                    height: 2048,
                    format: Some(Texture::DEPTH_FORMAT),
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                        | wgpu::TextureUsages::TEXTURE_BINDING,
                    layers: NUM_CASCADES as u32,
                }),
            )
            .output(
                standard_resources::point_shadow_map(),
                ResourceSpec::Texture(TextureKey {
                    width: 512,
                    height: 512,
                    format: Some(Texture::DEPTH_FORMAT),
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                        | wgpu::TextureUsages::TEXTURE_BINDING,
                    layers: (MAX_POINT_LIGHTS * 6) as u32,
                }),
            )
            .output(
                standard_resources::shadow_cascade_buffer(),
                ResourceSpec::buffer(
                    size_of::<CascadeUniform>() as BufferAddress,
                    wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                ),
            )
    }

    fn prepare(&mut self, context: &mut FrameContext) {}

    fn run(&mut self, context: &mut FrameContext) {
        use crate::render::light::MAX_POINT_LIGHTS;
        use crate::render::render_graph::TextureKey;
        use crate::render::Texture;

        let device = &context.render_context.device;

        let camera_bind_group_layout = context
            .pool
            .get_bind_group_layout("camera_bind_group_layout")
            .unwrap()
            .clone();

        if self.pipeline.is_none() {
            let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("shadow pipeline layout"),
                bind_group_layouts: &[&camera_bind_group_layout],
                push_constant_ranges: &[],
            });

            let shader = wgpu::ShaderModuleDescriptor {
                label: Some("shadow shader"),
                source: wgpu::ShaderSource::Wgsl(
                    include_str!("../../../shaders/shadow.wgsl").into(),
                ),
            };

            let pipeline = create_render_pipeline(
                device,
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

        let first_d3_cam = context
            .render_world
            .extracted
            .cameras
            .types
            .iter()
            .position(|t| *t == CameraType::D3);

        if first_d3_cam == None {
            return;
        }

        let offset_unit = CameraUniform::get_uniform_offset_unit();

        let mut point_shadow_view_projs: Vec<Mat4> = Vec::new();

        let directional_shadow_camera_buffer_size = offset_unit * NUM_CASCADES as u32;
        let directional_shadow_camera_buffer = {
            let buffer_key = BufferKey {
                size: directional_shadow_camera_buffer_size as BufferAddress,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            };

            context.get_buffer_by_id(
                &standard_resources::directional_shadow_camera_buffer(),
                buffer_key,
            )
        };
        
        let cascade_uniform_buffer = context.buffer(&standard_resources::shadow_cascade_buffer());

        let mut cascade_view_projs = [Mat4::IDENTITY; NUM_CASCADES];

        // Prepare directional shadow uniforms
        if let (Some(directional_light), Some(camera_idx)) = (
            &context.render_world.extracted.lights.directional_light,
            first_d3_cam,
        ) {
            let camera_uniform = context.render_world.extracted.cameras.uniforms[camera_idx];
            let light_dir = Vec3::from_array(directional_light.direction).normalize();

            // 视锥体分割距离
            let near = 0.1;
            let far = 100.0;
            let cascade_splits = [near, 10.0, 35.0, far];

            let view_mat = Mat4::from_cols_array_2d(&camera_uniform.view);
            let proj_mat = Mat4::from_cols_array_2d(&camera_uniform.proj);
            let inv_cam = (proj_mat * view_mat).inverse();

            let mut shadow_camera_uniforms = Vec::new();
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

                let light_view =
                    Mat4::look_at_rh(center - light_dir * radius * 2.0, center, light_up);

                // glam::Mat4::orthographic_rh maps Z to [0, 1]
                let light_proj =
                    Mat4::orthographic_rh(-radius, radius, -radius, radius, 0.0, radius * 4.0);
                let view_proj = light_proj * light_view;

                shadow_camera_uniforms.push(CameraUniform {
                    view_position: [center.x, center.y, center.z, 1.0],
                    view: light_view.to_cols_array_2d(),
                    proj: light_proj.to_cols_array_2d(),
                    view_proj: view_proj.to_cols_array_2d(),
                    inv_proj: Mat4::IDENTITY.to_cols_array_2d(),
                    ssao_enabled: 0,
                    _pad: [0; 3],
                });

                cascade_uniform.view_proj[i] = view_proj.to_cols_array_2d();
                cascade_view_projs[i] = view_proj;
            }

            // 写入缓冲区逻辑保持不变
            let mut shadow_camera_data = vec![0u8; directional_shadow_camera_buffer_size as usize];
            for i in 0..NUM_CASCADES {
                let bytes = bytemuck::bytes_of(&shadow_camera_uniforms[i]);
                let offset = i * offset_unit as usize;
                shadow_camera_data[offset..offset + bytes.len()].copy_from_slice(bytes);
            }

            context.render_context.queue.write_buffer(
                &directional_shadow_camera_buffer.buffer,
                0,
                &shadow_camera_data,
            );

            context.render_context.queue.write_buffer(
                &cascade_uniform_buffer.buffer,
                0,
                bytemuck::bytes_of(&cascade_uniform),
            );
        }

        // Update point shadow buffers
        let point_shadow_camera_buffer_size = offset_unit * (MAX_POINT_LIGHTS * 6) as u32;

        let camera_bind_group_layout = context
            .pool
            .get_bind_group_layout("camera_bind_group_layout")
            .unwrap()
            .clone();

        let point_shadow_camera_buffer = {
            let buffer_key = BufferKey {
                size: point_shadow_camera_buffer_size as BufferAddress,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            };

            context.get_buffer_by_id(
                &standard_resources::point_shadow_camera_buffer(),
                buffer_key,
            )
        };

        let point_shadow_camera_bind_group = context.create_bind_group(
            &camera_bind_group_layout,
            vec![point_shadow_camera_buffer.id],
            |ctx| {
                ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    layout: &camera_bind_group_layout,
                    entries: &[wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                            buffer: &point_shadow_camera_buffer.buffer,
                            offset: 0,
                            size: Some(
                                wgpu::BufferSize::new(size_of::<CameraUniform>() as u64).unwrap(),
                            ),
                        }),
                    }],
                    label: Some("point shadow camera bind group"),
                })
            },
        );

        // Upload data
        {
            let mut point_camera_uniforms = vec![CameraUniform::default(); MAX_POINT_LIGHTS * 6];
            // render_resources.point_shadow_view_projs.clear();

            for (i, light) in context
                .render_world
                .extracted
                .lights
                .point_lights
                .iter()
                .enumerate()
            {
                if i >= MAX_POINT_LIGHTS {
                    break;
                }
                let light_pos = Vec3::from_array(light.position);
                let point_light_proj = wgpu_perspective(light.shadow_near, light.shadow_far);

                for face in 0..6 {
                    let (target, up) = crate::render::light::POINT_SHADOW_FACES[face];
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
                    // render_resources.point_shadow_view_projs.push(view_proj);
                }
            }

            let mut point_shadow_camera_data = vec![0u8; point_shadow_camera_buffer_size as usize];
            for i in 0..(MAX_POINT_LIGHTS * 6) {
                let bytes = bytemuck::bytes_of(&point_camera_uniforms[i]);
                let offset = i * (offset_unit as usize);
                point_shadow_camera_data[offset..offset + bytes.len()].copy_from_slice(bytes);
            }
            context.render_context.queue.write_buffer(
                &point_shadow_camera_buffer.buffer,
                0,
                &point_shadow_camera_data,
            );
        }

        let directional_shadow_map = context.texture(&standard_resources::directional_shadow_map());
        let point_shadow_map = context.texture(&standard_resources::point_shadow_map());

        let directional_shadow_camera_buffer_size =
            CameraUniform::get_uniform_offset_unit() * (NUM_CASCADES as u32);
        let directional_shadow_camera_buffer_key = BufferKey {
            size: directional_shadow_camera_buffer_size as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        };
        let directional_shadow_camera_buffer = context.get_buffer_by_id(
            &standard_resources::directional_shadow_camera_buffer(),
            directional_shadow_camera_buffer_key,
        );

        let directional_shadow_camera_bind_group = context.create_bind_group(
            &camera_bind_group_layout,
            vec![directional_shadow_camera_buffer.id],
            |ctx| {
                ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    layout: &camera_bind_group_layout,
                    entries: &[wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                            buffer: &directional_shadow_camera_buffer.buffer,
                            offset: 0,
                            size: Some(
                                wgpu::BufferSize::new(size_of::<CameraUniform>() as u64).unwrap(),
                            ),
                        }),
                    }],
                    label: Some("directional shadow camera bind group"),
                })
            },
        );

        let world = &*context.render_world;

        if world.mesh_render_resources.global_instance_buffer.is_none() {
            return;
        }

        let offset_unit = CameraUniform::get_uniform_offset_unit();

        // Draw directional shadow (multiple passes)
        for cascade_idx in 0..NUM_CASCADES {
            let cascade_view = directional_shadow_map.get_view(&wgpu::TextureViewDescriptor {
                label: Some("shadow cascade view"),
                format: Some(Texture::DEPTH_FORMAT),
                dimension: Some(wgpu::TextureViewDimension::D2),
                usage: Some(wgpu::TextureUsages::RENDER_ATTACHMENT),
                aspect: wgpu::TextureAspect::DepthOnly,
                base_mip_level: 0,
                mip_level_count: None,
                base_array_layer: cascade_idx as u32, // Change layer
                array_layer_count: Some(1),
            });

            let mut render_pass = context
                .encoder
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some(format!("directional shadow render pass {}", cascade_idx).as_str()),
                    color_attachments: &[],
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                        view: &cascade_view.0,
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Clear(1.0),
                            store: wgpu::StoreOp::Store,
                        }),
                        stencil_ops: None,
                    }),
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });

            let dynamic_offset = (cascade_idx as u32) * offset_unit;
            render_pass.set_bind_group(0, &directional_shadow_camera_bind_group, &[dynamic_offset]);

            let frustum = Frustum::from_view_proj(cascade_view_projs[cascade_idx]);

            let mut visible_indices = Vec::new();
            if world.extracted.bvh.root.is_some() {
                world.extracted.bvh.query(&frustum, &mut visible_indices);
            } else {
                visible_indices = (0..world.extracted.meshes.len()).collect();
            }

            render_pass.set_vertex_buffer(
                0,
                world
                    .mesh_render_resources
                    .mesh_allocator
                    .vertex_buffer
                    .slice(..),
            );
            render_pass.set_vertex_buffer(
                1,
                world
                    .mesh_render_resources
                    .global_instance_buffer
                    .as_ref()
                    .unwrap()
                    .slice(..),
            );
            render_pass.set_index_buffer(
                world
                    .mesh_render_resources
                    .mesh_allocator
                    .index_buffer
                    .slice(..),
                wgpu::IndexFormat::Uint32,
            );

            for info in &world.mesh_render_resources.mesh_infos {
                let mesh = world.mesh_cache.get(info.mesh_id).unwrap();

                render_pass.set_pipeline(self.pipeline.as_ref().unwrap());
                render_pass.draw_indexed(
                    mesh.index_offset..mesh.index_offset + mesh.index_count,
                    mesh.vertex_offset as i32,
                    info.base_instance..info.base_instance + info.instance_count,
                );
            }
        }

        // Draw point shadow
        {
            for light_layer_idx in 0..(world.extracted.lights.point_lights.len() * 6) {
                if light_layer_idx >= MAX_POINT_LIGHTS * 6 {
                    break;
                }

                let psm_face_view = point_shadow_map.get_view(&wgpu::TextureViewDescriptor {
                    label: Some("point shadow face view"),
                    format: Some(Texture::DEPTH_FORMAT),
                    dimension: Some(wgpu::TextureViewDimension::D2),
                    usage: Some(wgpu::TextureUsages::RENDER_ATTACHMENT),
                    aspect: wgpu::TextureAspect::DepthOnly,
                    base_mip_level: 0,
                    mip_level_count: None,
                    base_array_layer: light_layer_idx as u32,
                    array_layer_count: Some(1),
                });

                let mut render_pass =
                    context
                        .encoder
                        .begin_render_pass(&wgpu::RenderPassDescriptor {
                            label: Some("point shadow render pass"),
                            color_attachments: &[],
                            depth_stencil_attachment: Some(
                                wgpu::RenderPassDepthStencilAttachment {
                                    view: &psm_face_view.0,
                                    depth_ops: Some(wgpu::Operations {
                                        load: wgpu::LoadOp::Clear(1.0),
                                        store: wgpu::StoreOp::Store,
                                    }),
                                    stencil_ops: None,
                                },
                            ),
                            timestamp_writes: None,
                            occlusion_query_set: None,
                        });

                let dynamic_offset = (light_layer_idx as u32) * offset_unit;
                render_pass.set_bind_group(0, &point_shadow_camera_bind_group, &[dynamic_offset]);

                render_pass.set_vertex_buffer(
                    0,
                    world
                        .mesh_render_resources
                        .mesh_allocator
                        .vertex_buffer
                        .slice(..),
                );
                render_pass.set_vertex_buffer(
                    1,
                    world
                        .mesh_render_resources
                        .global_instance_buffer
                        .as_ref()
                        .unwrap()
                        .slice(..),
                );
                render_pass.set_index_buffer(
                    world
                        .mesh_render_resources
                        .mesh_allocator
                        .index_buffer
                        .slice(..),
                    wgpu::IndexFormat::Uint32,
                );

                for info in &world.mesh_render_resources.mesh_infos {
                    let mesh = world.mesh_cache.get(info.mesh_id).unwrap();

                    render_pass.set_pipeline(self.pipeline.as_ref().unwrap());
                    render_pass.draw_indexed(
                        mesh.index_offset..mesh.index_offset + mesh.index_count,
                        mesh.vertex_offset as i32,
                        info.base_instance..info.base_instance + info.instance_count,
                    );
                }
            }
        }
    }
}

fn wgpu_perspective(near: f32, far: f32) -> Mat4 {
    // glam::Mat4::perspective_rh maps Z to [0, 1]
    Mat4::perspective_rh(90.0f32.to_radians(), 1.0f32, near, far)
}
