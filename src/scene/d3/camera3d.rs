use crate::render::camera::CameraUniform;
use crate::window::{InputEvent, InputServer};
use glam::{Mat4, UVec2, Vec3};
use winit::event::MouseButton;
use winit::keyboard::KeyCode;

const DEFAULT_FOV: f32 = 60.0_f32.to_radians();
const DEFAULT_NEAR: f32 = 0.1;
const DEFAULT_FAR: f32 = 100.0;

pub struct Camera3dComponent {
    pub fov: f32,
    pub near: f32,
    pub far: f32,
    pub ssao_enabled: bool,
    pub fxaa_enabled: bool,
    pub volumetric_enabled: bool,
    pub viewport_size: UVec2,
}

impl Camera3dComponent {
    pub fn new() -> Self {
        Self {
            fov: DEFAULT_FOV,
            near: DEFAULT_NEAR,
            far: DEFAULT_FAR,
            ssao_enabled: true,
            fxaa_enabled: true,
            volumetric_enabled: true,
            viewport_size: UVec2::new(1280, 720),
        }
    }

    pub fn calc_view_matrix(&self, global_transform: &Mat4) -> Mat4 {
        let (scale, rotation, translation) = global_transform.to_scale_rotation_translation();
        let forward = rotation * Vec3::NEG_Z;

        Mat4::look_to_rh(translation, forward, Vec3::Y)
    }

    pub fn build_uniform(&self, global_transform: &Mat4) -> CameraUniform {
        let mut uniform = CameraUniform::default();

        let view_mat = self.calc_view_matrix(global_transform);
        let aspect_ratio = self.viewport_size.x as f32 / self.viewport_size.y as f32;
        let proj_mat = Mat4::perspective_rh(self.fov, aspect_ratio, self.near, self.far);

        let (_, _, translation) = global_transform.to_scale_rotation_translation();

        uniform.view_position = translation.extend(1.0).to_array();
        uniform.view = view_mat.to_cols_array_2d();
        uniform.proj = proj_mat.to_cols_array_2d();
        uniform.view_proj = (proj_mat * view_mat).to_cols_array_2d();
        uniform.inv_proj = proj_mat.inverse().to_cols_array_2d();
        uniform.inv_view = view_mat.inverse().to_cols_array_2d();
        uniform.ssao_enabled = if self.ssao_enabled { 1 } else { 0 };
        uniform.volumetric_enabled = if self.volumetric_enabled { 1 } else { 0 };

        uniform
    }
}

pub struct Camera3dController {
    pub amount_left: f32,
    pub amount_right: f32,
    pub amount_forward: f32,
    pub amount_backward: f32,
    pub amount_up: f32,
    pub amount_down: f32,
    pub yaw: f32,
    pub pitch: f32,
    pub scroll: f32,
    pub speed: f32,
    pub sensitivity: f32,
    pub cursor_captured: bool,
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
            yaw: 0.0,
            pitch: 0.0,
            scroll: 0.0,
            speed,
            sensitivity,
            cursor_captured: false,
        }
    }

    /// 处理输入事件，更新控制器状态
    pub fn handle_input(&mut self, event: &InputEvent, input_server: &mut InputServer) {
        match event {
            InputEvent::MouseButton(e) => {
                if e.button == MouseButton::Right {
                    self.cursor_captured = e.pressed;
                    input_server.set_cursor_capture(e.pressed);
                }
            }
            InputEvent::MouseMotion(e) => {
                if self.cursor_captured {
                    self.yaw -= e.delta.0 * self.sensitivity;
                    self.pitch -= e.delta.1 * self.sensitivity;
                    self.pitch = self
                        .pitch
                        .clamp(-89.0f32.to_radians(), 89.0f32.to_radians());
                }
            }
            InputEvent::Key(e) => {
                let amount = if e.pressed { 1.0 } else { 0.0 };
                match e.key_code {
                    KeyCode::KeyW => self.amount_forward = amount,
                    KeyCode::KeyS => self.amount_backward = amount,
                    KeyCode::KeyA => self.amount_left = amount,
                    KeyCode::KeyD => self.amount_right = amount,
                    KeyCode::KeyE => self.amount_up = amount,
                    KeyCode::KeyQ => self.amount_down = amount,
                    _ => {}
                }
            }
            _ => {}
        }
    }
}
