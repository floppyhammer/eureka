use crate::render::render_graph::RenderContext;

pub trait Node: Send + Sync + 'static {
    fn prepare(&mut self, _context: &mut RenderContext) {}
    fn run(&mut self, context: &mut RenderContext);
}
