use crate::render::render_graph::FrameContext;
use std::any::Any;

pub trait Node: Send + Sync + 'static {
    fn prepare(&mut self, _context: &mut FrameContext) {}
    fn run(&mut self, context: &mut FrameContext);
    fn as_any_mut(&mut self) -> &mut dyn Any;
}
