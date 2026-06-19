use std::sync::Arc;

use crate::core::time::Time;
use winit::dpi::{LogicalSize, PhysicalSize, Size};
use winit::{
    application::ApplicationHandler,
    event::*,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowAttributes, WindowId},
};

// Import local crates.
use crate::asset::AssetManager;
use crate::core::singleton::Singletons;
use crate::render::render_world::{RenderCommand, RenderWorld};
use crate::render::RenderContext;
use crate::scene::World;
use crate::text::TextServer;
use crate::window::InputServer;

const INITIAL_WINDOW_WIDTH: u32 = 1280;
const INITIAL_WINDOW_HEIGHT: u32 = 720;

pub struct App {
    window: Option<Arc<Window>>,
    window_size: LogicalSize<u32>,
    scale_factor: f64,
    pub world: World,
    pub render_world: Option<RenderWorld>,
    pub singletons: Option<Singletons>,
    initialized: bool,
    /// We keep the event loop in an option to take it when running.
    event_loop: Option<EventLoop<()>>,
    /// Callback for user setup logic after initialization.
    setup_callback: Option<Box<dyn FnOnce(&mut App)>>,
    /// Callback for user-defined update logic, called every frame after the world update.
    update_callbacks: Vec<Box<dyn FnMut(&mut App, f32)>>,
}

impl App {
    pub fn new() -> Self {
        // Config logger
        let env = env_logger::Env::default()
            .filter_or("EUREKA_LOG_LEVEL", "info")
            .write_style_or("EUREKA_LOG_STYLE", "always");
        let _ = env_logger::try_init_from_env(env);

        // New winit event loop
        let event_loop = EventLoop::new().unwrap();

        let window_size = LogicalSize::new(INITIAL_WINDOW_WIDTH, INITIAL_WINDOW_HEIGHT);

        let world = World::new();

        Self {
            window: None,
            window_size,
            scale_factor: 1.0,
            world,
            render_world: None,
            singletons: None,
            initialized: false,
            event_loop: Some(event_loop),
            setup_callback: None,
            update_callbacks: Vec::new(),
        }
    }

    pub fn setup<F>(&mut self, f: F)
    where
        F: FnOnce(&mut App) + 'static,
    {
        self.setup_callback = Some(Box::new(f));
    }

    /// Register a callback that will be invoked every frame after the world update.
    /// The callback receives a mutable reference to the app and the delta time in seconds.
    pub fn add_update<F>(&mut self, f: F)
    where
        F: FnMut(&mut App, f32) + 'static,
    {
        self.update_callbacks.push(Box::new(f));
    }

    /// Creating some of the wgpu types requires async code.
    async fn init_render(
        window: Arc<Window>,
        render_cpu_time: Arc<std::sync::atomic::AtomicU64>,
        gpu_time: Arc<std::sync::atomic::AtomicU64>,
    ) -> (RenderContext, wgpu::Surface<'static>) {
        // Context for all other wgpu objects.
        let instance = wgpu::Instance::default();

        // Handle to a presentable surface.
        let surface = instance.create_surface(window.clone()).unwrap();

        // Handle to a physical graphics and/or compute device.
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance, // 强制请求高性能显卡
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .expect("Failed to find an appropriate adapter!");

        let mut features = wgpu::Features::TEXTURE_BINDING_ARRAY
            | wgpu::Features::SAMPLED_TEXTURE_AND_STORAGE_BUFFER_ARRAY_NON_UNIFORM_INDEXING
            | wgpu::Features::INDIRECT_FIRST_INSTANCE
            | wgpu::Features::MULTI_DRAW_INDIRECT;

        if adapter.features().contains(wgpu::Features::TIMESTAMP_QUERY) {
            features |= wgpu::Features::TIMESTAMP_QUERY;
        }

        if adapter
            .features()
            .contains(wgpu::Features::TIMESTAMP_QUERY_INSIDE_ENCODERS)
        {
            features |= wgpu::Features::TIMESTAMP_QUERY_INSIDE_ENCODERS;
        }

        // Use the adapter to create a device and a queue.
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: features,
                // 移除了 STORAGE_RESOURCE_BINDING_ARRAY，因为目前没用到
                required_limits: wgpu::Limits {
                    max_binding_array_elements_per_shader_stage: 1024,
                    ..Default::default()
                },
                memory_hints: Default::default(),
                trace: Default::default(),
            })
            .await
            .expect("Failed to create device!");

        // Get the window's inner size.
        let size = window.inner_size();

        let mut surface_config = surface
            .get_default_config(&adapter, size.width, size.height)
            .expect("Surface unsupported by adapter!");

        let surface_capabilities = surface.get_capabilities(&adapter);

        // 如果无法从 capabilities 获取，通常根据显示模式推断：
        // Fifo (V-Sync) 通常需要 2 张图，Mailbox (Triple Buffering) 通常需要 3 张。
        // 为了看到真实的 release 性能，我们优先尝试使用 Mailbox 或 Immediate (不锁帧)
        let present_mode = if surface_capabilities
            .present_modes
            .contains(&wgpu::PresentMode::Mailbox)
        {
            wgpu::PresentMode::Mailbox
        } else if surface_capabilities
            .present_modes
            .contains(&wgpu::PresentMode::Immediate)
        {
            wgpu::PresentMode::Immediate
        } else {
            wgpu::PresentMode::Fifo
        };

        let frames_in_flight = if present_mode == wgpu::PresentMode::Mailbox {
            3
        } else {
            2
        };

        surface_config.present_mode = present_mode;

        surface.configure(&device, &surface_config);

        // Create a render server.
        (
            RenderContext::new(
                surface_config,
                device,
                queue,
                frames_in_flight,
                render_cpu_time,
                gpu_time,
            ),
            surface,
        )
    }

    pub fn run(&mut self) {
        let event_loop = self.event_loop.take().expect("Event loop already taken");
        event_loop.set_control_flow(ControlFlow::Poll);
        event_loop.run_app(self).expect("Failed to run event loop");
    }

    /// Resize window.
    fn resize(&mut self, new_size: PhysicalSize<u32>) {
        self.window_size = new_size.to_logical(self.scale_factor);

        if let Some(singletons) = &mut self.singletons {
            if new_size.width > 0 && new_size.height > 0 {
                // 1. 更新 wgpu surface 配置 (逻辑层记录)
                singletons.render_context.surface_config.width = new_size.width;
                singletons.render_context.surface_config.height = new_size.height;

                // 3. 通知渲染线程执行真正的配置和资源清理
                if let Some(render_world) = &self.render_world {
                    let _ = render_world
                        .sender
                        .send(RenderCommand::Resize(new_size.width, new_size.height));
                }
            }
        }
    }

    /// Handle input events.
    fn input(&mut self, event: &WindowEvent) -> bool {
        if let (Some(window), Some(singletons)) = (&self.window, &mut self.singletons) {
            // Convert to our own input events.
            singletons.input_server.prepare_input_event(window, event);

            return true;
        }
        false
    }

    fn update(&mut self) {
        if let (Some(singletons), Some(render_world)) =
            (&mut self.singletons, &mut self.render_world)
        {
            // Reconcile fonts.
            singletons.text_server.update(
                &singletons.render_context,
                &mut render_world.imported_texture_cache.write().unwrap(),
                &mut singletons.asset_manager,
            );

            singletons.time.tick();
            self.world
                .update(singletons.time.get_delta() as f32, singletons, render_world);

            // Run user-defined update callbacks.
            let dt = singletons.time.get_delta() as f32;
            let callbacks = std::mem::take(&mut self.update_callbacks);
            for mut callback in callbacks {
                callback(self, dt);
                self.update_callbacks.push(callback);
            }
        }
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let (Some(singletons), Some(render_world)) = (&mut self.singletons, &mut self.render_world)
        else {
            return Ok(());
        };

        // Extract render entities from the draw commands.
        let extracted = self.world.extract_render_objects();

        // Update server GPU resources (text).
        // Note: text_server.prepare currently writes to texture cache, so we need a write lock.
        singletons.text_server.prepare(
            &singletons.render_context,
            &mut render_world.imported_texture_cache.write().unwrap(),
        );

        // Send to render thread.
        let _ = render_world.sender.send(RenderCommand::Render(extracted));

        Ok(())
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        // Use cargo package name as the window title.
        let title = env!("CARGO_PKG_NAME");

        let mut attributes = WindowAttributes::default();
        attributes.title = title.to_string();
        attributes.inner_size = Some(Size::from(self.window_size));

        let window = Arc::new(event_loop.create_window(attributes).unwrap());
        self.window = Some(window.clone());

        let time = Time::new();

        // App::init_render uses async code, so we're going to wait for it to finish.
        let (render_context, surface) = pollster::block_on(Self::init_render(
            window.clone(),
            time.render_cpu_time.clone(),
            time.gpu_time.clone(),
        ));

        let mut asset_manager = AssetManager::new();
        let render_world = RenderWorld::new(render_context.clone(), surface);
        let text_server = TextServer::new(&mut asset_manager);

        self.singletons = Some(Singletons {
            time,
            render_context,
            input_server: InputServer::new(),
            text_server,
            asset_manager,
        });

        self.render_world = Some(render_world);
        self.initialized = true;

        // Run user setup callback if provided.
        if let Some(setup) = self.setup_callback.take() {
            setup(self);
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        // Clone the Arc to release the borrow on self and satisfy the borrow checker.
        let window = match &self.window {
            Some(w) if w.id() == window_id => w.clone(),
            _ => return,
        };

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(physical_size) => {
                self.resize(physical_size);
                log::info!("Window resized to {:?}", physical_size);
            }
            // Scale factor changed.
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                self.scale_factor = scale_factor;

                let new_physical_size = self.window_size.to_physical(scale_factor);
                self.resize(new_physical_size);

                let _ = window.request_inner_size(new_physical_size);

                log::info!(
                    "Scale factor changed, change window size to {:?}",
                    new_physical_size
                );
            }
            // Redraw request.
            WindowEvent::RedrawRequested => {
                if self.singletons.is_some() {
                    let logic_start = std::time::Instant::now();

                    {
                        let singletons = self.singletons.as_mut().unwrap();
                        singletons.input_server.update(&window);
                        self.world.input(&mut singletons.input_server);
                    }

                    // 在清空事件之前执行更新，让用户回调能访问输入事件
                    self.update();

                    {
                        let singletons = self.singletons.as_mut().unwrap();
                        singletons.input_server.input_events.clear();
                    }

                    let render_result = self.render();

                    // Store logic time (including extraction).
                    if let Some(singletons) = &mut self.singletons {
                        singletons.time.logic_time.store(
                            logic_start.elapsed().as_nanos() as u64,
                            std::sync::atomic::Ordering::Relaxed,
                        );
                    }

                    match render_result {
                        Ok(_) => {
                            window.request_redraw();
                        }
                        // Reconfigure the surface if lost or outdated.
                        Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                            self.resize(self.window_size.to_physical(self.scale_factor))
                        }
                        // The system is out of memory, we should probably quit.
                        Err(wgpu::SurfaceError::OutOfMemory) => event_loop.exit(),
                        // All other errors (Timeout) should be resolved by the next frame.
                        Err(e) => eprintln!("App resource error: {:?}", e),
                    }
                }
            }
            _ => {
                // Other input events should be handled by the input server.
                self.input(&event);
            }
        }
    }

    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _device_id: DeviceId,
        event: DeviceEvent,
    ) {
        if let Some(singletons) = &mut self.singletons {
            singletons.input_server.handle_device_event(&event);
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}
