use glam::Vec3;

#[derive(Debug, Copy, Clone, Default)]
pub struct Aabb {
    pub min: Vec3,
    pub max: Vec3,
}

impl Aabb {
    pub fn new(min: Vec3, max: Vec3) -> Self {
        Self { min, max }
    }

    pub fn from_points(points: &[Vec3]) -> Self {
        if points.is_empty() {
            return Self::default();
        }

        let mut min = points[0];
        let mut max = points[0];

        for &p in &points[1..] {
            min = min.min(p);
            max = max.max(p);
        }

        Self { min, max }
    }

    pub fn transform(&self, transform: &crate::math::transform::Transform3d) -> Self {
        let matrix = glam::Mat4::from_scale_rotation_translation(
            transform.scale,
            transform.rotation,
            transform.position,
        );

        let corners = [
            Vec3::new(self.min.x, self.min.y, self.min.z),
            Vec3::new(self.min.x, self.min.y, self.max.z),
            Vec3::new(self.min.x, self.max.y, self.min.z),
            Vec3::new(self.min.x, self.max.y, self.max.z),
            Vec3::new(self.max.x, self.min.y, self.min.z),
            Vec3::new(self.max.x, self.min.y, self.max.z),
            Vec3::new(self.max.x, self.max.y, self.min.z),
            Vec3::new(self.max.x, self.max.y, self.max.z),
        ];

        let mut new_min = Vec3::splat(f32::INFINITY);
        let mut new_max = Vec3::splat(f32::NEG_INFINITY);

        for &c in &corners {
            let transformed = matrix.transform_point3(c);
            new_min = new_min.min(transformed);
            new_max = new_max.max(transformed);
        }

        Self {
            min: new_min,
            max: new_max,
        }
    }

    pub fn union(&self, other: &Self) -> Self {
        Self {
            min: self.min.min(other.min),
            max: self.max.max(other.max),
        }
    }
}
