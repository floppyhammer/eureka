use cgmath::*;
use winit::event::*;
use winit::dpi::{LogicalPosition, PhysicalPosition, Position};
use std::time::Duration;
use std::f32::consts::FRAC_PI_2;
use cgmath::num_traits::clamp;
use wgpu::util::DeviceExt;
use winit::window::Window;
use crate::scene::node::WithInput;
use crate::server::input_server::InputEvent;

#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.0,
    0.0, 0.0, 0.5, 1.0,
);

const SAFE_FRAC_PI_2: f32 = FRAC_PI_2 - 0.0001;

pub struct Camera {
    position: Point3<f32>,
    yaw: Rad<f32>,
    pitch: Rad<f32>,

    projection: Projection,
    camera_controller: CameraController,
    camera_uniform: CameraUniform,
    camera_buffer: wgpu::Buffer,

    pub(crate) camera_bind_group_layout: wgpu::BindGroupLayout,
    pub(crate) camera_bind_group: wgpu::BindGroup,
}

impl Camera {
    pub fn new<
        V: Into<Point3<f32>>,
        Y: Into<Rad<f32>>,
        P: Into<Rad<f32>>,
    >(
        position: V,
        yaw: Y,
        pitch: P,
        config: &wgpu::SurfaceConfiguration,
        device: &wgpu::Device,
    ) -> Self {
        let projection = Projection::new(config.width, config.height, cgmath::Deg(45.0), 0.1, 100.0);
        let camera_controller = CameraController::new(4.0, 0.4);

        // This will be used in the model shader.
        let mut camera_uniform = CameraUniform::new();

        // Create a buffer for the camera uniform.
        let camera_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Camera Buffer"),
                contents: bytemuck::cast_slice(&[camera_uniform]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            }
        );

        // Bind group layout is used to create actual bind groups.
        // A bind group describes a set of resources and how they can be accessed by a shader.

        // Create a bind group layout for the camera buffer.
        let camera_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }
            ],
            label: Some("camera_bind_group_layout"),
        });

        // Create the actual bind group.
        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &camera_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera_buffer.as_entire_binding(),
                }
            ],
            label: Some("camera_bind_group"),
        });
        // ----------------------------

        Self {
            position: position.into(),
            yaw: yaw.into(),
            pitch: pitch.into(),
            projection,
            camera_controller,
            camera_uniform,
            camera_buffer,
            camera_bind_group_layout,
            camera_bind_group,
        }
    }

    /// Get view matrix.
    pub fn calc_matrix(&self) -> Matrix4<f32> {
        Matrix4::look_to_rh(
            self.position,
            Vector3::new(
                self.yaw.0.cos(),
                self.pitch.0.sin(),
                self.yaw.0.sin(),
            ).normalize(),
            Vector3::unit_y(),
        )
    }

    pub fn update(&mut self, dt: f32, queue: &wgpu::Queue) {
        // Update camera transform.
        {
            // Move forward/backward and left/right.
            let (yaw_sin, yaw_cos) = self.yaw.0.sin_cos();
            let (pitch_sin, pitch_cos) = self.pitch.0.sin_cos();
            let forward = Vector3::new(pitch_cos * yaw_cos, pitch_sin, pitch_cos * yaw_sin).normalize();
            let right = Vector3::new(-yaw_sin, 0.0, yaw_cos).normalize();
            self.position += forward * (self.camera_controller.amount_forward - self.camera_controller.amount_backward) * self.camera_controller.speed * dt;
            self.position += right * (self.camera_controller.amount_right - self.camera_controller.amount_left) * self.camera_controller.speed * dt;

            // Adjust navigation speed by scrolling.
            self.camera_controller.speed += self.camera_controller.scroll * 0.001;
            self.camera_controller.speed = clamp(self.camera_controller.speed, 0.1, 10.0);
            self.camera_controller.scroll = 0.0;

            // Move up/down. Since we don't use roll, we can just
            // modify the y coordinate directly.
            self.position.y += (self.camera_controller.amount_up - self.camera_controller.amount_down) * self.camera_controller.speed * dt;

            // Rotate.
            self.yaw += Rad(self.camera_controller.rotate_horizontal) * self.camera_controller.sensitivity * dt;
            self.pitch += Rad(-self.camera_controller.rotate_vertical) * self.camera_controller.sensitivity * dt;

            // If process_mouse isn't called every frame, these values
            // will not get set to zero, and the camera will rotate
            // when moving in a non cardinal direction.
            self.camera_controller.rotate_horizontal = 0.0;
            self.camera_controller.rotate_vertical = 0.0;

            // Keep the camera's angle from going too high/low.
            if self.pitch < -Rad(SAFE_FRAC_PI_2) {
                self.pitch = -Rad(SAFE_FRAC_PI_2);
            } else if self.pitch > Rad(SAFE_FRAC_PI_2) {
                self.pitch = Rad(SAFE_FRAC_PI_2);
            }
        }

        // Update camera uniform and its buffer.
        {
            // We're using Vector4 because of the uniforms 16 byte spacing requirement.
            self.camera_uniform.view_position = self.position.to_homogeneous().into();
            self.camera_uniform.view_proj = (self.projection.calc_matrix() * self.calc_matrix()).into();

            // Update camera buffer.
            queue.write_buffer(&self.camera_buffer, 0, bytemuck::cast_slice(&[self.camera_uniform]));
        }
    }

    pub fn when_view_size_changes(&mut self, new_width: u32, new_height: u32) {
        self.projection.resize(new_width, new_height);
    }

    pub fn when_capture_state_changed(&self, window: &Window) {
        if self.camera_controller.cursor_capture_state_changed {
            window.set_cursor_visible(!self.camera_controller.cursor_captured);

            // When right button releases, we need to set mouse position back to where
            // it was before being set invisible.
            if !self.camera_controller.cursor_captured {
                window.set_cursor_position(
                    Position::new(
                        LogicalPosition::new(
                            self.camera_controller.cursor_captured_position.x,
                            self.camera_controller.cursor_captured_position.y)
                    )
                );
            }
        }
    }
}

impl WithInput for Camera {
    fn input(&mut self, input: InputEvent) {
        self.camera_controller.cursor_capture_state_changed = false;

        match input {
            InputEvent::MouseButton(event) => {
                self.camera_controller.process_mouse_button(event.button, event.pressed);
            }
            InputEvent::MouseMotion(event) => {
                self.camera_controller.process_mouse_motion(
                    event.delta.0,
                    event.delta.1,
                    event.position.0,
                    event.position.1,
                );
            }
            InputEvent::MouseScroll(event) => {
                self.camera_controller.process_scroll(event.delta);
            }
            InputEvent::Key(event) => {
                self.camera_controller.process_keyboard(event.key, event.pressed);
            }
            _ => {}
        }
    }
}

/// The projection only really needs to change if the window resizes.
pub struct Projection {
    aspect: f32,
    fovy: Rad<f32>,
    znear: f32,
    zfar: f32,
}

impl Projection {
    pub fn new<F: Into<Rad<f32>>>(
        width: u32,
        height: u32,
        fovy: F,
        znear: f32,
        zfar: f32,
    ) -> Self {
        Self {
            aspect: width as f32 / height as f32,
            fovy: fovy.into(),
            znear,
            zfar,
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.aspect = width as f32 / height as f32;
    }

    /// Get projection matrix.
    pub fn calc_matrix(&self) -> Matrix4<f32> {
        OPENGL_TO_WGPU_MATRIX * perspective(self.fovy, self.aspect, self.znear, self.zfar)
    }
}

// We need this for Rust to store our data correctly for the shaders.
#[repr(C)]
// This is so we can store this in a buffer.
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    view_position: [f32; 4],
    /// Multiplication of the view and projection matrices.
    // We can't use cgmath with bytemuck directly so we'll have
    // to convert the Matrix4 into a 4x4 f32 array.
    view_proj: [[f32; 4]; 4],
}

impl CameraUniform {
    pub(crate) fn new() -> Self {
        use cgmath::SquareMatrix;
        Self {
            view_position: [0.0; 4],
            view_proj: cgmath::Matrix4::identity().into(),
        }
    }
}

#[derive(Debug)]
pub struct CameraController {
    amount_left: f32,
    amount_right: f32,
    amount_forward: f32,
    amount_backward: f32,
    amount_up: f32,
    amount_down: f32,
    rotate_horizontal: f32,
    rotate_vertical: f32,
    scroll: f32,
    speed: f32,
    sensitivity: f32,

    pub cursor_captured: bool,
    pub cursor_captured_position: cgmath::Vector2<f32>,
    pub(crate) cursor_capture_state_changed: bool,
}

impl CameraController {
    pub fn new(speed: f32, sensitivity: f32) -> Self {
        Self {
            amount_left: 0.0,
            amount_right: 0.0,
            amount_forward: 0.0,
            amount_backward: 0.0,
            amount_up: 0.0,
            amount_down: 0.0,
            rotate_horizontal: 0.0,
            rotate_vertical: 0.0,
            scroll: 0.0,
            speed,
            sensitivity,
            cursor_captured: false,
            cursor_captured_position: cgmath::Vector2::new(0.0, 0.0),
            cursor_capture_state_changed: false,
        }
    }

    pub fn process_keyboard(&mut self, key: VirtualKeyCode, pressed: bool) -> bool {
        let amount = if pressed { 1.0 } else { 0.0 };
        match key {
            VirtualKeyCode::W | VirtualKeyCode::Up => {
                self.amount_forward = amount;
                true
            }
            VirtualKeyCode::S | VirtualKeyCode::Down => {
                self.amount_backward = amount;
                true
            }
            VirtualKeyCode::A | VirtualKeyCode::Left => {
                self.amount_left = amount;
                true
            }
            VirtualKeyCode::D | VirtualKeyCode::Right => {
                self.amount_right = amount;
                true
            }
            VirtualKeyCode::E => {
                self.amount_up = amount;
                true
            }
            VirtualKeyCode::Q => {
                self.amount_down = amount;
                true
            }
            _ => false,
        }
    }

    pub fn process_mouse_motion(&mut self, mouse_dx: f32, mouse_dy: f32, mouse_x: f32, mouse_y: f32) {
        if self.cursor_captured {
            self.rotate_horizontal = mouse_dx;
            self.rotate_vertical = mouse_dy;
        } else {
            //println!("Cursor position updated: {:.1}, {:.1}", mouse_x, mouse_y);
            self.cursor_captured_position.x = mouse_x;
            self.cursor_captured_position.y = mouse_y;
        }
    }

    pub fn process_mouse_button(&mut self, button_id: u32, pressed: bool) {
        // Not the right button.
        if button_id != 3 {
            return;
        }

        let old_pressed = self.cursor_captured;

        if pressed != old_pressed {
            self.cursor_captured = pressed;

            self.cursor_capture_state_changed = true;
        }
    }

    pub fn process_scroll(&mut self, delta: f32) {
        self.scroll = delta;
    }
}
