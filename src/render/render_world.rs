use crate::render::atlas::{prepare_atlas, render_atlas, AtlasRenderResources, ExtractedAtlas};
use crate::render::camera::{CameraRenderResources, CameraType, ExtractedCameras};
use crate::render::draw_command::DrawCommands;
use crate::render::gizmo::GizmoRenderResources;
use crate::render::light::{prepare_shadow, render_shadow, ExtractedLights, LightRenderResources};
use crate::render::shader_maker::ShaderMaker;
use crate::render::sky::{prepare_sky, render_sky, ExtractedSky, SkyRenderResources};
use crate::render::sprite::{
    prepare_sprite, render_sprite, ExtractedSprite2d, SpriteBatch, SpriteRenderResources,
};
use crate::render::ssao::SsaoRenderResources;
use crate::render::{
    prepare_meshes, render_meshes, ExtractedMesh, MeshCache, MeshRenderResources, RenderServer,
    Texture, TextureCache, TextureId,
};
use crate::render::render_graph::{MainPassNode, RenderGraph, ShadowNode, SsaoNode, CullingNode};
use crate::scene::Bvh;

#[derive(Default, Clone)]
pub struct Extracted {
    pub(crate) sprites: Vec<ExtractedSprite2d>,

    pub(crate) meshes: Vec<ExtractedMesh>,

    pub(crate) bvh: Bvh,

    pub(crate) cameras: ExtractedCameras,

    pub(crate) lights: ExtractedLights,

    pub(crate) atlases: Vec<ExtractedAtlas>,

    pub(crate) sky: Option<ExtractedSky>,
}

/// Contains GPU resources
pub struct RenderWorld {
    // Common resources.
    pub(crate) surface_depth_texture: TextureId,
    pub texture_cache: TextureCache,
    pub(crate) shader_maker: ShaderMaker,
    pub(crate) camera_render_resources: CameraRenderResources,

    // Sprites.
    pub(crate) sprite_render_resources: SpriteRenderResources,

    // Meshes.
    pub mesh_cache: MeshCache,
    pub mesh_render_resources: MeshRenderResources,

    // Lights.
    pub(crate) light_render_resources: LightRenderResources,

    // Temporary.
    pub(crate) extracted: Extracted,
    pub(crate) sprite_batches: Vec<SpriteBatch>,

    // Cameras.

    // Lights.

    // Extra.
    pub gizmo_render_resources: GizmoRenderResources,

    pub atlas_render_resources: AtlasRenderResources,

    pub sky_render_resources: SkyRenderResources,

    pub ssao_render_resources: SsaoRenderResources,

    pub render_graph: RenderGraph,
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

        let light_render_resources = LightRenderResources::new();

        let ssao_render_resources = SsaoRenderResources::new(
            render_server,
            &mut texture_cache,
            &camera_render_resources,
            depth_texture,
        );

        Self {
            surface_depth_texture: depth_texture,
            texture_cache,
            mesh_cache: MeshCache::new(),
            camera_render_resources,
            sprite_render_resources,
            mesh_render_resources,
            light_render_resources,
            shader_maker: ShaderMaker::new(),
            extracted: Extracted::default(),
            sprite_batches: vec![],
            gizmo_render_resources,
            atlas_render_resources,
            sky_render_resources,
            ssao_render_resources,
            render_graph: Self::default_graph(),
        }
    }

    pub fn run_graph(
        &mut self,
        render_server: &RenderServer,
        encoder: &mut wgpu::CommandEncoder,
        output_view: &wgpu::TextureView,
    ) {
        let mut graph = std::mem::take(&mut self.render_graph);
        graph.run(render_server, self, encoder, output_view);
        self.render_graph = graph;
    }

    fn default_graph() -> RenderGraph {
        let mut graph = RenderGraph::new();
        graph.add_node("cull", CullingNode::default());
        graph.add_node("shadow", ShadowNode::default());
        graph.add_node("ssao", SsaoNode::default());
        graph.add_node("main", MainPassNode);

        graph.add_node_edge("cull", "shadow");
        graph.add_node_edge("cull", "main");
        graph.add_node_edge("shadow", "main");
        graph.add_node_edge("ssao", "main");
        graph
    }

    pub fn extract(&mut self, draw_commands: &DrawCommands) {
        self.extracted = draw_commands.extracted.clone();
    }

    // Prepare GPU resources.
    pub fn prepare(&mut self, render_server: &RenderServer) {
        self.camera_render_resources
            .prepare_cameras(render_server, &self.extracted.cameras);

        // Build BVH for 3D meshes once per frame.
        if !self.extracted.meshes.is_empty() {
            let mut bvh_objects = Vec::with_capacity(self.extracted.meshes.len());
            for (i, extracted) in self.extracted.meshes.iter().enumerate() {
                if let Some(mesh) = self.mesh_cache.get(extracted.mesh_id) {
                    let world_aabb = mesh.aabb.transform(&extracted.transform);
                    bvh_objects.push((world_aabb, i));
                }
            }
            self.extracted.bvh = Bvh::build(bvh_objects);
        }

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
                let skybox_texture_id = self.extracted.sky.as_ref().map(|sky| sky.texture);

                prepare_meshes(
                    &self.extracted.meshes,
                    &self.extracted.lights,
                    &self.texture_cache,
                    &mut self.shader_maker,
                    &mut self.mesh_render_resources,
                    &self.light_render_resources,
                    &self.camera_render_resources,
                    &render_server,
                    &self.mesh_cache,
                    self.ssao_render_resources.blur_texture,
                    skybox_texture_id,
                );

                let main_camera = if self.extracted.cameras.uniforms.len() > i {
                    Some(&self.extracted.cameras.uniforms[i])
                } else {
                    None
                };

                prepare_shadow(
                    &self.extracted.lights,
                    main_camera,
                    render_server,
                    &mut self.texture_cache,
                    &mut self.light_render_resources,
                    &self.camera_render_resources,
                );

                if self.extracted.sky.is_some() {
                    prepare_sky(
                        &mut self.sky_render_resources,
                        render_server,
                        &self.texture_cache,
                        &self.extracted.sky.unwrap().texture,
                        &self.camera_render_resources.bind_group_layout,
                        &mut self.mesh_render_resources.mesh_allocator,
                    );
                }
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

        self.ssao_render_resources.on_resize(
            &render_server.device,
            &mut self.texture_cache,
            render_server.surface_config.width,
            render_server.surface_config.height,
            self.surface_depth_texture,
        );
    }
}
