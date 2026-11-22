use crate::math::transform::Transform2d;
use crate::render::atlas::Atlas;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextMode {
    Raster,
    Msdf,
}

pub struct LabelComponent {
    pub text: String,
    pub text_is_dirty: bool,
    pub layout_is_dirty: bool,
    pub font_id: Option<String>,
    pub single_line: bool,
    pub leading: f32,
    pub tracking: f32,
    pub(crate) atlas: Option<Atlas>,
    pub last_global_transform: Transform2d,
    pub mode: TextMode,
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
            mode: TextMode::Raster,
        }
    }

    /// 更新文本内容并自动标记 text_is_dirty，确保 UI 渲染层重新生成字体图集
    pub fn set_text(&mut self, text: impl Into<String>) {
        let new_text = text.into();
        if self.text != new_text {
            self.text = new_text;
            self.text_is_dirty = true;
            self.layout_is_dirty = true;
        }
    }

    pub fn with_mode(mut self, mode: TextMode) -> Self {
        self.mode = mode;
        self
    }
}
