use crate::render::camera::ExtractedCameras;
use crate::render::light::ExtractedLights;
use crate::render::material::MaterialCache;
use crate::render::mesh_allocator::MeshAllocator;
pub(crate) use crate::render::render_backend::{RenderBackend, RenderCommand};
use crate::render::shader_maker::ShaderMaker;
use crate::render::sky::ExtractedSky;
use crate::render::sprite::ExtractedSprite2d;
use crate::render::{ExtractedMesh, MeshCache, RenderContext, TextureCache};
use std::sync::{Arc, RwLock};

#[derive(Clone, Default)]
pub struct Extracted {
    pub(crate) sprites: Vec<ExtractedSprite2d>,
    pub(crate) meshes: Vec<ExtractedMesh>,

    pub(crate) cameras: ExtractedCameras,
    pub(crate) lights: ExtractedLights,
    pub(crate) sky: Option<ExtractedSky>,
    pub fxaa_enabled: bool,
    pub ssao_enabled: bool,
    pub taa_enabled: bool,
}

/// Render frontend.
pub struct RenderWorld {
    pub sender: std::sync::mpsc::SyncSender<RenderCommand>,
    /// Shared handles: material textures, sprite textures.
    pub imported_texture_cache: Arc<RwLock<TextureCache>>,
    pub imported_mesh_cache: Arc<RwLock<MeshCache>>,
    pub imported_material_cache: Arc<RwLock<MaterialCache>>,
    pub imported_mesh_allocator: Arc<RwLock<MeshAllocator>>,
    pub shader_maker: ShaderMaker,
}

impl RenderWorld {
    pub fn new(render_context: RenderContext, surface: wgpu::Surface<'static>) -> Self {
        let imported_texture_cache = Arc::new(RwLock::new(TextureCache::new()));
        let imported_mesh_cache = Arc::new(RwLock::new(MeshCache::new()));
        let imported_material_cache = Arc::new(RwLock::new(MaterialCache::new()));
        let imported_mesh_allocator =
            Arc::new(RwLock::new(MeshAllocator::new(&render_context.device)));

        // Use a capacity based on frames in flight to prevent deadlocks in get_current_texture.
        let channel_cap = (render_context.frames_in_flight.saturating_sub(1)) as usize;
        let (tx, rx) = std::sync::mpsc::sync_channel::<RenderCommand>(channel_cap.max(1));

        let mut backend = RenderBackend::new(
            &render_context,
            surface,
            imported_texture_cache.clone(),
            imported_mesh_cache.clone(),
            imported_material_cache.clone(),
            imported_mesh_allocator.clone(),
        );

        std::thread::spawn(move || {
            while let Ok(cmd) = rx.recv() {
                match cmd {
                    RenderCommand::Render(extracted) => {
                        backend.run(&render_context, extracted);
                    }
                    RenderCommand::Resize(w, h) => {
                        let mut config = render_context.surface_config.clone();
                        config.width = w;
                        config.height = h;
                        backend.surface.configure(&render_context.device, &config);
                        backend.render_graph.pool.clear_bind_group_cache();
                    }
                }
            }
        });

        Self {
            sender: tx,
            imported_texture_cache,
            imported_mesh_cache,
            imported_material_cache,
            imported_mesh_allocator,
            shader_maker: ShaderMaker::new(),
        }
    }
}
