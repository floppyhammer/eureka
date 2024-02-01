use crate::asset::AssetServer;
use crate::core::engine::Engine;
use crate::render::RenderServer;
use crate::text::TextServer;
use crate::window::InputServer;

pub struct Singletons<'a> {
    pub engine: Engine,
    pub render_server: RenderServer<'a>,
    pub input_server: InputServer,
    pub text_server: TextServer,
    pub asset_server: AssetServer,
}
