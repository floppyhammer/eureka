use crate::asset::AssetServer;
use crate::core::time::Time;
use crate::render::RenderServer;
use crate::text::TextServer;
use crate::window::InputServer;

pub struct Singletons<'a> {
    pub time: Time,
    pub render_server: RenderServer<'a>,
    pub input_server: InputServer,
    pub text_server: TextServer,
    pub asset_server: AssetServer,
}
