pub mod color;
pub mod transform;

use allsorts::pathfinder_geometry::rect::RectF;
use cgmath::Vector4;
use color::*;
use transform::*;

pub fn rect_to_vector4(rect: RectF) -> Vector4<f32> {
    Vector4::new(
        rect.min_x(),
        rect.min_y(),
        rect.lower_right().x(),
        rect.lower_right().y(),
    )
}
