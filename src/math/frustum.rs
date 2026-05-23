use crate::math::aabb::Aabb;
use glam::{Mat4, Vec3, Vec4};

#[derive(Debug, Copy, Clone)]
pub struct Plane {
    pub normal: Vec3,
    pub d: f32,
}

impl Plane {
    pub fn new(normal: Vec3, d: f32) -> Self {
        let length = normal.length();
        Self {
            normal: normal / length,
            d: d / length,
        }
    }

    pub fn dot_point(&self, point: Vec3) -> f32 {
        self.normal.dot(point) + self.d
    }
}

pub struct Frustum {
    pub planes: [Plane; 6],
}

impl Frustum {
    pub fn from_view_proj(matrix: Mat4) -> Self {
        let row = matrix.to_cols_array_2d();
        let row0 = Vec4::from_array([row[0][0], row[1][0], row[2][0], row[3][0]]);
        let row1 = Vec4::from_array([row[0][1], row[1][1], row[2][1], row[3][1]]);
        let row2 = Vec4::from_array([row[0][2], row[1][2], row[2][2], row[3][2]]);
        let row3 = Vec4::from_array([row[0][3], row[1][3], row[2][3], row[3][3]]);

        let planes = [
            // Left
            Plane::new((row3 + row0).truncate(), row3.w + row0.w),
            // Right
            Plane::new((row3 - row0).truncate(), row3.w - row0.w),
            // Bottom
            Plane::new((row3 + row1).truncate(), row3.w + row1.w),
            // Top
            Plane::new((row3 - row1).truncate(), row3.w - row1.w),
            // Near
            Plane::new(row2.truncate(), row2.w),
            // Far
            Plane::new((row3 - row2).truncate(), row3.w - row2.w),
        ];

        Self { planes }
    }

    pub fn intersects_aabb(&self, aabb: &Aabb) -> bool {
        for plane in &self.planes {
            let p = Vec3::new(
                if plane.normal.x > 0.0 {
                    aabb.max.x
                } else {
                    aabb.min.x
                },
                if plane.normal.y > 0.0 {
                    aabb.max.y
                } else {
                    aabb.min.y
                },
                if plane.normal.z > 0.0 {
                    aabb.max.z
                } else {
                    aabb.min.z
                },
            );

            if plane.dot_point(p) < 0.0 {
                return false;
            }
        }
        true
    }
}
