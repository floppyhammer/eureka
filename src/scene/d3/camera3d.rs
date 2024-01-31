use crate::render::camera::{CameraType, CameraUniform, PerspectiveProjection, Projection};
use crate::render::draw_command::DrawCommands;
use crate::scene::{AsNode, NodeType};
use crate::window::{InputEvent, InputServer, Key};
use crate::{RenderServer, Singletons};
use cgmath::num_traits::clamp;
use cgmath::*;
use std::any::Any;
use std::f32::consts::FRAC_PI_2;
use std::mem;
use std::rc::Rc;
use std::time::Duration;
use wgpu::util::DeviceExt;
use wgpu::BufferAddress;
use winit::dpi::{LogicalPosition, PhysicalPosition, Position};
use winit::event::*;
use winit::keyboard::Key::{Named, Unidentified};
use winit::keyboard::{KeyCode, NamedKey, NativeKeyCode};
use winit::window::Window;

#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.0,
    0.0, 0.0, 0.5, 1.0,
);

const SAFE_FRAC_PI_2: f32 = FRAC_PI_2 - 0.0001;

pub struct Camera3d {
    position: Point3<f32>,
    yaw: Rad<f32>,
    pitch: Rad<f32>,
    fov: f32,

    projection: Projection,

    controller: Camera3dController,
}

impl Camera3d {
    pub fn new<V: Into<Point3<f32>>, Y: Into<Rad<f32>>, P: Into<Rad<f32>>>(
        position: V,
        yaw: Y,
        pitch: P,
        render_server: &RenderServer,
    ) -> Self {
        let device = &render_server.device;
        let config = &render_server.surface_config;

        let fov = 45.0;

        let projection = PerspectiveProjection::new(
            config.width, // Render target size
            config.height,
            cgmath::Deg(fov),
            0.1,
            100.0,
        );

        let controller = Camera3dController::new(4.0, 0.4);

        Self {
            position: position.into(),
            yaw: yaw.into(),
            pitch: pitch.into(),
            fov,
            projection: projection.into(),
            controller,
        }
    }

    /// Get view matrix.
    pub fn calc_view_matrix(&self) -> Matrix4<f32> {
        let (sin_pitch, cos_pitch) = self.pitch.0.sin_cos();
        let (sin_yaw, cos_yaw) = self.yaw.0.sin_cos();

        // Refer to https://learnopengl.com/Getting-started/Camera.
        Matrix4::look_to_rh(
            self.position,
            Vector3::new(cos_pitch * cos_yaw, sin_pitch, cos_pitch * sin_yaw).normalize(),
            Vector3::unit_y(),
        )
    }

    pub fn calc_view_matrix_without_pos(&self) -> Matrix4<f32> {
        Matrix4::look_to_rh(
            Point3::new(0.0, 0.0, 0.0),
            Vector3::new(self.yaw.0.cos(), self.pitch.0.sin(), self.yaw.0.sin()).normalize(),
            Vector3::unit_y(),
        )
    }

    pub fn when_view_size_changes(&mut self, new_size: Vector2<u32>) {
        self.projection.update(new_size.x as f32, new_size.y as f32);
    }
}

// We need this for Rust to store our data correctly for the shaders.
#[repr(C)]
// This is so we can store this in a buffer.
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ModelUniform {
    model: [[f32; 4]; 4],
}

impl ModelUniform {
    pub(crate) fn new() -> Self {
        use cgmath::SquareMatrix;
        Self {
            model: cgmath::Matrix4::identity().into(),
        }
    }
}

/// A simple 3D fly camera controller.
#[derive(Debug)]
pub struct Camera3dController {
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

impl Camera3dController {
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

    pub fn process_keyboard(&mut self, key: KeyCode, pressed: bool) -> bool {
        let amount = if pressed { 1.0 } else { 0.0 };
        match key {
            KeyCode::KeyW => {
                self.amount_forward = amount;
                true
            }
            KeyCode::KeyS => {
                self.amount_backward = amount;
                true
            }
            KeyCode::KeyA => {
                self.amount_left = amount;
                true
            }
            KeyCode::KeyD => {
                self.amount_right = amount;
                true
            }
            KeyCode::KeyE => {
                self.amount_up = amount;
                true
            }
            KeyCode::KeyQ => {
                self.amount_down = amount;
                true
            }
            _ => false,
        }
    }

    pub fn process_mouse_motion(
        &mut self,
        mouse_dx: f32,
        mouse_dy: f32,
        mouse_x: f32,
        mouse_y: f32,
    ) {
        if self.cursor_captured {
            self.rotate_horizontal = mouse_dx;
            self.rotate_vertical = mouse_dy;
        } else {
            self.cursor_captured_position.x = mouse_x;
            self.cursor_captured_position.y = mouse_y;
        }
    }

    pub fn process_mouse_button(&mut self, button: MouseButton, pressed: bool) {
        // If the right button is not pressed.
        if button != MouseButton::Right {
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

impl AsNode for Camera3d {
    fn node_type(&self) -> NodeType {
        NodeType::Camera3d
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn input(&mut self, input_event: &mut InputEvent, input_server: &mut InputServer) {
        self.controller.cursor_capture_state_changed = false;

        match input_event {
            InputEvent::MouseButton(event) => {
                self.controller
                    .process_mouse_button(event.button, event.pressed);
            }
            InputEvent::MouseMotion(event) => {
                self.controller.process_mouse_motion(
                    event.delta.0,
                    event.delta.1,
                    event.position.0,
                    event.position.1,
                );
            }
            InputEvent::MouseScroll(event) => {
                self.controller.process_scroll(event.delta);
            }
            InputEvent::Key(key) => {
                self.controller.process_keyboard(key.key_code, key.pressed);
            }
            _ => {}
        }

        if self.controller.cursor_capture_state_changed {
            input_server.set_cursor_capture(self.controller.cursor_captured);
        }
    }

    fn update(&mut self, dt: f32, singletons: &mut Singletons) {
        self.projection.update(
            singletons.render_server.surface_config.width as f32,
            singletons.render_server.surface_config.height as f32,
        );

        // Update camera transform.
        {
            // Move forward/backward and left/right.
            let (yaw_sin, yaw_cos) = self.yaw.0.sin_cos();
            let (pitch_sin, pitch_cos) = self.pitch.0.sin_cos();
            let forward =
                Vector3::new(pitch_cos * yaw_cos, pitch_sin, pitch_cos * yaw_sin).normalize();
            let right = Vector3::new(-yaw_sin, 0.0, yaw_cos).normalize();
            self.position += forward
                * (self.controller.amount_forward - self.controller.amount_backward)
                * self.controller.speed
                * dt;
            self.position += right
                * (self.controller.amount_right - self.controller.amount_left)
                * self.controller.speed
                * dt;

            // Adjust navigation speed by scrolling.
            self.controller.speed += self.controller.scroll * 0.001;
            self.controller.speed = clamp(self.controller.speed, 0.1, 10.0);
            self.controller.scroll = 0.0;

            // Move up/down. Since we don't use roll, we can just
            // modify the y coordinate directly.
            self.position.y += (self.controller.amount_up - self.controller.amount_down)
                * self.controller.speed
                * dt;

            // Horizontal rotation.
            self.yaw += Rad(self.controller.rotate_horizontal) * self.controller.sensitivity * dt;

            // Vertical rotation.
            self.pitch += Rad(-self.controller.rotate_vertical) * self.controller.sensitivity * dt;

            // If process_mouse isn't called every frame, these values
            // will not get set to zero, and the camera will rotate
            // when moving in a non cardinal direction.
            self.controller.rotate_horizontal = 0.0;
            self.controller.rotate_vertical = 0.0;

            // Keep the camera's angle from going too high/low.
            if self.pitch < -Rad(SAFE_FRAC_PI_2) {
                self.pitch = -Rad(SAFE_FRAC_PI_2);
            } else if self.pitch > Rad(SAFE_FRAC_PI_2) {
                self.pitch = Rad(SAFE_FRAC_PI_2);
            }
        }

        // Update camera uniform and its buffer.
        {}
    }

    fn draw(&self, draw_cmds: &mut DrawCommands) {
        let mut uniform = CameraUniform::default();

        let view_mat = self.calc_view_matrix();
        let proj_mat = self.projection.calc_matrix();

        uniform.view_position = self.position.to_homogeneous().into();
        uniform.view = view_mat.into();
        uniform.proj = proj_mat.into();

        uniform.view_proj = (proj_mat * view_mat).into();

        draw_cmds.extracted.cameras.add(CameraType::D3, uniform);
    }
}
