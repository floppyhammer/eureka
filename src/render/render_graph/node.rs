use crate::render::render_backend::PreparedFrame;
use crate::render::render_graph::frame_context::FrameContext;
use crate::render::render_graph::resource::NodeResources;
use std::any::Any;

pub trait Node: Send + Sync + 'static {
    fn as_any_mut(&mut self) -> &mut dyn Any;

    /// 声明节点所需的资源声明（输入和输出，包含类型和规格）
    fn node_resources(&self, _prepared: &PreparedFrame) -> NodeResources {
        NodeResources::new()
    }

    fn run(&mut self, context: &mut FrameContext);
}
