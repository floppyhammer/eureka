use crate::render::render_graph::FrameContext;

pub trait Node: Send + Sync + 'static {
    fn prepare(&mut self, _context: &mut FrameContext) {}
    fn run(&mut self, context: &mut FrameContext);
}
