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

    pub fn from_vertices(vertices: &[crate::render::vertex::Vertex3d]) -> Self {
        if vertices.is_empty() {
            return Self::default();
        }

        let mut min = Vec3::from_array(vertices[0].position);
        let mut max = min;

        for v in &vertices[1..] {
            let p = Vec3::from_array(v.position);
            min = min.min(p);
            max = max.max(p);
        }

        Self { min, max }
    }

    pub fn transform(&self, transform: &crate::math::transform::Transform3d) -> Self {
        self.transform_by_matrix(&transform.matrix())
    }

    pub fn transform_by_matrix(&self, matrix: &glam::Mat4) -> Self {
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

    pub fn center(&self) -> Vec3 {
        (self.min + self.max) * 0.5
    }
}
