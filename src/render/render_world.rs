use crate::asset::AssetServer;
use crate::core::engine::Engine;
use crate::math::alignup_u32;
use crate::render::atlas::{prepare_atlas, render_atlas, AtlasRenderResources, ExtractedAtlas};
use crate::render::bind_group::BindGroupCache;
use crate::render::camera::{CameraRenderResources, CameraType, CameraUniform, ExtractedCameras};
use crate::render::draw_command::DrawCommands;
use crate::render::gizmo::GizmoRenderResources;
use crate::render::shader_maker::ShaderMaker;
use crate::render::sky::{prepare_sky, render_sky, ExtractedSky, SkyRenderResources};
use crate::render::sprite::{
    prepare_sprite, render_sprite, ExtractedSprite2d, SpriteBatch, SpriteRenderResources,
};
use crate::render::{
    prepare_meshes, render_meshes, DrawModel, ExtractedMesh, MeshCache, MeshRenderResources,
    RenderServer, Texture, TextureCache, TextureId,
};
use crate::scene::{Camera2d, LightUniform, World};
use crate::window::InputServer;
use crate::{App, Singletons, INITIAL_WINDOW_HEIGHT, INITIAL_WINDOW_WIDTH};
use cgmath::Point2;
use std::mem;
use wgpu::{BufferAddress, DynamicOffset, SamplerBindingType};
use winit::event_loop::{EventLoop, EventLoopWindowTarget};
use winit::window::{Window, WindowBuilder};

#[derive(Default, Clone)]
pub struct Extracted {
    pub(crate) sprites: Vec<ExtractedSprite2d>,

    pub(crate) meshes: Vec<ExtractedMesh>,

    pub(crate) cameras: ExtractedCameras,

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

        let gizmo_render_resources =
            GizmoRenderResources::new(render_server, &camera_render_resources.bind_group_layout);

        let atlas_render_resources = AtlasRenderResources::new(render_server);

        let sky_render_resources = SkyRenderResources::new(render_server);

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
        self.camera_render_resources
            .prepare_cameras(render_server, &self.extracted.cameras);

        for i in 0..self.extracted.cameras.uniforms.len() {
            if self.extracted.cameras.types[i] == CameraType::D2 {
                self.sprite_batches = prepare_sprite(
                    &self.extracted.sprites,
                    &mut self.sprite_render_resources,
                    &self.texture_cache,
                    render_server,
                    &self.camera_render_resources.bind_group_layout,
                );

                prepare_atlas(
                    &self.extracted.atlases,
                    &mut self.atlas_render_resources,
                    render_server,
                    &self.texture_cache,
                    &mut self.shader_maker,
                );
            } else {
                prepare_meshes(
                    &self.extracted.meshes,
                    &self.extracted.lights,
                    &self.texture_cache,
                    &mut self.shader_maker,
                    &mut self.mesh_render_resources,
                    &self.camera_render_resources,
                    &render_server,
                );

                if (self.extracted.sky.is_some()) {
                    prepare_sky(
                        &mut self.sky_render_resources,
                        render_server,
                        &self.texture_cache,
                        &self.extracted.sky.unwrap().texture,
                        &self.camera_render_resources.bind_group_layout,
                    );
                }
            }
        }
    }

    // Send draw calls.
    pub(crate) fn render<'a, 'b: 'a>(&'b self, render_pass: &mut wgpu::RenderPass<'a>) {
        for i in 0..self.extracted.cameras.uniforms.len() {
            if self.extracted.cameras.types[i] == CameraType::D2 {
                render_atlas(
                    &self.extracted.atlases,
                    &self.atlas_render_resources,
                    render_pass,
                );

                // Draw sprites.
                render_sprite(
                    &self.sprite_batches,
                    &self.sprite_render_resources,
                    render_pass,
                    self.camera_render_resources.bind_group.as_ref().unwrap(),
                );
            } else {
                if (self.camera_render_resources.bind_group.is_some()) {
                    render_sky(
                        self.camera_render_resources.bind_group.as_ref().unwrap(),
                        &self.sky_render_resources,
                        render_pass,
                    );
                }

                render_meshes(
                    &self.extracted.meshes,
                    &self.mesh_cache,
                    &self.mesh_render_resources,
                    &self.camera_render_resources,
                    &self.gizmo_render_resources,
                    render_pass,
                );
            }
        }
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
