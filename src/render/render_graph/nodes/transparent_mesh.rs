use crate::render::camera::CameraType;
use crate::render::render_graph::{FrameContext, Node, TextureKey};
use crate::render::vertex::{Vertex3d, VertexBuffer};
use crate::render::{create_render_pipeline, InstanceRaw, Texture};
use crate::render::mesh::{ExtractedMesh, MeshCache, MeshRenderResources};
use crate::render::camera::{CameraRenderResources, CameraUniform};
use glam::{Mat4, Vec3};
use std::mem;

pub struct TransparentMeshNode {
    pipeline: Option<wgpu::RenderPipeline>,
    instance_buffer: Option<wgpu::Buffer>,
}

impl Default for TransparentMeshNode {
    fn default() -> Self {
        Self { 
            pipeline: None,
            instance_buffer: None,
        }
    }
}

impl Node for TransparentMeshNode {
    fn prepare(&mut self, context: &mut FrameContext) {
        if self.pipeline.is_some() {
            return;
        }
        let device = &context.render_context.device;
        let world = &*context.render_world;
        let resources = &world.mesh_render_resources;

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("transparent mesh layout"),
            bind_group_layouts: &[
                &world.camera_render_resources.bind_group_layout,
                &resources.light_bind_group_layout,
                &resources.bindless_bind_group_layout,
            ],
            push_constant_ranges: &[],
        });

        let shader = wgpu::ShaderModuleDescriptor {
            label: Some("transparent mesh shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../../../shaders/mesh.wgsl").into()),
        };

        self.pipeline = Some(create_render_pipeline(
            device,
            &pipeline_layout,
            Some(context.render_context.surface_config.format),
            Some(Texture::DEPTH_FORMAT),
            &[Vertex3d::desc(), InstanceRaw::desc()],
            shader,
            "transparent bindless",
            true,
            Some(wgpu::Face::Back),
        ));
    }

    fn run(&mut self, context: &mut FrameContext) {
        let width = context.render_context.surface_config.width;
        let height = context.render_context.surface_config.height;
        let format = context.render_context.surface_config.format;

        let main_color_key = TextureKey {
            width,
            height,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        };
        let main_depth_key = TextureKey {
            width,
            height,
            format: Texture::DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        };

        let main_color = context.get_texture("main_color", main_color_key);
        let main_depth = context.get_texture("main_depth", main_depth_key);

        let world = &*context.render_world;
        if world.extracted.transparent_meshes.is_empty() {
            return;
        }

        let mut render_pass = context
            .encoder
            .begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("transparent mesh render pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &main_color.view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &main_depth.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

        for i in 0..world.extracted.cameras.uniforms.len() {
            if world.extracted.cameras.types[i] == CameraType::D3 {
                render_transparent_meshes(
                    &world.extracted.transparent_meshes,
                    &world.mesh_cache,
                    &world.mesh_render_resources,
                    &world.camera_render_resources,
                    i,
                    &world.extracted.cameras.uniforms[i],
                    &mut render_pass,
                    self.pipeline.as_ref().unwrap(),
                    &context.render_context.device,
                    &context.render_context.queue,
                    &mut self.instance_buffer,
                );
            }
        }
    }
}

fn render_transparent_meshes<'a, 'b: 'a>(
    extracted_meshes: &'b Vec<ExtractedMesh>,
    mesh_cache: &'b MeshCache,
    mesh_render_resources: &'b MeshRenderResources,
    camera_render_resources: &'b CameraRenderResources,
    camera_index: usize,
    camera_uniform: &CameraUniform,
    render_pass: &mut wgpu::RenderPass<'a>,
    pipeline: &'b wgpu::RenderPipeline,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    instance_buffer: &mut Option<wgpu::Buffer>,
) {
    if camera_render_resources.bind_group.is_none()
        || mesh_render_resources.light_bind_group.is_none()
        || mesh_render_resources.bindless_bind_group.is_none()
    {
        return;
    }

    let view_proj = Mat4::from_cols_array_2d(&camera_uniform.view_proj);
    
    let mut sorted_meshes: Vec<_> = extracted_meshes.iter().enumerate().collect();
    sorted_meshes.sort_by(|(_, a), (_, b)| {
        let a_center = a.transform.position;
        let b_center = b.transform.position;
        let a_dist = (view_proj * Vec3::new(a_center.x, a_center.y, a_center.z).extend(1.0)).z;
        let b_dist = (view_proj * Vec3::new(b_center.x, b_center.y, b_center.z).extend(1.0)).z;
        b_dist.partial_cmp(&a_dist).unwrap_or(std::cmp::Ordering::Equal)
    });

    let mut sorted_instances: Vec<InstanceRaw> = Vec::new();
    let mut sorted_mesh_info: Vec<(MeshId, u32, u32)> = Vec::new();

    for (_, mesh) in sorted_meshes {
        if let Some(m) = mesh_cache.get(mesh.mesh_id) {
            let material_idx = mesh.material_id
                .and_then(|id| mesh_render_resources.material_index_map.get(&id))
                .cloned()
                .unwrap_or(0);

            let instance_raw = crate::render::mesh::Instance {
                position: mesh.transform.position,
                scale: mesh.transform.scale,
                rotation: mesh.transform.rotation,
                material_idx,
            }.to_raw();

            sorted_instances.push(instance_raw);
            sorted_mesh_info.push((mesh.mesh_id, m.index_offset, m.index_count));
        }
    }

    if sorted_instances.is_empty() {
        return;
    }

    let buffer_size = (sorted_instances.len() * mem::size_of::<InstanceRaw>()) as wgpu::BufferAddress;
    
    if instance_buffer.is_none() || instance_buffer.as_ref().unwrap().size() < buffer_size {
        *instance_buffer = Some(device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("transparent instance buffer"),
            size: buffer_size,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }));
    }

    queue.write_buffer(instance_buffer.as_ref().unwrap(), 0, bytemuck::cast_slice(&sorted_instances));

    render_pass.set_pipeline(pipeline);
    render_pass.set_bind_group(0, camera_render_resources.bind_group.as_ref().unwrap(), &[camera_index as u32 * CameraUniform::get_uniform_offset_unit()]);
    render_pass.set_bind_group(1, mesh_render_resources.light_bind_group.as_ref().unwrap(), &[]);
    render_pass.set_bind_group(2, mesh_render_resources.bindless_bind_group.as_ref().unwrap(), &[]);
    render_pass.set_vertex_buffer(0, mesh_render_resources.mesh_allocator.vertex_buffer.slice(..));
    render_pass.set_index_buffer(mesh_render_resources.mesh_allocator.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
    render_pass.set_vertex_buffer(1, instance_buffer.as_ref().unwrap().slice(..));

    let mut base_instance = 0u32;
    for (_, index_offset, index_count) in sorted_mesh_info {
        render_pass.draw_indexed(
            index_offset..index_offset + index_count,
            0,
            base_instance..base_instance + 1,
        );
        base_instance += 1;
    }
}

use crate::render::mesh::MeshId;