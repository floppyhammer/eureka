use crate::render::render_graph::{FrameContext, Node};

pub struct TextNode;

impl Node for TextNode {
    fn run(&mut self, _context: &mut FrameContext) {
        // 由于 TextServer 目前不在 RenderWorld 里，
        // 我们暂时通过 context.render_world 的某些字段来传递绘制。
        // 等待后续将 TextServer 彻底接入 RenderWorld。
    }
}
