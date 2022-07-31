use std::convert::TryFrom;
use std::mem;
use std::num::NonZeroU32;
use std::sync::Arc;

use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
    window::Window,
    dpi::{LogicalPosition, PhysicalPosition, Position, Size},
};

use cgmath::prelude::*;
use cgmath::{Point2, Vector2, Vector3};

use wgpu::{SamplerBindingType, TextureView};
use wgpu::util::DeviceExt;

// Do this before importing local crates.
mod resource;
mod scene;
mod ecs;
mod server;

// Import local crates.
use crate::resource::{Vertex, Texture, CubemapTexture};
use crate::scene::{
    DrawModel, Model, Light, DrawLight, LightUniform, World,
    Camera3d, Camera2d, Projection, Camera3dController, InputEvent,
    Sky,
};
use crate::scene::sprite::Sprite;
use crate::scene::vector_sprite::{DrawVector, VectorSprite};
use crate::server::render_server::RenderServer;

const INITIAL_WINDOW_WIDTH: u32 = 1280;
const INITIAL_WINDOW_HEIGHT: u32 = 720;


// For convenience we're going to pack all the fields into a struct,
// and create some methods on that.
struct App {
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
    render_server: RenderServer,
    depth_texture: Texture,
    light_model: Model,
    mouse_position: (f32, f32),
    cursor_captured: bool,
    previous_frame_time: f32,
    world: World,
}

impl App {
    // Create some of the wgpu types requires async code.
    async fn new(window: &Window) -> Self {
        // The instance is a handle to our GPU.
        // Backends::all => Vulkan + Metal + DX12 + Browser WebGPU.
        let instance = wgpu::Instance::new(wgpu::Backends::all());

        // The surface is the part of the window that we draw to.
        let surface = unsafe { instance.create_surface(window) };

        // The adapter is a handle to our actual graphics card.
        let adapter = instance.request_adapter(
            &wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            },
        ).await.unwrap();

        // Use the adapter to create the device and queue.
        let (device, queue) = adapter.request_device(
            &wgpu::DeviceDescriptor {
                features: wgpu::Features::empty(),
                limits: wgpu::Limits::default(),
                label: None,
            },
            None,
        ).await.unwrap();

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

        // For depth test.
        let depth_texture = Texture::create_depth_texture(&device, (config.width, config.height), "depth texture");

        // Create our own render server.
        let mut render_server = RenderServer::new(&device, config.format);

        // Get the asset directory.
        let asset_dir = std::path::Path::new(env!("OUT_DIR")).join("assets");
        println!("Asset dir: {}", asset_dir.display());

        // Create nodes.
        // ---------------------------------------------------
        let mut world = World::new();

        let camera3d = Camera3d::new((0.0, 5.0, 10.0), cgmath::Deg(-90.0), cgmath::Deg(-20.0), &config, &device, &render_server);

        let camera2d = Camera2d::new(Point2::new(0.0, 0.0), (size.width, size.height), &config, &device);

        render_server.camera2d = Some(camera2d);
        render_server.camera3d = Some(camera3d);

        let vec_sprite = Box::new(VectorSprite::new(&device, &queue, &render_server));
        world.add_node(vec_sprite);

        let sprite_tex = Texture::load(&device, &queue, asset_dir.join("happy-tree.png")).unwrap();
        let sprite = Box::new(Sprite::new(&device, &queue, &render_server, sprite_tex));
        world.add_node(sprite);

        let skybox_tex = CubemapTexture::load(&device, &queue, asset_dir.join("labeled_skybox.png")).unwrap();
        let sky = Box::new(Sky::new(&device, &queue, &render_server, skybox_tex));
        world.add_node(sky);

        // Light.
        let light = Light::new(&device, &render_server);
        render_server.light = Some(light);

        // Load models.
        let obj_model = Box::new(Model::load(
            &device,
            &queue,
            &render_server.model_texture_bind_group_layout,
            asset_dir.join("viking_room/viking_room.obj"),
        ).unwrap());
        world.add_node(obj_model);

        // TODO: Light is not a [Node] yet.
        let light_model = Model::load(
            &device,
            &queue,
            &render_server.model_texture_bind_group_layout,
            asset_dir.join("sphere.obj"),
        ).unwrap();
        // ---------------------------------------------------

        Self {
            surface,
            device,
            queue,
            config,
            size,
            render_server,
            depth_texture,
            light_model,
            mouse_position: (0.0, 0.0),
            cursor_captured: false,
            previous_frame_time: 0.0,
            world,
        }
    }

    fn capture_cursor() {}

    fn release_cursor() {}

    /// Resize window.
    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        // Reconfigure the surface everytime the window's size changes.
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);

            // Create a new depth_texture and depth_texture_view.
            // Make sure you update the depth_texture after you update config.
            // If you don't, your program will crash as the depth_texture will be a different size than the surface texture.
            self.depth_texture = Texture::create_depth_texture(&self.device, (self.config.width, self.config.height), "depth_texture");

            self.render_server.camera3d.as_mut().unwrap().when_view_size_changes(new_size.width, new_size.height);
            self.render_server.camera2d.as_mut().unwrap().when_view_size_changes(new_size.width, new_size.height);
        }
    }

    /// Handle input events.
    fn input(&mut self, event: &DeviceEvent, window: &Window) -> bool {
        // Convert to our own input events.
        let input_event = match event {
            DeviceEvent::Key(
                KeyboardInput {
                    virtual_keycode: Some(key),
                    state,
                    ..
                }
            ) => {
                server::input_server::InputEvent::Key {
                    0: server::input_server::Key {
                        key: *key,
                        pressed: *state == ElementState::Pressed,
                    }
                }
            }
            DeviceEvent::MouseWheel { delta, .. } => {
                let scroll = match delta {
                    // I'm assuming a line is about 100 pixels.
                    MouseScrollDelta::LineDelta(_, scroll) => scroll * 100.0,
                    MouseScrollDelta::PixelDelta(PhysicalPosition {
                                                     y: scroll,
                                                     ..
                                                 }) => *scroll as f32,
                };

                server::input_server::InputEvent::MouseScroll {
                    0: server::input_server::MouseScroll {
                        delta: scroll,
                    }
                }
            }
            DeviceEvent::Button {
                button: button_id,
                state,
            } => {
                server::input_server::InputEvent::MouseButton {
                    0: server::input_server::MouseButton {
                        button: *button_id,
                        pressed: *state == ElementState::Pressed,
                        position: self.mouse_position,
                    }
                }
            }
            DeviceEvent::MouseMotion { delta } => {
                server::input_server::InputEvent::MouseMotion {
                    0: server::input_server::MouseMotion {
                        delta: (delta.0 as f32, delta.1 as f32),
                        position: self.mouse_position,
                    }
                }
            }
            _ => {
                server::input_server::InputEvent::Invalid
            }
        };

        // Pass input events to nodes.
        self.render_server.camera3d.as_mut().unwrap().input(input_event);

        self.render_server.camera3d.as_mut().unwrap().when_capture_state_changed(window);

        true
    }

    fn update(&mut self, dt: std::time::Duration) {
        let dt_in_secs = dt.as_secs_f32();

        // Update the cameras.
        self.render_server.camera3d.as_mut().unwrap().update(dt_in_secs, &self.queue);
        self.render_server.camera2d.as_mut().unwrap().update(dt_in_secs, &self.queue);

        // Update the light.
        self.render_server.light.as_mut().unwrap().update(dt_in_secs, &self.queue);

        self.world.update(&self.queue, dt_in_secs, &self.render_server);
    }

    fn render(&mut self, window: &Window) -> Result<(), wgpu::SurfaceError> {
        // First we need to get a frame to resource to.
        let output_surface = self.surface.get_current_texture()?;

        // Creates a TextureView with default settings.
        let view = output_surface.texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Builds a command buffer that we can then send to the GPU.
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("main render encoder"),
        });

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
                            load: wgpu::LoadOp::Clear(
                                wgpu::Color {
                                    r: 0.1,
                                    g: 0.2,
                                    b: 0.3,
                                    a: 1.0,
                                }
                            ),
                            store: true,
                        },
                    })
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

            // Draw light.
            // ----------------------
            render_pass.set_pipeline(&self.render_server.light_pipeline);

            render_pass.draw_light_model(
                &self.light_model,
                &self.render_server.camera3d.as_ref().unwrap().bind_group,
                &self.render_server.light.as_ref().unwrap().bind_group,
            );
            // ----------------------

            self.world.draw(&mut render_pass, &self.render_server);
        }

        // Finish the command buffer, and to submit it to the GPU's resource queue.
        // Submit will accept anything that implements IntoIter.
        self.queue.submit(std::iter::once(encoder.finish()));

        // Present the [`SurfaceTexture`].
        output_surface.present();

        Ok(())
    }
}

fn main() {
    env_logger::init();

    let event_loop = EventLoop::new();

    // Use cargo package name as the window title.
    let title = env!("CARGO_PKG_NAME");

    let window = WindowBuilder::new()
        .with_title(title)
        .with_inner_size(winit::dpi::PhysicalSize::new(
            INITIAL_WINDOW_WIDTH,
            INITIAL_WINDOW_HEIGHT,
        ))
        .build(&event_loop)
        .unwrap();

    // App::new uses async code, so we're going to wait for it to finish
    let mut app = pollster::block_on(App::new(&window));

    let start_time = std::time::Instant::now();

    // Used to calculate frame delta.
    let mut last_render_time = std::time::Instant::now();

    let mut is_init = false;

    // Main loop.
    event_loop.run(move |event, _, control_flow| {
        match event {
            // This handles input better.
            Event::DeviceEvent {
                ref event,
                .. // We're not using device_id currently
            } => {
                app.input(event, &window);
            }
            // Window event.
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == window.id() => {
                match event {
                    // Close window.
                    WindowEvent::CloseRequested | WindowEvent::KeyboardInput {
                        input:
                        KeyboardInput {
                            state: ElementState::Pressed,
                            virtual_keycode: Some(VirtualKeyCode::Escape),
                            ..
                        },
                        ..
                    } => *control_flow = ControlFlow::Exit,
                    // Resize window.
                    WindowEvent::Resized(physical_size) => {
                        // See https://github.com/rust-windowing/winit/issues/2094.
                        if is_init {
                            return;
                        }

                        app.resize(*physical_size);

                        println!("Window resized to {:?}", physical_size);
                    }
                    // Scale factor changed.
                    WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                        app.resize(**new_inner_size);

                        println!("Scale factor changed, new window size is {:?}", new_inner_size);
                    }
                    WindowEvent::CursorMoved { position, .. } => {
                        //let inner_size = window.inner_size();

                        // Move origin to bottom left.
                        //let y_position = inner_size.height as f64 - position.y;

                        app.mouse_position = ((position.x / window.scale_factor()) as f32,
                                              (position.y / window.scale_factor()) as f32);
                    }
                    _ => {}
                }
            }
            // Redraw request.
            Event::RedrawRequested(_) => {
                let now = std::time::Instant::now();
                let dt = now - last_render_time;
                last_render_time = now;

                app.update(dt);

                match app.render(&window) {
                    Ok(_) => {}
                    // Reconfigure the surface if lost.
                    Err(wgpu::SurfaceError::Lost) => app.resize(app.size),
                    // The system is out of memory, we should probably quit.
                    Err(wgpu::SurfaceError::OutOfMemory) => *control_flow = ControlFlow::Exit,
                    // All other errors (Outdated, Timeout) should be resolved by the next frame.
                    Err(e) => eprintln!("App resource error: {:?}", e),
                }
            }
            Event::MainEventsCleared => {
                // RedrawRequested will only trigger once, unless we manually request it.
                window.request_redraw();
            }
            Event::NewEvents(cause) => {
                if cause == StartCause::Init {
                    is_init = true;
                } else {
                    is_init = false;
                }
            }
            _ => {}
        }
    });
}
