use glam::{Quat, Vec2, Vec3};

#[derive(Debug, Copy, Clone)]
pub struct Transform2d {
    pub position: Vec2,
    pub rotation: f32,
    pub scale: Vec2,
}

impl Transform2d {
    pub fn default() -> Self {
        Self {
            position: Vec2::new(0.0, 0.0),
            rotation: 0.0,
            scale: Vec2::new(1.0, 1.0),
        }
    }

    pub fn transform_point(&self, point: &Vec2) -> Vec2 {
        let (sin, cos) = self.rotation.sin_cos();
        let m00 = cos * self.scale.x;
        let m01 = -sin * self.scale.y;
        let m10 = sin * self.scale.x;
        let m11 = cos * self.scale.y;

        let mut new_point = Vec2::ZERO;
        new_point.x = m00 * point.x + m01 * point.y;
        new_point.y = m10 * point.x + m11 * point.y;

        new_point + self.position
    }

    pub fn combine(&self, other: &Self) -> Self {
        Self {
            position: self.transform_point(&other.position),
            rotation: self.rotation + other.rotation,
            scale: self.scale * other.scale,
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Transform3d {
    pub position: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl Transform3d {
    pub fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        }
    }

    pub fn matrix(&self) -> glam::Mat4 {
        glam::Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.position)
    }

    pub fn combine(&self, other: &Self) -> Self {
        Self {
            position: self.rotation * (self.scale * other.position) + self.position,
            rotation: self.rotation * other.rotation,
            scale: self.scale * other.scale,
        }
    }
}
