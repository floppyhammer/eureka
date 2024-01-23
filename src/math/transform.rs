use cgmath::{Deg, InnerSpace, Point3, Quaternion, Rotation3, Vector2, Vector3, Zero};

#[derive(Copy, Clone)]
pub struct Transform2d {
    pub position: Vector2<f32>,
    pub rotation: f32,
    pub scale: Vector2<f32>,
}

impl Transform2d {
    pub fn default() -> Self {
        Self {
            position: Vector2::new(0.0, 0.0),
            rotation: 0.0,
            scale: Vector2::new(1.0, 1.0),
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Transform3d {
    pub position: Vector3<f32>,
    pub rotation: Quaternion<f32>,
    pub scale: Vector3<f32>,
}

impl Transform3d {
    pub fn default() -> Self {
        // Transform.
        let position = Vector3 {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        };
        let rotation = if position.is_zero() {
            // This is needed so an object at (0, 0, 0) won't get scaled to zero
            // as Quaternions can effect scale if they're not created correctly.
            Quaternion::from_axis_angle(Vector3::unit_z(), Deg(0.0))
        } else {
            Quaternion::from_axis_angle(position.normalize(), Deg(45.0))
        };
        let scale = Vector3 {
            x: 1.0,
            y: 1.0,
            z: 1.0,
        };

        Self {
            position,
            rotation,
            scale,
        }
    }
}
