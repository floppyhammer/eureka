use crate::asset::AssetServer;
use crate::core::time::Time;
use crate::render::RenderContext;
use crate::text::FontServer;
use crate::window::InputServer;

pub struct Singletons {
    pub time: Time,
    pub render_context: RenderContext,
    pub input_server: InputServer,
    pub font_server: FontServer,
    pub asset_server: AssetServer,
}
