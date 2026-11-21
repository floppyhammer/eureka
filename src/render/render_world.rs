use crate::render::camera::ExtractedCameras;
use crate::render::light::ExtractedLights;
use crate::render::material::MaterialCache;
use crate::render::mesh_allocator::MeshAllocator;
pub(crate) use crate::render::render_backend::{RenderBackend, RenderCommand};
use crate::render::sky::ExtractedSky;
use crate::render::sprite::ExtractedSprite2d;
use crate::render::{ExtractedMesh, MeshCache, RenderContext, TextureCache};
use std::sync::{Arc, RwLock};

/// 渲染快照（Render Snapshot / Frame Data）。
/// 包含了当前帧渲染所需的所有场景数据和状态（如 2D 精灵、网格、相机、光源、天空盒以及后期处理开关等）。
/// 在每帧开始时从主世界（Main World）提取，并通过 `RenderCommand::Render`
/// 发送给独立的渲染后端线程（RenderBackend），实现逻辑与渲染的解耦。
#[derive(Clone, Default)]
pub struct Extracted {
    pub(crate) sprites: Vec<ExtractedSprite2d>,
    pub(crate) meshes: Vec<ExtractedMesh>,

    pub(crate) cameras: ExtractedCameras,
    pub(crate) lights: ExtractedLights,
    pub(crate) sky: Option<ExtractedSky>,
    pub render_config: crate::render::render_graph::RenderGraphConfig,
}

/// 渲染世界（Render Frontend）。
/// 负责管理渲染前端与独立渲染后端线程（RenderBackend）之间的通信通道，
/// 以及跨帧共享的资源缓存（纹理、网格、材质）和显存分配器。
pub struct RenderWorld {
    /// 向渲染后端线程发送命令（如渲染快照 RenderCommand::Render、窗口大小调整 RenderCommand::Resize）的通道发送端
    pub sender: std::sync::mpsc::SyncSender<RenderCommand>,
    /// 共享句柄：纹理缓存
    pub imported_texture_cache: Arc<RwLock<TextureCache>>,
    /// 共享句柄：网格缓存
    pub imported_mesh_cache: Arc<RwLock<MeshCache>>,
    /// 共享句柄：材质缓存
    pub imported_material_cache: Arc<RwLock<MaterialCache>>,
    /// 共享句柄：网格分配器
    pub imported_mesh_allocator: Arc<RwLock<MeshAllocator>>,
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

        // 初始化渲染后端（RenderBackend），负责管理 wgpu 设备交互、渲染管线、渲染图（RenderGraph）及表面配置
        let mut backend = RenderBackend::new(
            &render_context,
            surface,
            imported_texture_cache.clone(),
            imported_mesh_cache.clone(),
            imported_material_cache.clone(),
            imported_mesh_allocator.clone(),
        );

        // 启动独立的渲染线程。
        // 该线程循环接收来自主线程的 RenderCommand 命令：
        // 1. RenderCommand::Render(extracted)：接收渲染快照并调用 backend.run 执行一帧的图形渲染。
        // 2. RenderCommand::Resize(w, h)：处理窗口大小改变，重新配置 surface 并清理旧分辨率的显存缓存，防止显存泄漏。
        std::thread::spawn(move || {
            let mut render_context = render_context;
            while let Ok(cmd) = rx.recv() {
                match cmd {
                    RenderCommand::Render(extracted, tris, textures_delta, scale_factor) => {
                        backend.run(&render_context, extracted, tris, textures_delta, scale_factor);
                    }
                    RenderCommand::Resize(w, h) => {
                        render_context.surface_config.width = w;
                        render_context.surface_config.height = h;
                        backend.surface.configure(&render_context.device, &render_context.surface_config);

                        // 关键修复：缩放后不仅清理 BindGroup，也要清空旧分辨率的瞬时资源池，防止显存泄露
                        backend.render_graph.pool.clear_bind_group_cache();
                        backend.render_graph.pool.clear_transient_pools();
                    }
                    RenderCommand::UpdateConfig(config) => {
                        backend.set_render_config(config);
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
        }
    }

    /// 向渲染后端发送最新的 RenderGraphConfig。如果配置改变，渲染后端会自动重构 RenderGraph。
    pub fn update_render_config(&self, config: crate::render::render_graph::RenderGraphConfig) {
        let _ = self.sender.send(RenderCommand::UpdateConfig(config));
    }
}
