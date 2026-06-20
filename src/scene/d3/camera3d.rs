use crate::render::camera::CameraUniform;
use crate::window::{InputContent, InputEvent, InputServer};
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
    pub taa_enabled: bool,
    pub volumetric_enabled: bool,
    pub ssr_enabled: bool,
    pub viewport_size: UVec2,
    pub frame_count: u64,
    pub prev_view_proj: Mat4, // 新增：保存上一帧的矩阵
}

impl Camera3dComponent {
    pub fn new() -> Self {
        Self {
            fov: DEFAULT_FOV,
            near: DEFAULT_NEAR,
            far: DEFAULT_FAR,
            ssao_enabled: true,
            fxaa_enabled: false,
            taa_enabled: true,
            volumetric_enabled: false,
            ssr_enabled: false,
            viewport_size: UVec2::new(1280, 720),
            frame_count: 0,
            prev_view_proj: Mat4::IDENTITY,
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
        let unjittered_proj = Mat4::perspective_rh(self.fov, aspect_ratio, self.near, self.far);
        let mut proj_mat = unjittered_proj;

        // TAA Jittering
        if self.taa_enabled {
            let jitter = self.get_halton_jitter(self.frame_count);
            let prev_jitter = self.get_halton_jitter(self.frame_count.wrapping_sub(1));

            let jitter_x = (jitter.0 * 2.0 - 1.0) / self.viewport_size.x as f32;
            let jitter_y = (jitter.1 * 2.0 - 1.0) / self.viewport_size.y as f32;

            let prev_jitter_x = (prev_jitter.0 * 2.0 - 1.0) / self.viewport_size.x as f32;
            let prev_jitter_y = (prev_jitter.1 * 2.0 - 1.0) / self.viewport_size.y as f32;

            proj_mat.col_mut(2).x += jitter_x;
            proj_mat.col_mut(2).y += jitter_y;

            uniform.jitter = [jitter_x, jitter_y, prev_jitter_x, prev_jitter_y];
        }

        let (_, _, translation) = global_transform.to_scale_rotation_translation();

        uniform.view_position = translation.extend(1.0).to_array();
        uniform.view = view_mat.to_cols_array_2d();
        uniform.proj = proj_mat.to_cols_array_2d();
        uniform.unjittered_proj = unjittered_proj.to_cols_array_2d();

        let view_proj = proj_mat * view_mat;
        let unjittered_view_proj = unjittered_proj * view_mat;

        uniform.view_proj = view_proj.to_cols_array_2d();
        uniform.unjittered_view_proj = unjittered_view_proj.to_cols_array_2d();

        uniform.inv_proj = proj_mat.inverse().to_cols_array_2d();
        uniform.inv_view = view_mat.inverse().to_cols_array_2d();
        uniform.inv_view_proj = view_proj.inverse().to_cols_array_2d();
        uniform.inv_unjittered_view_proj = unjittered_view_proj.inverse().to_cols_array_2d(); // 新增

        // 关键：这里使用的是存储的上一帧矩阵
        uniform.prev_view_proj = self.prev_view_proj.to_cols_array_2d();

        uniform.ssao_enabled = if self.ssao_enabled { 1 } else { 0 };
        uniform.volumetric_enabled = if self.volumetric_enabled { 1 } else { 0 };
        uniform.taa_enabled = if self.taa_enabled { 1 } else { 0 };
        uniform.ssr_enabled = if self.ssr_enabled { 1 } else { 0 };
        uniform.frame_count = self.frame_count as u32;
        uniform._pad = [0; 3];

        uniform
    }

    /// 在 extract 之后调用，手动更新历史矩阵 (始终保存 Unjittered 矩阵)
    pub fn update_after_extract(&mut self, global_transform: &Mat4) {
        let view_mat = self.calc_view_matrix(global_transform);
        let aspect_ratio = self.viewport_size.x as f32 / self.viewport_size.y as f32;
        let proj_mat = Mat4::perspective_rh(self.fov, aspect_ratio, self.near, self.far);
        self.prev_view_proj = proj_mat * view_mat;
    }

    fn get_halton_jitter(&self, index: u64) -> (f32, f32) {
        fn halton(mut i: u64, base: u64) -> f32 {
            let mut f = 1.0;
            let mut r = 0.0;
            while i > 0 {
                f /= base as f32;
                r += f * (i % base) as f32;
                i /= base;
            }
            r
        }

        // 使用 8 帧为一个周期的 Halton 序列 (2, 3)
        let idx = (index % 8) + 1;
        (halton(idx, 2), halton(idx, 3))
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
        match &event.content {
            InputContent::MouseButton(e) => {
                if e.button == MouseButton::Right {
                    self.cursor_captured = e.pressed;
                    input_server.set_cursor_capture(e.pressed);
                }
            }
            InputContent::MouseMotion(e) => {
                if self.cursor_captured {
                    self.yaw -= e.delta.0 * self.sensitivity;
                    self.pitch -= e.delta.1 * self.sensitivity;
                    self.pitch = self
                        .pitch
                        .clamp(-89.0f32.to_radians(), 89.0f32.to_radians());
                }
            }
            InputContent::Key(e) => {
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
