use std::sync::Arc;

use winit::{
    application::ApplicationHandler,
    event::*,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowAttributes, WindowId},
};

use glam::UVec2;
use indextree::NodeId;

use crate::core::engine::Engine;
use winit::dpi::{LogicalSize, PhysicalSize, Size};

// Import local crates.
use crate::asset::AssetServer;
use crate::core::singleton::Singletons;
use crate::render::render_world::RenderWorld;
use crate::render::RenderServer;
use crate::scene::{AsNode, Model, Sky, Sprite2d, World};
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
        let env = env_logger::Env::default()
            .filter_or("EUREKA_LOG_LEVEL", "info")
            .write_style_or("EUREKA_LOG_STYLE", "always");
        let _ = env_logger::try_init_from_env(env);

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

    // Creating some of the wgpu types requires async code.
    async fn init_render(window: Arc<Window>) -> RenderServer<'a> {
        // Context for all other wgpu objects.
        let instance = wgpu::Instance::default();

        // Handle to a presentable surface.
        let surface = instance.create_surface(window.clone()).unwrap();

        // Handle to a physical graphics and/or compute device.
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .expect("Failed to find an appropriate adapter!");

        // Use the adapter to create a device and a queue.
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                required_limits: Default::default(),
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
        let present_mode = if surface_capabilities
            .present_modes
            .contains(&wgpu::PresentMode::Mailbox)
        {
            wgpu::PresentMode::Mailbox
        } else {
            wgpu::PresentMode::Fifo // 保底使用 Fifo
        };
        surface_config.present_mode = present_mode;

        surface.configure(&device, &surface_config);

        // Create a render server.
        RenderServer::new(surface, surface_config, device, queue)
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
                let config = &mut singletons.render_server.surface_config;
                config.width = new_size.width;
                config.height = new_size.height;

                singletons
                    .render_server
                    .surface
                    .configure(&singletons.render_server.device, config);

                if let Some(render_world) = &mut self.render_world {
                    render_world.recreate_depth_texture(&singletons.render_server);
                }

                self.world
                    .when_view_size_changes(UVec2::new(new_size.width, new_size.height))
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
                &singletons.render_server,
                &mut render_world.texture_cache,
                &singletons.asset_server,
            );

            // Reconcile pending models.
            let ids = self.world.traverse();
            for id in ids {
                // 1. Reconcile Models
                let mut model_raw_path = None;
                if let Some(model) = self.world.get_node::<Model>(id) {
                    if let Some(path) = &model.asset_path {
                        singletons.asset_server.request_load(path);
                        if singletons.asset_server.loaded_raw_models.contains_key(path) {
                            model_raw_path = Some(path.clone());
                        }
                    }
                }
                if let Some(path) = model_raw_path {
                    if let Some(model) = self.world.get_node_mut::<Model>(id) {
                        let raw = singletons
                            .asset_server
                            .loaded_raw_models
                            .get(&path)
                            .unwrap()
                            .clone();
                        model.finalize(
                            raw,
                            &singletons.render_server,
                            &mut render_world.texture_cache,
                            &mut render_world.mesh_render_resources.material_cache,
                            &mut render_world.mesh_cache,
                        );
                    }
                }

                // 2. Reconcile Sky
                let mut sky_raw_path = None;
                if let Some(sky) = self.world.get_node::<Sky>(id) {
                    if let Some(path) = &sky.asset_path {
                        singletons.asset_server.request_cubemap(path);
                        if singletons
                            .asset_server
                            .loaded_raw_cubemaps
                            .contains_key(path)
                        {
                            sky_raw_path = Some(path.clone());
                        }
                    }
                }
                if let Some(path) = sky_raw_path {
                    if let Some(sky) = self.world.get_node_mut::<Sky>(id) {
                        let raw = singletons
                            .asset_server
                            .loaded_raw_cubemaps
                            .get(&path)
                            .unwrap()
                            .clone();
                        sky.finalize(
                            raw,
                            &singletons.render_server,
                            &mut render_world.texture_cache,
                        );
                    }
                }

                // 3. Reconcile Sprite2d
                let mut sprite_raw_path = None;
                if let Some(sprite) = self.world.get_node::<Sprite2d>(id) {
                    if let Some(path) = &sprite.asset_path {
                        singletons.asset_server.request_texture(path);
                        if singletons
                            .asset_server
                            .loaded_raw_textures
                            .contains_key(path)
                        {
                            sprite_raw_path = Some(path.clone());
                        }
                    }
                }
                if let Some(path) = sprite_raw_path {
                    if let Some(sprite) = self.world.get_node_mut::<Sprite2d>(id) {
                        let raw = singletons
                            .asset_server
                            .loaded_raw_textures
                            .get(&path)
                            .unwrap()
                            .clone();
                        sprite.finalize(
                            raw,
                            &singletons.render_server,
                            &mut render_world.texture_cache,
                        );
                    }
                }
            }

            singletons.engine.tick();
            self.world
                .update(singletons.engine.get_delta() as f32, singletons);
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

        let render_server = &singletons.render_server;

        render_world.prepare(render_server);

        // Update server GPU resources.
        singletons
            .text_server
            .prepare(&singletons.render_server, &mut render_world.texture_cache);

        // First we need to get a frame to draw to.
        let surface_texture = render_server.surface.get_current_texture()?;

        // Creates a TextureView with default settings.
        let view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let depth_texture = render_world
            .texture_cache
            .get(render_world.surface_depth_texture)
            .unwrap();

        // Builds a command buffer that we can then send to the GPU.
        let mut encoder =
            render_server
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("main render encoder"),
                });

        render_world.render_shadow(&mut encoder);

        let ssao_ran = render_world.render_ssao(&mut encoder);

        // The RenderPass has all the methods to do the actual drawing.
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("main render pass"),
                color_attachments: &[
                    // This is what @location(0) in the fragment shader targets.
                    Some(wgpu::RenderPassColorAttachment {
                        view: &view, // Change this to change where to draw.
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                r: 0.1,
                                g: 0.2,
                                b: 0.3,
                                a: 1.0,
                            }),
                            store: wgpu::StoreOp::Store,
                        },
                    }),
                ],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &depth_texture.view,
                    depth_ops: Some(wgpu::Operations {
                        load: if ssao_ran {
                            wgpu::LoadOp::Load
                        } else {
                            wgpu::LoadOp::Clear(1.0)
                        }, // Z-Prepass
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            render_world.render(&mut render_pass);
        }

        // Finish the command encoder to generate a command buffer,
        // then submit it for execution.
        singletons
            .render_server
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
        let render_server = pollster::block_on(Self::init_render(window.clone()));

        let engine = Engine::new();
        let mut asset_server = AssetServer::new();
        let render_world = RenderWorld::new(&render_server);
        let text_server = TextServer::new(&mut asset_server);

        self.singletons = Some(Singletons {
            engine,
            render_server,
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
