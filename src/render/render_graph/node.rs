use crate::render::render_graph::resource::ResourceId;
use crate::render::render_graph::FrameContext;
use std::any::Any;

pub trait Node: Send + Sync + 'static {
    fn as_any_mut(&mut self) -> &mut dyn Any;

    /// 声明节点的输入资源ID列表
    fn input_resources(&self) -> Vec<ResourceId<()>> {
        Vec::new()
    }

    /// 声明节点的输出资源ID列表
    fn output_resources(&self) -> Vec<ResourceId<()>> {
        Vec::new()
    }

    fn prepare(&mut self, _context: &mut FrameContext) {}

    fn run(&mut self, context: &mut FrameContext);
}
