use crate::render::render_graph::{Node, RenderContext};
use crate::render::camera::CameraType;
use crate::render::light::render_shadow;
use crate::render::atlas::render_atlas;
use crate::render::sprite::render_sprite;
use crate::render::sky::render_sky;
use crate::render::{render_meshes, prepare_meshes};

pub struct CullingNode;

impl Node for CullingNode {
    fn run(&mut self, context: &mut RenderContext) {
        let world = context.render_world;
        let resources = &world.mesh_render_resources;

        if let Some(pipeline) = &resources.cull_pipeline {
            if let Some(bind_group) = &resources.cull_bind_group {
                let mut compute_pass = context.encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("Global Culling Pass"),
                    timestamp_writes: None,
                });
                compute_pass.set_pipeline(pipeline);
                compute_pass.set_bind_group(0, bind_group, &[0]); // Camera offset

                // Dispatch based on total instance count across all meshes
                // We'll calculate total instances in prepare_instances or just use a large enough number
                let total_instances: u32 = world.extracted.meshes.len() as u32;
                if total_instances > 0 {
                    compute_pass.dispatch_workgroups((total_instances + 63) / 64, 1, 1);
                }
            }
        }
    }
}

pub struct ShadowNode;

impl Node for ShadowNode {
    fn run(&mut self, context: &mut RenderContext) {
        let world = context.render_world;
        render_shadow(
            context.encoder,
            &world.texture_cache,
            &world.light_render_resources,
            &world.extracted.lights,
            &world.extracted.meshes,
            &world.mesh_cache,
            &world.mesh_render_resources,
            &world.extracted.bvh,
        );
    }
}

pub struct SsaoNode;

impl Node for SsaoNode {
    fn run(&mut self, context: &mut RenderContext) {
        let world = context.render_world;
        if world.extracted.meshes.is_empty() {
            return;
        }

        // Check if any camera wants SSAO.
        let mut camera_wants_ssao = false;
        let mut ssao_camera_index = 0;
        for i in 0..world.extracted.cameras.types.len() {
            if world.extracted.cameras.types[i] == CameraType::D3 {
                if world.extracted.cameras.uniforms[i].ssao_enabled == 1 {
                    camera_wants_ssao = true;
                    ssao_camera_index = i;
                    break;
                }
            }
        }

        if !camera_wants_ssao {
            return;
        }

        let camera_bind_group = world.camera_render_resources.bind_group.as_ref().unwrap();

        let normal_view = &world
            .texture_cache
            .get(world.ssao_render_resources.normal_texture)
            .unwrap()
            .view;
        let depth_view = &world
            .texture_cache
            .get(world.surface_depth_texture)
            .unwrap()
            .view;

        // 1. Normal Pass
        {
            let mut render_pass = context.encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("SSAO Normal Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: normal_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            world.ssao_render_resources.render_normal(
                &mut render_pass,
                &world.extracted.meshes,
                &world.mesh_cache,
                &world.mesh_render_resources,
                camera_bind_group,
                ssao_camera_index,
                &world.extracted.cameras.uniforms[ssao_camera_index],
                &world.extracted.bvh,
            );
        }

        // 2. SSAO Pass
        {
            let ssao_view = &world
                .texture_cache
                .get(world.ssao_render_resources.ssao_texture)
                .unwrap()
                .view;
            let mut render_pass = context.encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("SSAO Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: ssao_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            render_pass.set_pipeline(&world.ssao_render_resources.ssao_pipeline);
            render_pass.set_bind_group(0, camera_bind_group, &[0]);
            render_pass.set_bind_group(1, &world.ssao_render_resources.ssao_bind_group, &[]);
            render_pass.draw(0..3, 0..1);
        }

        // 3. Blur Pass
        {
            let blur_view = &world
                .texture_cache
                .get(world.ssao_render_resources.blur_texture)
                .unwrap()
                .view;
            let mut render_pass = context.encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("SSAO Blur Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: blur_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            render_pass.set_pipeline(&world.ssao_render_resources.blur_pipeline);
            render_pass.set_bind_group(0, &world.ssao_render_resources.blur_bind_group, &[]);
            render_pass.draw(0..3, 0..1);
        }
    }
}

pub struct MainPassNode;

impl Node for MainPassNode {
    fn prepare(&mut self, context: &mut RenderContext) {
        let world = context.render_world;
        let skybox_texture_id = world.extracted.sky.as_ref().map(|sky| sky.texture);

        // This is a bit ugly because we are mutably borrowing world inside RenderContext
        // but for now we'll assume prepare_meshes can be called here.
        // Actually, RenderContext has &RenderWorld (immutable).
        // We need to fix RenderContext or the way we call prepare.
    }

    fn run(&mut self, context: &mut RenderContext) {
        let world = context.render_world;
        let depth_texture = world
            .texture_cache
            .get(world.surface_depth_texture)
            .unwrap();

        let ssao_ran = {
            let mut wants_ssao = false;
            for (i, cam_type) in world.extracted.cameras.types.iter().enumerate() {
                if *cam_type == CameraType::D3 {
                    if world.extracted.cameras.uniforms[i].ssao_enabled == 1 {
                        wants_ssao = true;
                        break;
                    }
                }
            }
            wants_ssao && !world.extracted.meshes.is_empty()
        };

        let mut render_pass = context.encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("main render pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: context.output_view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.1,
                        g: 0.2,
                        b: 0.3,
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &depth_texture.view,
                depth_ops: Some(wgpu::Operations {
                    load: if ssao_ran {
                        wgpu::LoadOp::Load
                    } else {
                        wgpu::LoadOp::Clear(1.0)
                    },
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        for i in 0..world.extracted.cameras.uniforms.len() {
            if world.extracted.cameras.types[i] == CameraType::D2 {
                render_atlas(
                    &world.extracted.atlases,
                    &world.atlas_render_resources,
                    &mut render_pass,
                );

                // Draw sprites.
                render_sprite(
                    &world.sprite_batches,
                    &world.sprite_render_resources,
                    &mut render_pass,
                    world.camera_render_resources.bind_group.as_ref().unwrap(),
                );
            } else {
                if world.camera_render_resources.bind_group.is_some() {
                    render_sky(
                        world.camera_render_resources.bind_group.as_ref().unwrap(),
                        &world.sky_render_resources,
                        &mut render_pass,
                        &world.mesh_render_resources.mesh_allocator,
                    );
                }

                render_meshes(
                    &world.extracted.meshes,
                    &world.mesh_cache,
                    &world.mesh_render_resources,
                    &world.camera_render_resources,
                    i,
                    &world.extracted.cameras.uniforms[i],
                    &world.gizmo_render_resources,
                    &mut render_pass,
                    &world.extracted.bvh,
                );
            }
        }
    }
}
