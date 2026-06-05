use crate::render::atlas::{prepare_atlas, AtlasRenderResources, ExtractedAtlas};
use crate::render::camera::{CameraRenderResources, CameraType, ExtractedCameras};
use crate::render::draw_command::DrawCommands;
use crate::render::gizmo::GizmoRenderResources;
use crate::render::light::{prepare_shadow, ExtractedLights, LightRenderResources};
use crate::render::shader_maker::ShaderMaker;
use crate::render::sky::{prepare_sky, ExtractedSky, SkyRenderResources};
use crate::render::sprite::{
    prepare_sprite, ExtractedSprite2d, SpriteBatch, SpriteRenderResources,
};
use crate::render::ssao::SsaoRenderResources;
use crate::render::{
    prepare_meshes, ExtractedMesh, MeshCache, MeshRenderResources, RenderContext,
    Texture, TextureCache, TextureId,
};
use crate::render::render_graph::{RenderGraph, ShadowNode, SsaoNode, CullingNode, SkyboxNode, ClearNode, MeshNode, SpriteNode};
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

pub struct RenderWorld {
    pub(crate) surface_depth_texture: TextureId,
    pub texture_cache: TextureCache,
    pub(crate) shader_maker: ShaderMaker,
    pub(crate) camera_render_resources: CameraRenderResources,
    pub(crate) sprite_render_resources: SpriteRenderResources,
    pub mesh_cache: MeshCache,
    pub mesh_render_resources: MeshRenderResources,
    pub(crate) light_render_resources: LightRenderResources,
    pub(crate) extracted: Extracted,
    pub(crate) sprite_batches: Vec<SpriteBatch>,
    pub gizmo_render_resources: GizmoRenderResources,
    pub atlas_render_resources: AtlasRenderResources,
    pub sky_render_resources: SkyRenderResources,
    pub ssao_render_resources: SsaoRenderResources,
    pub render_graph: RenderGraph,
}

impl RenderWorld {
    pub fn new(render_server: &RenderContext) -> Self {
        let mut texture_cache = TextureCache::new();
        let depth_texture = Texture::create_depth_texture(&render_server.device, &mut texture_cache, &render_server.surface_config, Some("surface depth texture"));
        let camera_render_resources = CameraRenderResources::new(render_server);
        let sprite_render_resources = SpriteRenderResources::new(render_server);
        let mesh_render_resources = MeshRenderResources::new(render_server);
        let gizmo_render_resources = GizmoRenderResources::new(render_server, &camera_render_resources.bind_group_layout);
        let atlas_render_resources = AtlasRenderResources::new(render_server);
        let sky_render_resources = SkyRenderResources::new(render_server);
        let light_render_resources = LightRenderResources::new();
        let ssao_render_resources = SsaoRenderResources::new(render_server, &mut texture_cache, &camera_render_resources, depth_texture);

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

    pub fn run_graph(&mut self, render_server: &RenderContext, encoder: &mut wgpu::CommandEncoder, output_view: &wgpu::TextureView) {
        let mut graph = std::mem::take(&mut self.render_graph);
        graph.run(render_server, self, encoder, output_view);
        self.render_graph = graph;
    }

    fn default_graph() -> RenderGraph {
        let mut graph = RenderGraph::new();
        graph.add_node("cull", CullingNode::default());
        graph.add_node("shadow", ShadowNode::default());
        graph.add_node("ssao", SsaoNode::default());
        graph.add_node("clear", ClearNode);
        graph.add_node("skybox", SkyboxNode::default());
        graph.add_node("mesh", MeshNode::default());
        graph.add_node("sprite", SpriteNode::default());

        graph.add_node_edge("cull", "shadow");
        graph.add_node_edge("cull", "mesh");
        graph.add_node_edge("shadow", "mesh");
        graph.add_node_edge("ssao", "mesh");
        graph.add_node_edge("clear", "skybox");
        graph.add_node_edge("skybox", "mesh");
        graph.add_node_edge("mesh", "sprite");
        graph
    }

    pub fn extract(&mut self, draw_commands: &DrawCommands) {
        self.extracted = draw_commands.extracted.clone();
    }

    pub fn prepare(&mut self, render_server: &RenderContext) {
        // 1. Prepare global camera data
        self.camera_render_resources.prepare_cameras(render_server, &self.extracted.cameras);

        // 2. Prepare Bindless Materials (Now includes sprite textures)
        self.mesh_render_resources.prepare_materials(&self.texture_cache, render_server, &self.extracted.sprites);

        // 3. Prepare 3D Mesh BVH
        if !self.extracted.meshes.is_empty() {
            let bvh_objects: Vec<_> = self.extracted.meshes.iter().enumerate().filter_map(|(i, ext)| {
                self.mesh_cache.get(ext.mesh_id).map(|mesh| (mesh.aabb.transform(&ext.transform), i))
            }).collect();
            self.extracted.bvh = Bvh::build(bvh_objects);
        }

        // 4. Prepare Sprites & Atlases
        self.sprite_batches = prepare_sprite(&self.extracted.sprites, &mut self.sprite_render_resources, &self.texture_cache, render_server, &self.mesh_render_resources);
        prepare_atlas(&self.extracted.atlases, &mut self.atlas_render_resources, render_server, &self.texture_cache, &mut self.shader_maker);

        // 5. Prepare 3D Meshes & Lights
        if !self.extracted.meshes.is_empty() || !self.extracted.lights.point_lights.is_empty() {
             prepare_meshes(&self.extracted.meshes, &self.extracted.lights, &self.texture_cache, &mut self.shader_maker, &mut self.mesh_render_resources, &self.light_render_resources, &self.camera_render_resources, render_server, &self.mesh_cache, self.ssao_render_resources.blur_texture, self.extracted.sky.as_ref().map(|s| s.texture));
        }

        // 6. Prepare Shadow & Sky
        let first_d3_cam = self.extracted.cameras.types.iter().position(|t| *t == CameraType::D3);
        if let Some(idx) = first_d3_cam {
            prepare_shadow(&self.extracted.lights, Some(&self.extracted.cameras.uniforms[idx]), render_server, &mut self.texture_cache, &mut self.light_render_resources, &self.camera_render_resources);
        }

        if let Some(sky) = &self.extracted.sky {
            prepare_sky(&mut self.sky_render_resources, render_server, &self.texture_cache, &sky.texture, &self.camera_render_resources.bind_group_layout, &mut self.mesh_render_resources.mesh_allocator);
        }
    }

    pub fn recreate_depth_texture(&mut self, render_server: &RenderContext) {
        self.texture_cache.remove(self.surface_depth_texture);
        self.surface_depth_texture = Texture::create_depth_texture(&render_server.device, &mut self.texture_cache, &render_server.surface_config, Some("surface depth texture"));
        self.ssao_render_resources.on_resize(&render_server.device, &mut self.texture_cache, render_server.surface_config.width, render_server.surface_config.height, self.surface_depth_texture);
    }
}
