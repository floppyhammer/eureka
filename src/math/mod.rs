pub mod aabb;
pub(crate) mod bvh;
pub mod color;
pub mod frustum;
pub mod transform;

use allsorts::pathfinder_geometry::rect::RectF;
use glam::Vec4;

pub fn rect_to_vec4(rect: RectF) -> Vec4 {
    Vec4::new(
        rect.min_x(),
        rect.min_y(),
        rect.lower_right().x(),
        rect.lower_right().y(),
    )
}
