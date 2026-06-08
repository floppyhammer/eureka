use crate::render::atlas::AtlasRenderResources;
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
use crate::render::render_graph::{RenderGraph, ShadowNode, SsaoNode, CullingNode, SkyboxNode, ClearNode, MeshNode, SpriteNode, FxaaNode, TransparentMeshNode, PresentNode};
use crate::render::render_graph::standard_resources;
use crate::scene::Bvh;

#[derive(Default, Clone)]
pub struct Extracted {
    pub(crate) sprites_2d: Vec<ExtractedSprite2d>,
    pub(crate) meshes: Vec<ExtractedMesh>,
    pub(crate) transparent_meshes: Vec<ExtractedMesh>,
    pub(crate) bvh: Bvh,
    pub(crate) cameras: ExtractedCameras,
    pub(crate) lights: ExtractedLights,
    pub(crate) sky: Option<ExtractedSky>,
    pub fxaa_enabled: bool,
    pub ssao_enabled: bool,
}

pub struct RenderWorld {
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
    pub sky_render_resources: SkyRenderResources,
    pub ssao_render_resources: SsaoRenderResources,
    pub render_graph: RenderGraph,
}

impl RenderWorld {
    pub fn new(render_server: &RenderContext) -> Self {
        let mut texture_cache = TextureCache::new();
        let camera_render_resources = CameraRenderResources::new(render_server);
        let sprite_render_resources = SpriteRenderResources::new(render_server);
        let mesh_render_resources = MeshRenderResources::new(render_server);
        let gizmo_render_resources = GizmoRenderResources::new(render_server, &camera_render_resources.bind_group_layout);
        let sky_render_resources = SkyRenderResources::new(render_server);
        let light_render_resources = LightRenderResources::new();
        let ssao_render_resources = SsaoRenderResources::new(render_server, &mut texture_cache, &camera_render_resources);

        // 我们不再单独管理深度纹理，因为它现在由 SSAO 资源池管理并自动创建。
        // 原有的 create_main_depth_texture 调用可以移除，因为它在 SsaoRenderResources::new 里已经被覆盖。

        Self {
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
        graph.add_node("transparent_mesh", TransparentMeshNode::default());
        graph.add_node("fxaa", FxaaNode::default());
        graph.add_node("present", PresentNode::default());
        graph.add_node("sprite", SpriteNode::default());

        graph.add_node_edge("cull", "shadow");
        graph.add_node_edge("cull", "mesh");
        graph.add_node_edge("shadow", "mesh");
        graph.add_node_edge("ssao", "mesh");
        graph.add_node_edge("clear", "skybox");
        graph.add_node_edge("skybox", "mesh");
        graph.add_node_edge("mesh", "transparent_mesh");
        graph.add_node_edge("transparent_mesh", "fxaa");
        graph.add_node_edge("fxaa", "present");
        graph.add_node_edge("transparent_mesh", "present");
        graph.add_node_edge("present", "sprite");
        graph
    }

    pub fn extract(&mut self, draw_commands: &DrawCommands) {
        self.extracted = draw_commands.extracted.clone();
    }

    pub fn prepare(&mut self, render_server: &RenderContext) {
        // 1. Prepare global camera data
        self.camera_render_resources.prepare_cameras(render_server, &self.extracted.cameras);

        // 2. Configure Render Graph based on settings (Optional: only if changed)
        let final_3d_resource = if self.extracted.fxaa_enabled {
            standard_resources::fxaa_color()
        } else {
            standard_resources::main_color()
        };

        if let Some(present) = self.render_graph.get_node_mut::<PresentNode>("present") {
            present.input_resource_id = final_3d_resource;
        }

        // 3. Prepare Bindless Materials (Now includes all 2D textures)
        self.mesh_render_resources.prepare_materials(&self.texture_cache, render_server, &self.extracted.sprites_2d);

        // 3. Separate opaque and transparent meshes
        let mut opaque_meshes = Vec::new();
        let mut transparent_meshes = Vec::new();
        for mesh in &self.extracted.meshes {
            let is_transparent = if let Some(material_id) = mesh.material_id {
                if let Some(material) = self.mesh_render_resources.material_cache.get(&material_id) {
                    material.transparent || mesh.transparent
                } else {
                    mesh.transparent
                }
            } else {
                mesh.transparent
            };

            if is_transparent {
                transparent_meshes.push(*mesh);
            } else {
                opaque_meshes.push(*mesh);
            }
        }
        self.extracted.transparent_meshes = transparent_meshes;
        let original_meshes = std::mem::replace(&mut self.extracted.meshes, opaque_meshes);

        // 4. Prepare 3D Mesh BVH (only for opaque meshes)
        if !self.extracted.meshes.is_empty() {
            let bvh_objects: Vec<_> = self.extracted.meshes.iter().enumerate().filter_map(|(i, ext)| {
                self.mesh_cache.get(ext.mesh_id).map(|mesh| (mesh.aabb.transform(&ext.transform), i))
            }).collect();
            self.extracted.bvh = Bvh::build(bvh_objects);
        }

        // 5. Prepare Unified 2D UI
        self.sprite_batches = prepare_sprite(&self.extracted.sprites_2d, &mut self.sprite_render_resources, &self.texture_cache, render_server, &self.mesh_render_resources, &self.extracted.cameras);

        // 6. Prepare 3D Meshes & Lights (combine opaque and transparent for instance preparation)
        let all_meshes: Vec<_> = self.extracted.meshes.iter().chain(self.extracted.transparent_meshes.iter()).cloned().collect();
        if !all_meshes.is_empty() || !self.extracted.lights.point_lights.is_empty() {
             prepare_meshes(&all_meshes, &self.extracted.lights, &self.texture_cache, &mut self.shader_maker, &mut self.mesh_render_resources, &self.light_render_resources, &self.camera_render_resources, render_server, &self.mesh_cache, self.ssao_render_resources.blur_texture, self.extracted.sky.as_ref().map(|s| s.texture));
        }

        // 6.5 Re-include MASKED transparent meshes for SSAO (normal pre-pass)
        // This allows leaves/grass to produce SSAO occlusion.
        let mut ssao_meshes = self.extracted.meshes.clone();
        for mesh in &self.extracted.transparent_meshes {
            if let Some(mat_id) = mesh.material_id {
                if let Some(mat) = self.mesh_render_resources.material_cache.get(&mat_id) {
                    if mat.alpha_mode == crate::render::material::AlphaMode::Mask {
                        ssao_meshes.push(*mesh);
                    }
                }
            }
        }
        self.extracted.meshes = ssao_meshes;

        // 6. Prepare Shadow & Sky
        let first_d3_cam = self.extracted.cameras.types.iter().position(|t| *t == CameraType::D3);
        if let Some(idx) = first_d3_cam {
            prepare_shadow(&self.extracted.lights, Some(&self.extracted.cameras.uniforms[idx]), render_server, &mut self.texture_cache, &mut self.light_render_resources, &self.camera_render_resources);
        }

        if let Some(sky) = &self.extracted.sky {
            prepare_sky(&mut self.sky_render_resources, render_server, &self.texture_cache, &sky.texture, &self.camera_render_resources.bind_group_layout, &mut self.mesh_render_resources.mesh_allocator);
        }
    }

    pub fn recreate_depth_texture(&mut self, _render_server: &RenderContext) {
        // 深度纹理现在由 RenderGraph::ResourcePool 按需管理，
        // 节点的 run 方法会通过 context.get_texture 自动处理 Resize。
    }

    fn create_main_color_texture(device: &wgpu::Device, config: &wgpu::SurfaceConfiguration, cache: &mut TextureCache) -> TextureId {
        let size = wgpu::Extent3d {
            width: config.width,
            height: config.height,
            depth_or_array_layers: 1,
        };
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("main color texture"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: config.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        cache.add(Texture {
            size: (config.width, config.height),
            texture,
            view,
            sampler,
            format: config.format,
        })
    }
}
