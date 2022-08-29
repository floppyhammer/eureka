use crate::{RenderServer, Singletons};

pub(crate) struct Gizmo {
    pub(crate) color: [f32; 3],
}

impl Gizmo {
    pub(crate) fn new() -> Self {
        Self {
            color: [1.0, 1.0, 1.0],
        }
    }

    pub(crate) fn draw<'a, 'b: 'a>(
        &'b self,
        render_pass: &mut wgpu::RenderPass<'a>,
        render_server: &'b RenderServer,
        singletons: &'b Singletons,
    ) {
        render_pass.set_pipeline(&render_server.gizmo_pipeline);

        // Set camera group.
        render_pass.set_bind_group(0,
                                   &singletons.camera3d.as_ref().unwrap().bind_group,
                                   &[]);

        render_pass.draw(0..4, 0..1);
    }
}
