use crate::core::singleton::Singletons;
use crate::render::camera::{CameraType, CameraUniform, PerspectiveProjection, Projection};
use crate::render::draw_command::DrawCommands;
use crate::render::RenderContext;
use crate::scene::{AsNode, NodeType};
use crate::window::{InputEvent, InputServer};
use glam::{Mat4, UVec2, Vec2, Vec3};
use std::any::Any;
use std::f32::consts::FRAC_PI_2;
use winit::event::*;
use winit::keyboard::KeyCode;

const SAFE_FRAC_PI_2: f32 = FRAC_PI_2 - 0.0001;
const DEFAULT_FOV: f32 = 60.0_f32.to_radians();

pub struct Camera3d {
    position: Vec3,
    yaw: f32,
    pitch: f32,
    fov: f32,

    projection: Projection,

    pub ssao_enabled: bool,

    controller: Camera3dController,
}

impl Camera3d {
    pub fn new(
        position: Vec3,
        yaw_radians: f32,
        pitch_radians: f32,
        render_server: &RenderContext,
    ) -> Self {
        let config = &render_server.surface_config;

        let fov = DEFAULT_FOV;

        let projection = PerspectiveProjection::new(
            config.width, // Render target size
            config.height,
            fov,
            0.1,
            100.0,
        );

        let controller = Camera3dController::new(4.0, 0.003);

        Self {
            position,
            yaw: yaw_radians,
            pitch: pitch_radians,
            fov,
            projection: projection.into(),
            ssao_enabled: true,
            controller,
        }
    }

    /// Get view matrix.
    pub fn calc_view_matrix(&self) -> Mat4 {
        let (sin_pitch, cos_pitch) = self.pitch.sin_cos();
        let (sin_yaw, cos_yaw) = self.yaw.sin_cos();

        Mat4::look_to_rh(
            self.position,
            Vec3::new(cos_pitch * cos_yaw, sin_pitch, cos_pitch * sin_yaw).normalize(),
            Vec3::Y,
        )
    }

    pub fn calc_view_matrix_without_pos(&self) -> Mat4 {
        Mat4::look_to_rh(
            Vec3::ZERO,
            Vec3::new(self.yaw.cos(), self.pitch.sin(), self.yaw.sin()).normalize(),
            Vec3::Y,
        )
    }

    pub fn when_view_size_changes(&mut self, new_size: UVec2) {
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
        Self {
            model: Mat4::IDENTITY.to_cols_array_2d(),
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
    pub cursor_captured_position: Vec2,
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
            cursor_captured_position: Vec2::ZERO,
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
            self.rotate_horizontal += mouse_dx;
            self.rotate_vertical += mouse_dy;
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
            singletons.render_context.surface_config.width as f32,
            singletons.render_context.surface_config.height as f32,
        );

        // Update camera transform.
        {
            // Move forward/backward and left/right.
            let (yaw_sin, yaw_cos) = self.yaw.sin_cos();
            let (pitch_sin, pitch_cos) = self.pitch.sin_cos();
            let forward =
                Vec3::new(pitch_cos * yaw_cos, pitch_sin, pitch_cos * yaw_sin).normalize();
            let right = Vec3::new(-yaw_sin, 0.0, yaw_cos).normalize();
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
            self.controller.speed = self.controller.speed.clamp(0.1, 10.0);
            self.controller.scroll = 0.0;

            // Move up/down. Since we don't use roll, we can just
            // modify the y coordinate directly.
            self.position.y += (self.controller.amount_up - self.controller.amount_down)
                * self.controller.speed
                * dt;

            // Horizontal rotation (Yaw)
            self.yaw += self.controller.rotate_horizontal * self.controller.sensitivity;

            // Vertical rotation (Pitch)
            self.pitch += -self.controller.rotate_vertical * self.controller.sensitivity;

            // If process_mouse isn't called every frame, these values
            // will not get set to zero, and the camera will rotate
            // when moving in a non cardinal direction.
            self.controller.rotate_horizontal = 0.0;
            self.controller.rotate_vertical = 0.0;

            // Keep the camera's angle from going too high/low.
            if self.pitch < -SAFE_FRAC_PI_2 {
                self.pitch = -SAFE_FRAC_PI_2;
            } else if self.pitch > SAFE_FRAC_PI_2 {
                self.pitch = SAFE_FRAC_PI_2;
            }
        }
    }

    fn draw(&self, draw_cmds: &mut DrawCommands) {
        let mut uniform = CameraUniform::default();

        let view_mat = self.calc_view_matrix();
        let proj_mat = self.projection.calc_matrix();

        uniform.view_position = self.position.extend(1.0).to_array();
        uniform.view = view_mat.to_cols_array_2d();
        uniform.proj = proj_mat.to_cols_array_2d();
        uniform.view_proj = (proj_mat * view_mat).to_cols_array_2d();
        uniform.inv_proj = proj_mat.inverse().to_cols_array_2d();
        uniform.ssao_enabled = if self.ssao_enabled { 1 } else { 0 };

        draw_cmds.extracted.cameras.add(CameraType::D3, uniform);
    }
}
