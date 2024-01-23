pub mod color;
pub mod transform;

use allsorts::pathfinder_geometry::rect::RectF;
use cgmath::Vector4;

pub fn rect_to_vector4(rect: RectF) -> Vector4<f32> {
    Vector4::new(
        rect.min_x(),
        rect.min_y(),
        rect.lower_right().x(),
        rect.lower_right().y(),
    )
}

pub fn alignup_u32(a: u32, base: u32) -> u32 {
    return (a + base - 1) / base;
}
