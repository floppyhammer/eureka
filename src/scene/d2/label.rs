use crate::math::transform::Transform2d;
use crate::render::atlas::Atlas;

pub struct LabelComponent {
    pub text: String,
    pub text_is_dirty: bool,
    pub layout_is_dirty: bool,
    pub font_id: Option<String>,
    pub single_line: bool,
    pub leading: f32,
    pub tracking: f32,
    pub atlas: Option<Atlas>,
    pub last_global_transform: Transform2d,
}

impl LabelComponent {
    pub fn new(text: &str) -> Self {
        Self {
            text: text.to_string(),
            text_is_dirty: true,
            layout_is_dirty: true,
            font_id: None,
            single_line: false,
            leading: 20.0,
            tracking: 0.0,
            atlas: None,
            last_global_transform: Transform2d::default(),
        }
    }
}
