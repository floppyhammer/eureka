use crate::render::camera::{CameraRenderResources, CameraUniform};
use crate::render::gizmo::GizmoRenderResources;
use crate::render::shader_maker::ShaderMaker;
use crate::render::vertex::{Vertex3d, VertexBuffer};
use crate::render::{create_render_pipeline, ExtractedMesh, InstanceRaw, MeshCache, MeshRenderResources, RenderServer, Texture, TextureCache, TextureId};
use crate::scene::OPENGL_TO_WGPU_MATRIX;
use cgmath::{EuclideanSpace, InnerSpace, Matrix4, Point3, Vector3};
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

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct LightUniform {
    // Due to uniforms requiring 16 byte (4 float) spacing, we need to use a padding field here.
    pub(crate) ambient_color: [f32; 3],
    pub(crate) ambient_strength: f32,
    pub(crate) directional_light: DirectionalLightUniform,
    pub(crate) point_lights: [PointLightUniform; MAX_POINT_LIGHTS],
    pub(crate) point_light_count: u32,
    pub(crate) _pad: [u32; 3],
}

pub(crate) struct LightRenderResources {
    pub(crate) shadow_map: Option<TextureId>,
    pub(crate) pipeline: Option<wgpu::RenderPipeline>,
    pub(crate) light_camera_bind_group: Option<wgpu::BindGroup>,
    pub(crate) light_camera_uniform_buffer: Option<wgpu::Buffer>,
}

impl LightRenderResources {
    pub(crate) fn new() -> Self {
        Self {
            shadow_map: None,
            pipeline: None,
            light_camera_bind_group: None,
            light_camera_uniform_buffer: None,
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
    render_server: &RenderServer,
    texture_cache: &mut TextureCache,
    render_resources: &mut LightRenderResources,
    camera_render_resources: &CameraRenderResources,
) {
    let uniform_size = mem::size_of::<CameraUniform>();

    if render_resources.light_camera_uniform_buffer.is_none() {
        let buffer = render_server.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("light camera uniform buffer"),
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
                        size: Some(wgpu::BufferSize::new(uniform_size as u64).unwrap()),
                    }),
                }],
                label: Some("light camera bind group"),
            });

        render_resources.light_camera_bind_group = Some(bind_group);
        render_resources.light_camera_uniform_buffer = Some(buffer);
    }

    if let Some(directional_light) = &extracted_lights.directional_light {
        let light_dir = Vector3::from(directional_light.direction).normalize();
        let light_position = light_dir * -directional_light.distance;

        let view = Matrix4::look_at_rh(
            Point3::from_vec(light_position),
            Point3::new(0.0, 0.0, 0.0),
            Vector3::unit_y(),
        );

        // Simple ortho projection for shadow mapping.
        let ortho_size = 20.0;
        let proj = cgmath::ortho(
            -ortho_size,
            ortho_size,
            -ortho_size,
            ortho_size,
            0.1,
            directional_light.distance * 2.0,
        );

        let view_proj = OPENGL_TO_WGPU_MATRIX * proj * view;

        let camera_uniform = CameraUniform {
            view_position: [light_position.x, light_position.y, light_position.z, 1.0],
            view: view.into(),
            proj: proj.into(),
            view_proj: view_proj.into(),
        };

        render_server.queue.write_buffer(
            render_resources
                .light_camera_uniform_buffer
                .as_ref()
                .unwrap(),
            0,
            bytemuck::cast_slice(&[camera_uniform]),
        );
    }

    if render_resources.shadow_map.is_none() {
        let depth_texture = Texture::create_depth_texture(
            &render_server.device,
            texture_cache,
            &render_server.surface_config,
            Some("shadow map"),
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

    let shadow_map = texture_cache.get(render_resources.shadow_map.unwrap());

    {
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

        if render_resources.light_camera_bind_group.is_none() {
            return;
        }

        let light_camera_bind_group = render_resources.light_camera_bind_group.as_ref().unwrap();

        // Set light camera uniform. (Uses dynamic offset from camera layout)
        render_pass.set_bind_group(0, light_camera_bind_group, &[0]);

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
