use std::convert::TryFrom;
use std::mem;
use std::num::NonZeroU32;
use std::sync::Arc;

use winit::{
    dpi::{LogicalPosition, PhysicalPosition, Position, Size},
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

use cgmath::{prelude::*, Point2, Vector2, Vector3, Vector4};
use indextree::NodeId;

use wgpu::{util::DeviceExt, SamplerBindingType, TextureView};
use winit::platform::run_return::EventLoopExtRunReturn;

// Do this before importing local crates.
pub mod math;
pub mod render;
pub mod resources;
pub mod scene;
pub mod servers;

// Import local crates.
use crate::render::atlas::{Atlas, AtlasInstance};
use crate::render::gizmo::Gizmo;
use crate::resources::{CubemapTexture, DynamicFont, Texture};
use crate::scene::sprite2d::Sprite2d;
use crate::scene::sprite3d::Sprite3d;
use crate::scene::vector_sprite::{DrawVector, VectorSprite};
use crate::scene::{
    AsNode, Camera2d, Camera3d, Camera3dController, InputEvent, InputServer, Label, Light,
    LightUniform, Model, Projection, Sky, World,
};
use crate::servers::render_server::RenderServer;
use crate::servers::text_server::TextServer;
use crate::servers::{AssetServer, core_server, CoreServer};

const INITIAL_WINDOW_WIDTH: u32 = 1280;
const INITIAL_WINDOW_HEIGHT: u32 = 720;

pub struct Singletons {
    pub core_server: CoreServer,
    pub render_server: RenderServer,
    pub input_server: InputServer,
    pub text_server: TextServer,
    pub asset_server: AssetServer,
}

// For convenience we're going to pack all the fields into a struct,
// and create some methods on that.
pub struct App {
    window: Window,
    window_size: winit::dpi::PhysicalSize<u32>,
    depth_texture: Texture,
    previous_frame_time: f32,
    world: World,
    pub singletons: Singletons,
    is_init: bool,
}

impl App {
    pub fn new(event_loop: &EventLoop<()>) -> Self {
        let env = env_logger::Env::default()
            .filter_or("EUREKA_LOG_LEVEL", "info")
            .write_style_or("EUREKA_LOG_STYLE", "always");
        env_logger::init_from_env(env);

        // Use cargo package name as the window title.
        let title = env!("CARGO_PKG_NAME");

        let window_size = winit::dpi::PhysicalSize::new(
            INITIAL_WINDOW_WIDTH,
            INITIAL_WINDOW_HEIGHT,
        );

        let window = WindowBuilder::new()
            .with_title(title)
            .with_inner_size(window_size)
            .build(&event_loop)
            .unwrap();

        // App::new uses async code, so we're going to wait for it to finish
        let mut render_server = pollster::block_on(App::init_render(&window));

        let mut core_server = CoreServer::new();

        // Depth texture for depth test.
        let depth_texture = Texture::create_depth_texture(
            &render_server.device,
            &render_server.config,
            "depth texture",
        );

        let asset_server = AssetServer::new();

        let mut text_server = TextServer::new(
            asset_server.asset_dir.join("fonts/Arial Unicode MS Font.ttf"),
            &render_server,
        );
        let mut world = World::new(Point2::new(window_size.width, window_size.height));
        // // Create nodes.
        // // ---------------------------------------------------
        //
        //
        // let camera3d = Camera3d::new(
        //     (0.0, 0.0, 0.0),
        //     cgmath::Deg(-90.0),
        //     cgmath::Deg(0.0),
        //     &render_server,
        // );
        // world.add_node(Box::new(camera3d), None);
        //
        // let camera2d = Camera2d::new((size.width, size.height));
        // world.add_node(Box::new(camera2d), None);
        //
        // let skybox_tex =
        //     CubemapTexture::load(&render_server, asset_dir.join("skybox.jpg")).unwrap();
        // let sky = Box::new(Sky::new(&render_server, skybox_tex));
        // world.add_node(sky, None);
        //
        // // Light.
        // let light = Light::new(&render_server, asset_dir.join("light.png"));
        // world.add_node(Box::new(light), None);
        //
        // // Model.
        // // let obj_model = Box::new(
        // //     Model::load(&render_server, asset_dir.join("ferris/ferris3d_v1.0.obj")).unwrap(),
        // // );
        // // world.add_node(obj_model, None);
        //

        //
        // let mut label = Box::new(Label::new(&render_server));
        // label.transform.position = Point2::new(0.0, 200.0);
        // label.set_text("This is a label!".to_string());
        // let fps_label_id = world.add_node(label, Some(vec_sprite_id));

        // ---------------------------------------------------

        // Test ground.
        // ---------------------------------------------------
        // let mut atlas = Atlas::new(&render_server);
        //
        // let mut instances = vec![];
        // for i in 0..10 {
        //     let instance = AtlasInstance {
        //         position: Vector2::new(i as f32 * 100.0 + 100.0, i as f32 * 100.0),
        //         size: Vector2::new(128.0, 128.0),
        //         region: Vector4::new(0.0, 0.0, 1.0, 1.0),
        //         color: Vector4::new(1.0, 1.0, 1.0, 1.0),
        //     };
        //     instances.push(instance);
        // }
        // atlas.set_instances(instances, &render_server);
        // atlas.set_texture(Texture::load(
        //     &render_server.device,
        //     &render_server.queue,
        //     asset_dir.join("happy-tree.png"),
        // ).unwrap(), &render_server);
        // ---------------------------------------------------

        let singletons = Singletons {
            core_server,
            render_server,
            input_server: InputServer::new(),
            text_server,
            asset_server,
        };

        Self {
            window,
            window_size,
            depth_texture,
            previous_frame_time: 0.0,
            world,
            singletons,
            is_init: false,
        }
    }

    // Creating some of the wgpu types requires async code.
    async fn init_render(window: &Window) -> RenderServer {
        // The instance is a handle to our GPU.
        let instance = wgpu::Instance::new(wgpu::Backends::all());

        // The surface is the part of the window that we draw to.
        let surface = unsafe { instance.create_surface(window) };

        // The adapter is a handle to our actual graphics card.
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        // Use the adapter to create the device and queue.
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    features: wgpu::Features::empty(),
                    limits: wgpu::Limits::default(),
                    label: None,
                },
                None,
            )
            .await
            .unwrap();

        // Get the window's inner size.
        let size = window.inner_size();

        // This will define how the surface creates its underlying SurfaceTextures.
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface.get_supported_formats(&adapter)[0],
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
        };
        surface.configure(&device, &config);

        // Create a render server.
        RenderServer::new(surface, config, device, queue)
    }

    pub fn run(&mut self, event_loop: &mut EventLoop<()>) {
        // Main loop.
        event_loop.run_return(move |event, _, control_flow| {
            match event {
                // Device event.
                Event::DeviceEvent {
                    ref event,
                    .. // We're not using device_id currently.
                } => {
                    // We're not handling raw input data currently.
                }
                // Window event.
                Event::WindowEvent {
                    ref event,
                    window_id,
                } if window_id == self.window.id() => {
                    match event {
                        WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                        WindowEvent::Resized(physical_size) => {
                            // See https://github.com/rust-windowing/winit/issues/2094.
                            if self.is_init {
                                return;
                            }

                            self.resize(*physical_size);

                            log::info!("Window resized to {:?}", physical_size);
                        }
                        // Scale factor changed.
                        WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                            self.resize(**new_inner_size);

                            log::info!("Scale factor changed, new window size is {:?}", new_inner_size);
                        }
                        _ => {
                            // Other input events should be handled by the input server.
                            self.input(event);
                        }
                    }
                }
                // Redraw request.
                Event::RedrawRequested(_) => {
                    self.singletons.input_server.update(&self.window);

                    self.update();

                    match self.render() {
                        Ok(_) => {}
                        // Reconfigure the surface if lost.
                        Err(wgpu::SurfaceError::Lost) => self.resize(self.window_size),
                        // The system is out of memory, we should probably quit.
                        Err(wgpu::SurfaceError::OutOfMemory) => *control_flow = ControlFlow::Exit,
                        // All other errors (Outdated, Timeout) should be resolved by the next frame.
                        Err(e) => eprintln!("App resource error: {:?}", e),
                    }
                }
                Event::MainEventsCleared => {
                    // RedrawRequested will only trigger once, unless we manually request it.
                    self.window.request_redraw();
                }
                Event::NewEvents(cause) => {
                    if cause == StartCause::Init {
                        self.is_init = true;
                    } else {
                        self.is_init = false;
                    }
                }
                _ => {}
            }
        });
    }

    pub fn add_node(&mut self, mut new_node: Box<dyn AsNode>, parent: Option<NodeId>) {
        self.world.add_node(new_node, parent);
    }

    fn capture_cursor() {}

    fn release_cursor() {}

    /// Resize window.
    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        // Reconfigure the surface everytime the window's size changes.
        if new_size.width > 0 && new_size.height > 0 {
            self.window_size = new_size;
            self.singletons.render_server.config.width = new_size.width;
            self.singletons.render_server.config.height = new_size.height;
            self.singletons.render_server.surface.configure(
                &self.singletons.render_server.device,
                &self.singletons.render_server.config,
            );

            // Create a new depth_texture and depth_texture_view.
            // Make sure you update the depth_texture after you update config.
            // If you don't, your program will crash as the depth_texture will be a different size than the surface texture.
            self.depth_texture = Texture::create_depth_texture(
                &self.singletons.render_server.device,
                &self.singletons.render_server.config,
                "depth texture",
            );

            self.world
                .when_view_size_changes(Point2::new(new_size.width, new_size.height))
        }
    }

    /// Handle input events.
    fn input(&mut self, event: &WindowEvent) -> bool {
        // Convert to our own input events.
        self.singletons
            .input_server
            .prepare_input_event(&self.window, event);

        self.world.input(&mut self.singletons.input_server);

        true
    }

    fn update(&mut self) {
        self.singletons.core_server.tick();

        // self.world
        //     .get_node_mut::<Label>(self.fps_label_id)
        //     .unwrap()
        //     .set_text(format!(
        //         "FPS: {}",
        //         self.singletons.core_server.get_fps() as i32
        //     ));

        self.world.update(self.singletons.core_server.get_delta() as f32, &mut self.singletons);
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        // First we need to get a frame to draw to.
        let output_surface = self
            .singletons
            .render_server
            .surface
            .get_current_texture()?;

        // Creates a TextureView with default settings.
        let view = output_surface
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        // Builds a command buffer that we can then send to the GPU.
        let mut encoder = self.singletons.render_server.device.create_command_encoder(
            &wgpu::CommandEncoderDescriptor {
                label: Some("main render encoder"),
            },
        );

        // The RenderPass has all the methods to do the actual drawing.
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("main render pass"),
                color_attachments: &[
                    // This is what @location(0) in the fragment shader targets.
                    Some(wgpu::RenderPassColorAttachment {
                        view: &view, // Change this to change where to draw.
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                r: 0.1,
                                g: 0.2,
                                b: 0.3,
                                a: 1.0,
                            }),
                            store: true,
                        },
                    }),
                ],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_texture.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: true,
                    }),
                    stencil_ops: None,
                }),
            });

            // Update server GPU resources.
            self.singletons
                .text_server
                .update_gpu(&self.singletons.render_server);

            self.world.draw(&mut render_pass, &self.singletons);
        }

        // Finish the command encoder to generate a command buffer,
        // then submit it for execution.
        self.singletons
            .render_server
            .queue
            .submit(std::iter::once(encoder.finish()));

        // Present the swapchain surface.
        output_surface.present();

        Ok(())
    }
}
