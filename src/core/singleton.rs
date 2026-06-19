use crate::asset::AssetManager;
use crate::core::time::Time;
use crate::render::RenderContext;
use crate::text::TextServer;
use crate::window::InputServer;

pub struct Singletons {
    pub time: Time,
    pub render_context: RenderContext,
    pub input_server: InputServer,
    pub text_server: TextServer,
    pub asset_manager: AssetManager,
}
