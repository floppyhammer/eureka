use std::sync::Arc;

use winit::{
    application::ApplicationHandler,
    event::*,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowAttributes, WindowId},
};

use glam::UVec2;
use indextree::NodeId;

use crate::core::time::Time;
use winit::dpi::{LogicalSize, PhysicalSize, Size};

// Import local crates.
use crate::asset::AssetServer;
use crate::core::singleton::Singletons;
use crate::render::render_world::RenderWorld;
use crate::render::RenderContext;
use crate::scene::{AsNode, World};
use crate::text::TextServer;
use crate::window::InputServer;

const INITIAL_WINDOW_WIDTH: u32 = 1280;
const INITIAL_WINDOW_HEIGHT: u32 = 720;

pub struct App<'a> {
    window: Option<Arc<Window>>,
    window_size: LogicalSize<u32>,
    scale_factor: f64,
    pub world: World,
    pub render_world: Option<RenderWorld>,
    pub singletons: Option<Singletons<'a>>,
    initialized: bool,
    /// We keep the event loop in an option to take it when running.
    event_loop: Option<EventLoop<()>>,
    /// Callback for user setup logic after initialization.
    setup_callback: Option<Box<dyn FnOnce(&mut App<'a>)>>,
}

impl<'a> App<'a> {
    pub fn new() -> Self {
        // Config logger
        let env = env_logger::Env::default()
            .filter_or("EUREKA_LOG_LEVEL", "info")
            .write_style_or("EUREKA_LOG_STYLE", "always");
        let _ = env_logger::try_init_from_env(env);

        // New winit event loop
        let event_loop = EventLoop::new().unwrap();

        let window_size = LogicalSize::new(INITIAL_WINDOW_WIDTH, INITIAL_WINDOW_HEIGHT);
        let world = World::new(UVec2::new(window_size.width, window_size.height));

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
        }
    }

    pub fn setup<F>(&mut self, f: F)
    where
        F: FnOnce(&mut App<'a>) + 'static,
    {
        self.setup_callback = Some(Box::new(f));
    }

    /// Creating some of the wgpu types requires async code.
    async fn init_render(window: Arc<Window>) -> RenderContext<'a> {
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

        // Use the adapter to create a device and a queue.
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::TEXTURE_BINDING_ARRAY
                    | wgpu::Features::SAMPLED_TEXTURE_AND_STORAGE_BUFFER_ARRAY_NON_UNIFORM_INDEXING
                    | wgpu::Features::INDIRECT_FIRST_INSTANCE
                    | wgpu::Features::MULTI_DRAW_INDIRECT,
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
        let is_mailbox = surface_capabilities.present_modes.contains(&wgpu::PresentMode::Mailbox);
        let frames_in_flight = if is_mailbox { 3 } else { 2 };

        surface_config.present_mode = if is_mailbox {
            wgpu::PresentMode::Mailbox
        } else {
            wgpu::PresentMode::Fifo
        };

        surface.configure(&device, &surface_config);

        // Create a render server.
        RenderContext::new(surface, surface_config, device, queue, frames_in_flight)
    }

    pub fn run(&mut self) {
        let event_loop = self.event_loop.take().expect("Event loop already taken");
        event_loop.set_control_flow(ControlFlow::Poll);
        event_loop.run_app(self).expect("Failed to run event loop");
    }

    pub fn add_node(&mut self, new_node: impl AsNode + 'static, parent: Option<NodeId>) {
        self.world.add_node(Box::new(new_node), parent);
    }

    /// Resize window.
    fn resize(&mut self, new_size: PhysicalSize<u32>) {
        self.window_size = new_size.to_logical(self.scale_factor);

        if let Some(singletons) = &mut self.singletons {
            // Reconfigure the surface everytime the window's size changes.
            if new_size.width > 0 && new_size.height > 0 {
                let config = &mut singletons.render_context.surface_config;
                config.width = new_size.width;
                config.height = new_size.height;

                singletons
                    .render_context
                    .surface
                    .configure(&singletons.render_context.device, config);

                if let Some(render_world) = &mut self.render_world {
                    render_world.recreate_depth_texture(&singletons.render_context);
                }

                self.world.when_view_size_changes(UVec2::new(new_size.width, new_size.height))
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
            // Update asset server (collects background loads)
            singletons.asset_server.update();

            // Reconcile fonts.
            singletons.text_server.update(
                &singletons.render_context,
                &mut render_world.texture_cache,
                &singletons.asset_server,
            );

            // Reconcile pending assets.
            let ids = self.world.traverse();
            for id in ids {
                self.world.arena[id].get_mut().reconcile(singletons, render_world);
            }

            singletons.time.tick();
            self.world.update(singletons.time.get_delta() as f32, singletons);
        }
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let (Some(singletons), Some(render_world)) = (&mut self.singletons, &mut self.render_world)
        else {
            return Ok(());
        };

        // Collects draw commands from the scene world.
        let draw_commands = self.world.queue_draw();

        // Extract render entities from the draw commands.
        render_world.extract(&draw_commands);

        let render_server = &singletons.render_context;

        render_world.prepare(render_server);

        // Update server GPU resources.
        singletons
            .text_server
            .prepare(&singletons.render_context, &mut render_world.texture_cache);

        // First we need to get a frame to draw to.
        let surface_texture = render_server.surface.get_current_texture()?;

        // Creates a TextureView with default settings.
        let view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        // Builds a command buffer that we can then send to the GPU.
        let mut encoder =
            render_server
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("main render encoder"),
                });

        render_world.run_graph(
            render_server,
            &mut encoder,
            &view,
        );

        // Finish the command encoder to generate a command buffer,
        // then submit it for execution.
        singletons
            .render_context
            .queue
            .submit(std::iter::once(encoder.finish()));

        // Present the swapchain surface.
        surface_texture.present();

        Ok(())
    }
}

impl<'a> ApplicationHandler for App<'a> {
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

        // App::init_render uses async code, so we're going to wait for it to finish.
        let render_context = pollster::block_on(Self::init_render(window.clone()));

        let time = Time::new();
        let mut asset_server = AssetServer::new();
        let render_world = RenderWorld::new(&render_context);
        let text_server = TextServer::new(&mut asset_server);

        self.singletons = Some(Singletons {
            time,
            render_context,
            input_server: InputServer::new(),
            text_server,
            asset_server,
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
                if let Some(singletons) = &mut self.singletons {
                    singletons.input_server.update(&window);

                    self.world.input(&mut singletons.input_server);
                    singletons.input_server.input_events.clear();

                    self.update();

                    match self.render() {
                        Ok(_) => {
                            window.request_redraw();
                        }
                        // Reconfigure the surface if lost.
                        Err(wgpu::SurfaceError::Lost) => {
                            self.resize(self.window_size.to_physical(self.scale_factor))
                        }
                        // The system is out of memory, we should probably quit.
                        Err(wgpu::SurfaceError::OutOfMemory) => event_loop.exit(),
                        // All other errors (Outdated, Timeout) should be resolved by the next frame.
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
