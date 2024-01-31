use std::mem;
use wgpu::BufferAddress;
use crate::render::camera::{CameraRenderResources, CameraUniform};
use crate::render::gizmo::GizmoRenderResources;
use crate::render::{ExtractedMesh, MeshCache, MeshRenderResources, RenderServer, Texture, TextureCache, TextureId};
use crate::render::shader_maker::ShaderMaker;

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct PointLight {
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
pub(crate) struct DirectionalLight {
    pub(crate) direction: [f32; 3],
    pub(crate) strength: f32,
    pub(crate) color: [f32; 3],
    pub(crate) distance: f32,
}

#[derive(Debug, Default, Clone)]
pub(crate) struct ExtractedLights {
    pub(crate) point_lights: Vec<PointLight>,
    pub(crate) directional_light: Option<DirectionalLight>,
}

const MAX_POINT_LIGHTS: usize = 10;

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct LightUniform {
    // Due to uniforms requiring 16 byte (4 float) spacing, we need to use a padding field here.
    pub(crate) ambient_color: [f32; 3],
    pub(crate) ambient_strength: f32,
    pub(crate) directional_light: DirectionalLight,
    pub(crate) point_lights: [PointLight; MAX_POINT_LIGHTS],
    pub(crate) point_light_count: u32,
    pub(crate) _pad: [u32; 3],
}

struct LightRenderResources {
    shadow_map: Option<TextureId>,
    pipeline: Option<wgpu::RenderPipeline>,
    pub(crate) light_camera_bind_group: Option<wgpu::BindGroup>,
    pub(crate) light_camera_uniform_buffer: Option<wgpu::Buffer>,
}

pub(crate) fn prepare_shadow(
    extracted_lights: &Vec<LightUniform>,
    render_server: &RenderServer,
    texture_cache: &mut TextureCache,
    render_resources: &mut LightRenderResources,
    camera_render_resources: &CameraRenderResources,
) {
    let uniform_size = mem::size_of::<CameraUniform>();

    if render_resources.light_camera_uniform_buffer.is_none() {
        let buffer = render_server.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("light camera uniform buffer (unique)"),
            size: uniform_size as BufferAddress,
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
                            wgpu::BufferSize::new(uniform_size as u64)
                                .unwrap(),
                        ),
                    }),
                }],
                label: Some("light camera bind group (unique)"),
            });

        render_resources.light_camera_bind_group = Some(bind_group);
        render_resources.light_camera_uniform_buffer = Some(buffer);
    }

    for l in extracted_lights {
        // A camera from the light's perspective.
        let camera_uniform = CameraUniform::default();

        // TODO: fill the camera uniform.

        // Write the instance buffer.
        render_server.queue.write_buffer(
            render_resources.light_camera_uniform_buffer.as_ref().unwrap(),
            0,
            bytemuck::cast_slice(&[camera_uniform]),
        );
    }

    if (render_resources.shadow_map.is_none()) {
        let depth_texture = Texture::create_depth_texture(
            &render_server.device,
            texture_cache,
            &render_server.surface_config,
            Some("shadow map"),
        );

        render_resources.shadow_map = Some(depth_texture);
    }
}

pub(crate) fn render_shadow(
    encoder: &mut wgpu::CommandEncoder,
    texture_cache: &TextureCache,
    render_resources: &LightRenderResources,
    extracted_meshes: &Vec<ExtractedMesh>,
    mesh_cache: &MeshCache,
    mesh_render_resources: &MeshRenderResources,
) {
    if render_resources.shadow_map.is_none() {
        return;
    }

    let shadow_map = texture_cache.get(render_resources.shadow_map.unwrap());

    let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("shadow render pass"),
        color_attachments: &[],
        depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
            view: &shadow_map.unwrap().view,
            depth_ops: Some(wgpu::Operations {
                load: wgpu::LoadOp::Clear(1.0),
                store: wgpu::StoreOp::Store,
            }),
            stencil_ops: None,
        }),
        timestamp_writes: None,
        occlusion_query_set: None,
    });

    if (render_resources.light_camera_bind_group.is_none()) {
        return;
    }
    if (mesh_render_resources.light_bind_group.is_none()) {
        return;
    }

    let light_camera_bind_group = render_resources.light_camera_bind_group.as_ref().unwrap();

    for extracted in extracted_meshes {
        let pipeline = render_resources.pipeline.as_ref().unwrap();

        let mesh = mesh_cache.get(extracted.mesh_id).unwrap();

        let instance = mesh_render_resources
            .instance_cache
            .get(&extracted.mesh_id)
            .unwrap();

        render_pass.set_pipeline(pipeline);
        // Set vertex buffer for InstanceInput.
        render_pass.set_vertex_buffer(1, instance.buffer.slice(..));

        // Set vertex buffer for VertexInput.
        render_pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));

        render_pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);

        // Set light camera uniform.
        render_pass.set_bind_group(0, light_camera_bind_group, &[]);

        render_pass.draw_indexed(0..mesh.index_count, 0, 0..1);
    }
}
