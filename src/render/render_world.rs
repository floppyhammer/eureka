use crate::asset::AssetServer;
use crate::core::engine::Engine;
use crate::math::alignup_u32;
use crate::render::bind_group::BindGroupCache;
use crate::render::camera::{CameraRenderResources, CameraUniform};
use crate::render::draw_command::DrawCommands;
use crate::render::gizmo::GizmoRenderResources;
use crate::render::shader_maker::ShaderMaker;
use crate::render::sprite::{ExtractedSprite2d, prepare_sprite, render_sprite, SpriteBatch, SpriteRenderResources};
use crate::render::{
    DrawModel, ExtractedMesh, MeshCache, MeshRenderResources, RenderServer, Texture, TextureCache,
    TextureId,
};
use crate::scene::{Camera2d, LightUniform, World};
use crate::window::InputServer;
use crate::{App, Singletons, INITIAL_WINDOW_HEIGHT, INITIAL_WINDOW_WIDTH};
use cgmath::Point2;
use std::mem;
use wgpu::{BufferAddress, DynamicOffset, SamplerBindingType};
use winit::event_loop::{EventLoop, EventLoopWindowTarget};
use winit::window::{Window, WindowBuilder};
use crate::render::atlas::{AtlasRenderResources, ExtractedAtlas, prepare_atlas, render_atlas};
use crate::render::sky::{ExtractedSky, prepare_sky, render_sky, SkyRenderResources};

#[derive(Default, Clone)]
pub struct Extracted {
    pub(crate) sprites: Vec<ExtractedSprite2d>,

    pub(crate) meshes: Vec<ExtractedMesh>,

    // Only for 3D. Only one for now.
    pub(crate) cameras: Vec<CameraUniform>,

    pub(crate) lights: Vec<LightUniform>,

    pub(crate) atlases: Vec<ExtractedAtlas>,

    pub(crate) sky: Option<ExtractedSky>,
}

/// Contains GPU resources
pub struct RenderWorld {
    // Common resources.
    pub(crate) surface_depth_texture: TextureId,
    pub texture_cache: TextureCache,
    pub(crate) shader_maker: ShaderMaker,
    pub camera_render_resources: CameraRenderResources,

    // Sprites.
    pub(crate) sprite_render_resources: SpriteRenderResources,

    // Meshes.
    pub mesh_cache: MeshCache,
    pub mesh_render_resources: MeshRenderResources,

    // Temporary.
    pub(crate) extracted: Extracted,
    pub(crate) sprite_batches: Vec<SpriteBatch>,

    // Cameras.

    // Lights.

    // Extra.
    pub gizmo_render_resources: GizmoRenderResources,

    pub atlas_render_resources: AtlasRenderResources,

    pub sky_render_resources: SkyRenderResources,
}

impl RenderWorld {
    pub fn new(render_server: &RenderServer) -> Self {
        let mut texture_cache = TextureCache::new();

        // Depth texture for depth test.
        let depth_texture = Texture::create_depth_texture(
            &render_server.device,
            &mut texture_cache,
            &render_server.surface_config,
            Some("surface depth texture"),
        );

        let camera_render_resources = CameraRenderResources::new(render_server);

        let sprite_render_resources = SpriteRenderResources::new(render_server);

        let mesh_render_resources = MeshRenderResources::new(render_server);

        let gizmo_render_resources = GizmoRenderResources::new(
            render_server,
            &camera_render_resources.bind_group_layout,
        );

        let atlas_render_resources = AtlasRenderResources::new(
            render_server,
        );

        let sky_render_resources = SkyRenderResources::new(
            render_server,
        );

        Self {
            surface_depth_texture: depth_texture,
            texture_cache,
            mesh_cache: MeshCache::new(),
            camera_render_resources,
            sprite_render_resources,
            mesh_render_resources,
            shader_maker: ShaderMaker::new(),
            extracted: Extracted::default(),
            sprite_batches: vec![],
            gizmo_render_resources,
            atlas_render_resources,
            sky_render_resources,
        }
    }

    pub fn extract(&mut self, draw_commands: &DrawCommands) {
        self.extracted = draw_commands.extracted.clone();
    }

    // Prepare GPU resources.
    pub fn prepare(&mut self, render_server: &RenderServer) {
        self.camera_render_resources.prepare_cameras(render_server, &self.extracted.cameras);

        self.sprite_batches = prepare_sprite(&self.extracted.sprites, &mut self.sprite_render_resources, &self.texture_cache, render_server, &self.camera_render_resources.bind_group_layout);

        self.prepare_meshes(render_server);

        prepare_atlas(&self.extracted.atlases, &mut self.atlas_render_resources, render_server, &self.texture_cache, &mut self.shader_maker);

        if (self.extracted.sky.is_some()) {
            prepare_sky(&mut self.sky_render_resources, render_server, &self.texture_cache, &self.extracted.sky.unwrap().texture, &self.camera_render_resources.bind_group_layout);
        }
    }

    // Send draw calls.
    pub(crate) fn render<'a, 'b: 'a>(
        &'b self,
        render_pass: &mut wgpu::RenderPass<'a>,
    ) {
        if (self.camera_render_resources.bind_group.is_some()) {
            render_sky(self.camera_render_resources.bind_group.as_ref().unwrap(), &self.sky_render_resources, render_pass);
        }

        // Draw sprites.
        render_sprite(&self.sprite_batches, &self.sprite_render_resources, render_pass, self.camera_render_resources.bind_group.as_ref().unwrap());

        self.render_meshes(render_pass);

        render_atlas(&self.extracted.atlases, &self.atlas_render_resources, render_pass);
    }

    pub(crate) fn prepare_meshes(&mut self, render_server: &RenderServer) {
        //
        // // Copy data from [Instance] to [InstanceRaw].
        // let instance_data = instances.iter().map(Instance::to_raw).collect::<Vec<_>>();
        //
        // // Create the instance buffer.
        // let instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        //     label: Some("model instance buffer"),
        //     contents: bytemuck::cast_slice(&instance_data),
        //     usage: wgpu::BufferUsages::VERTEX,
        // });

        for mesh in &self.extracted.meshes {
            self.mesh_render_resources
                .prepare_materials(&self.texture_cache, render_server);

            self.mesh_render_resources.prepare_pipeline(
                render_server,
                &mut self.shader_maker,
                &self.camera_render_resources.bind_group_layout,
                mesh.material_id,
            );
        }

        for light in &self.extracted.lights {
            self.mesh_render_resources
                .prepare_lights(render_server, *light);
        }

        self.mesh_render_resources
            .prepare_instances(render_server, &self.extracted.meshes);
    }

    pub(crate) fn render_meshes<'a, 'b: 'a>(&'b self, render_pass: &mut wgpu::RenderPass<'a>) {
        if (self.camera_render_resources.bind_group.is_none()) {
            return;
        }
        if (self.mesh_render_resources.light_bind_group.is_none()) {
            return;
        }

        let camera_bind_group = self
            .camera_render_resources
            .bind_group
            .as_ref()
            .unwrap();

        let light_bind_group = self
            .mesh_render_resources
            .light_bind_group
            .as_ref()
            .unwrap();

        for extracted in &self.extracted.meshes {
            let mut texture_bind_group = None;
            let mut flags = 0;

            if (extracted.material_id.is_some()) {
                let material_id = &extracted.material_id.unwrap();

                texture_bind_group = Some(
                    self.mesh_render_resources
                        .texture_bind_group_cache
                        .get(material_id)
                        .unwrap(),
                );

                let material = self
                    .mesh_render_resources
                    .material_cache
                    .get(material_id)
                    .unwrap();
                flags = material.get_flags();
            }

            let pipeline = self
                .mesh_render_resources
                .pipeline_cache
                .get(&flags)
                .unwrap();

            let mesh = self.mesh_cache.get(extracted.mesh_id).unwrap();

            let instance = self
                .mesh_render_resources
                .instance_cache
                .get(&extracted.mesh_id)
                .unwrap();

            render_pass.set_pipeline(pipeline);
            // Set vertex buffer for InstanceInput.
            render_pass.set_vertex_buffer(1, instance.buffer.slice(..));

            // Set vertex buffer for VertexInput.
            render_pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));

            render_pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);

            // FIXME
            // Set camera uniform.
            render_pass.set_bind_group(0, camera_bind_group, &[0]);

            // Set light uniform.
            render_pass.set_bind_group(1, light_bind_group, &[]);

            // Set textures.
            if (texture_bind_group.is_some()) {
                render_pass.set_bind_group(2, texture_bind_group.unwrap(), &[]);
            }

            render_pass.draw_indexed(0..mesh.index_count, 0, 0..1);
        }

        self.gizmo_render_resources.render(
            render_pass,
            self.camera_render_resources
                .bind_group
                .as_ref()
                .unwrap(),
        );
    }

    pub fn recreate_depth_texture(&mut self, render_server: &RenderServer) {
        // Remove the previous depth texture.
        self.texture_cache.remove(self.surface_depth_texture);

        // Create a new depth_texture and depth_texture_view.
        // Make sure you update the depth_texture after you update config.
        // If you don't, your program will crash as the depth_texture will be a different size than the surface texture.
        self.surface_depth_texture = Texture::create_depth_texture(
            &render_server.device,
            &mut self.texture_cache,
            &render_server.surface_config,
            Some("surface depth texture"),
        );
    }
}
