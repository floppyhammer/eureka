use crate::render::render_graph::resource::{NodeResources, ResourceId};
use crate::render::render_graph::FrameContext;
use std::any::Any;

pub trait Node: Send + Sync + 'static {
    fn as_any_mut(&mut self) -> &mut dyn Any;

    /// 声明节点所需的资源声明（输入和输出，包含类型和规格）
    fn node_resources(&self) -> NodeResources {
        NodeResources::new()
    }

    fn prepare(&mut self, _context: &mut FrameContext) {}

    fn run(&mut self, context: &mut FrameContext);
}
