use crate::{RenderServer, Singletons};
use crate::scene::CameraInfo;

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
        camera_info: &'b CameraInfo,
        singletons: &'b Singletons,
    ) {
        match &camera_info.bind_group {
            Some(b) => {
                render_pass.set_pipeline(&singletons.render_server.gizmo_pipeline);

                // Set camera group.
                render_pass.set_bind_group(0, b, &[]);

                render_pass.draw(0..4, 0..1);
            }
            None => {}
        }
    }
}
