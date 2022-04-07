use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
    window::Window,
};
use wgpu::util::DeviceExt;
use cgmath::prelude::*;
use winit::dpi::{LogicalPosition, PhysicalPosition, Position, Size};

use wgpu::SamplerBindingType;

// Do this before importing local crates.
mod render;
mod scene;

// Import local crates.
use crate::render::{DrawModel, DrawLight, Model, Vertex, Texture, LightUniform};
use crate::scene::{Camera, Projection, CameraController, InputEvent, WithInput};

const INITIAL_WINDOW_WIDTH: u32 = 1280;
const INITIAL_WINDOW_HEIGHT: u32 = 720;

// Instancing.
const NUM_INSTANCES_PER_ROW: u32 = 1;
const INSTANCE_DISPLACEMENT: cgmath::Vector3<f32> = cgmath::Vector3::new(NUM_INSTANCES_PER_ROW as f32 * 0.5, 0.0, NUM_INSTANCES_PER_ROW as f32 * 0.5);

// For convenience we're going to pack all the fields into a struct,
// and create some methods on that.
struct State {
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
    render_pipeline: wgpu::RenderPipeline,
    light_render_pipeline: wgpu::RenderPipeline,
    camera: Camera,
    // Instancing.
    instances: Vec<render::model::Instance>,
    instance_buffer: wgpu::Buffer,
    depth_texture: Texture,
    obj_model: Model,
    light_model: Model,
    light_uniform: LightUniform,
    light_buffer: wgpu::Buffer,
    light_bind_group: wgpu::BindGroup,
    mouse_position: (f32, f32),
    cursor_captured: bool,
}

impl State {
    // Creating some of the wgpu types requires async code.
    async fn new(window: &Window) -> Self {
        // Get window's inner size.
        let size = window.inner_size();

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

        // Let's use the adapter to create the device and queue.
        let (device, queue) = adapter.request_device(
            &wgpu::DeviceDescriptor {
                features: wgpu::Features::empty(),
                limits: wgpu::Limits::default(),
                label: None,
            },
            None, // Trace path.
        ).await.unwrap();

        // This will define how the surface creates its underlying SurfaceTextures.
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface.get_preferred_format(&adapter).unwrap(),
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
        };
        surface.configure(&device, &config);

        // Create camera.
        let camera = Camera::new((0.0, 5.0, 10.0), cgmath::Deg(-90.0), cgmath::Deg(-20.0), &config, &device);

        // Model textures.
        let texture_bind_group_layout = device.create_bind_group_layout(
            &wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    // Diffuse texture.
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler {
                            0: SamplerBindingType::Filtering,
                        },
                        count: None,
                    },
                    // Normal texture.
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler {
                            0: SamplerBindingType::Filtering,
                        },
                        count: None,
                    },
                ],
                label: Some("texture_bind_group_layout"),
            }
        );

        // Light.
        // -------------------------------
        let light_uniform = LightUniform {
            position: [2.0, 2.0, 2.0],
            _padding: 0,
            color: [1.0, 1.0, 1.0],
            _padding2: 0,
        };

        // We'll want to update our lights position, so we use COPY_DST.
        let light_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Light uniform buffer"),
                contents: bytemuck::cast_slice(&[light_uniform]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            }
        );

        let light_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: None,
            });

        let light_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &light_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: light_buffer.as_entire_binding(),
            }],
            label: None,
        });
        // -------------------------------

        // Pipeline to model.
        let render_pipeline = {
            // Set up render pipeline layout using bind group layouts.
            let layout =
                device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Model Render Pipeline Layout"),
                    bind_group_layouts: &[
                        &texture_bind_group_layout,
                        &camera.camera_bind_group_layout,
                        &light_bind_group_layout,
                    ],
                    push_constant_ranges: &[],
                });

            // Shader descriptor, not a shader module yet.
            let shader = wgpu::ShaderModuleDescriptor {
                label: Some("Model Shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("shader/shader.wgsl").into()),
            };

            render::server::create_render_pipeline(
                &device,
                &layout,
                config.format,
                Some(render::texture::Texture::DEPTH_FORMAT),
                &[render::model::MeshVertex::desc(), render::model::InstanceRaw::desc()],
                shader,
            )
        };

        // Pipeline to draw light source.
        let light_render_pipeline = {
            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Light Render Pipeline Layout"),
                bind_group_layouts: &[&camera.camera_bind_group_layout, &light_bind_group_layout],
                push_constant_ranges: &[],
            });

            let shader = wgpu::ShaderModuleDescriptor {
                label: Some("Light Shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("shader/light.wgsl").into()),
            };

            render::server::create_render_pipeline(
                &device,
                &layout,
                config.format,
                Some(render::texture::Texture::DEPTH_FORMAT),
                &[render::model::MeshVertex::desc()],
                shader,
            )
        };

        // Get the asset directory.
        let asset_dir = std::path::Path::new(env!("OUT_DIR")).join("assets");

        println!("Asset dir: {}", asset_dir.display());

        // Load models.
        let obj_model = Model::load(
            &device,
            &queue,
            &texture_bind_group_layout,
            asset_dir.join("viking_room/viking_room.obj"),
        ).unwrap();
        let light_model = Model::load(
            &device,
            &queue,
            &texture_bind_group_layout,
            asset_dir.join("sphere.obj"),
        ).unwrap();

        // Instance data.
        const SPACE_BETWEEN: f32 = 3.0;
        let instances = (0..NUM_INSTANCES_PER_ROW).flat_map(|z| {
            (0..NUM_INSTANCES_PER_ROW).map(move |x| {
                let x = SPACE_BETWEEN * (x as f32 - NUM_INSTANCES_PER_ROW as f32 / 2.0);
                let z = SPACE_BETWEEN * (z as f32 - NUM_INSTANCES_PER_ROW as f32 / 2.0);

                let position = cgmath::Vector3 { x, y: 0.0, z };

                let rotation = if position.is_zero() {
                    // This is needed so an object at (0, 0, 0) won't get scaled to zero
                    // as Quaternions can effect scale if they're not created correctly.
                    cgmath::Quaternion::from_axis_angle(cgmath::Vector3::unit_z(), cgmath::Deg(0.0))
                } else {
                    cgmath::Quaternion::from_axis_angle(position.normalize(), cgmath::Deg(45.0))
                };

                render::model::Instance {
                    position,
                    rotation,
                }
            })
        }).collect::<Vec<_>>();

        // Create the instance buffer.
        let instance_data = instances.iter().map(render::model::Instance::to_raw).collect::<Vec<_>>();
        let instance_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Instance Buffer"),
                contents: bytemuck::cast_slice(&instance_data),
                usage: wgpu::BufferUsages::VERTEX,
            }
        );

        // For depth test.
        let depth_texture = Texture::create_depth_texture(&device, &config, "depth_texture");

        Self {
            surface,
            device,
            queue,
            config,
            size,
            render_pipeline,
            light_render_pipeline,
            camera,
            instances,
            instance_buffer,
            depth_texture,
            obj_model,
            light_model,
            light_uniform,
            light_buffer,
            light_bind_group,
            mouse_position: (0.0, 0.0),
            cursor_captured: false,
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
            self.depth_texture = Texture::create_depth_texture(&self.device, &self.config, "depth_texture");

            self.camera.when_view_size_changes(new_size.width, new_size.height);
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
                scene::input_event::InputEvent::Key {
                    0: scene::input_event::Key {
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

                scene::input_event::InputEvent::MouseScroll {
                    0: scene::input_event::MouseScroll {
                        delta: scroll,
                    }
                }
            }
            DeviceEvent::Button {
                button: button_id,
                state,
            } => {
                scene::input_event::InputEvent::MouseButton {
                    0: scene::input_event::MouseButton {
                        button: *button_id,
                        pressed: *state == ElementState::Pressed,
                        position: self.mouse_position,
                    }
                }
            }
            DeviceEvent::MouseMotion { delta } => {
                scene::input_event::InputEvent::MouseMotion {
                    0: scene::input_event::MouseMotion {
                        delta: (delta.0 as f32, delta.1 as f32),
                        position: self.mouse_position,
                    }
                }
            }
            _ => {
                scene::input_event::InputEvent::Invalid
            }
        };

        // Pass input events to nodes.
        self.camera.input(input_event);

        self.camera.when_capture_state_changed(window);

        true
    }

    fn update(&mut self, dt: std::time::Duration) {
        // Update camera.
        self.camera.update(dt.as_secs_f32(), &self.queue);

        // Update the light.
        {
            let old_position: cgmath::Vector3<_> = self.light_uniform.position.into();
            self.light_uniform.position =
                (cgmath::Quaternion::from_axis_angle((0.0, 1.0, 0.0).into(), cgmath::Deg(60.0 * dt.as_secs_f32()))
                    * old_position).into();

            // Update light buffer.
            self.queue.write_buffer(&self.light_buffer, 0, bytemuck::cast_slice(&[self.light_uniform]));
        }
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        // First we need to get a frame to render to.
        let output = self.surface.get_current_texture()?;

        // Creates a TextureView with default settings.
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Builds a command buffer that we can then send to the gpu.
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

        // The RenderPass has all the methods to do the actual drawing.
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[
                    // This is what [[location(0)]] in the fragment shader targets.
                    wgpu::RenderPassColorAttachment {
                        view: &view,
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
                    }
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
            render_pass.set_pipeline(&self.light_render_pipeline);

            render_pass.draw_light_model(
                &self.light_model,
                &self.camera.camera_bind_group,
                &self.light_bind_group,
            );
            // ----------------------

            // Draw model.
            // ----------------------
            // Set vertex buffer for InstanceInput.
            render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));

            render_pass.set_pipeline(&self.render_pipeline);

            render_pass.draw_model_instanced(
                &self.obj_model,
                0..self.instances.len() as u32,
                &self.camera.camera_bind_group,
                &self.light_bind_group,
            );
            // ----------------------
        }

        // Finish the command buffer, and to submit it to the GPU's render queue.
        // Submit will accept anything that implements IntoIter.
        self.queue.submit(std::iter::once(encoder.finish()));

        // Present the [`SurfaceTexture`].
        output.present();

        Ok(())
    }
}

fn main() {
    env_logger::init();

    let event_loop = EventLoop::new();
    let title = env!("CARGO_PKG_NAME");

    let window = WindowBuilder::new()
        .with_title(title)
        .with_inner_size(winit::dpi::PhysicalSize::new(
            INITIAL_WINDOW_WIDTH,
            INITIAL_WINDOW_HEIGHT,
        ))
        .build(&event_loop)
        .unwrap();

    // State::new uses async code, so we're going to wait for it to finish
    let mut state = pollster::block_on(State::new(&window));

    // Used to calculate frame delta.
    let mut last_render_time = std::time::Instant::now();

    // Main loop.
    event_loop.run(move |event, _, control_flow| {
        match event {
            // This handles input better.
            Event::DeviceEvent {
                ref event,
                .. // We're not using device_id currently
            } => {
                state.input(event, &window);
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
                        state.resize(*physical_size);
                    }
                    // Scale factor changed.
                    WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                        state.resize(**new_inner_size);
                    }
                    WindowEvent::CursorMoved { position, .. } => {
                        //let inner_size = window.inner_size();

                        // Move origin to bottom left.
                        //let y_position = inner_size.height as f64 - position.y;

                        state.mouse_position = ((position.x / window.scale_factor()) as f32,
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

                state.update(dt);

                match state.render() {
                    Ok(_) => {}
                    // Reconfigure the surface if lost.
                    Err(wgpu::SurfaceError::Lost) => state.resize(state.size),
                    // The system is out of memory, we should probably quit.
                    Err(wgpu::SurfaceError::OutOfMemory) => *control_flow = ControlFlow::Exit,
                    // All other errors (Outdated, Timeout) should be resolved by the next frame.
                    Err(e) => eprintln!("State render error: {:?}", e),
                }
            }
            Event::MainEventsCleared => {
                // RedrawRequested will only trigger once, unless we manually request it.
                window.request_redraw();
            }
            _ => {}
        }
    });
}
