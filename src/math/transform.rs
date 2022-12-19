use cgmath::{Point2, Point3, Quaternion, Vector2, Vector3};

pub struct Transform2d {
    pub position: Point2<f32>,
    pub rotation: f32,
    pub scale: Vector2<f32>,
}

impl Transform2d {
    pub fn default() -> Self {
        Self {
            position: Point2::new(0.0, 0.0),
            rotation: 0.0,
            scale: Vector2::new(1.0, 1.0),
        }
    }
}

pub struct Transform3d {
    pub position: Point3<f32>,
    pub rotation: Quaternion<f32>,
    pub scale: Vector3<f32>,
}

impl Transform3d {
    pub fn default() -> Self {
        Self {
            position: Point3::new(0.0, 0.0, 0.0),
            rotation: Quaternion::new(0.0, 0.0, 0.0, 0.0),
            scale: Vector3::new(1.0, 1.0, 1.0),
        }
    }
}
