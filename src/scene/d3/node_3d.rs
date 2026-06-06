use crate::animation::property::{PropertyPath, PropertyProvider, PropertyValue};
use crate::math::transform::Transform3d;
use glam::{FloatExt, Quat, Vec3};

pub struct Node3d {
    pub transform: Transform3d,
    pub global_transform: Transform3d,
}

impl Default for Node3d {
    fn default() -> Self {
        Self {
            transform: Transform3d::default(),
            global_transform: Transform3d::default(),
        }
    }
}

pub trait AsNode3d {
    fn get_position(&self) -> Vec3;

    fn set_position(&mut self, position: Vec3);

    fn get_rotation(&self) -> Quat;

    fn set_rotation(&mut self, rotation: Quat);

    fn get_scale(&self) -> Vec3;

    fn set_scale(&mut self, scale: Vec3);

    fn get_transform(&self) -> Transform3d;

    fn get_global_transform(&self) -> Transform3d;

    fn set_global_transform(&mut self, transform: Transform3d);
}

impl PropertyProvider for Node3d {
    fn get_property(&self, path: &PropertyPath) -> Option<PropertyValue> {
        match path.path() {
            "transform.position" => Some(PropertyValue::Vec3(self.transform.position)),
            "transform.position.x" => Some(PropertyValue::Float(self.transform.position.x)),
            "transform.position.y" => Some(PropertyValue::Float(self.transform.position.y)),
            "transform.position.z" => Some(PropertyValue::Float(self.transform.position.z)),
            "transform.rotation" => Some(PropertyValue::Quat(self.transform.rotation)),
            "transform.scale" => Some(PropertyValue::Vec3(self.transform.scale)),
            "transform.scale.x" => Some(PropertyValue::Float(self.transform.scale.x)),
            "transform.scale.y" => Some(PropertyValue::Float(self.transform.scale.y)),
            "transform.scale.z" => Some(PropertyValue::Float(self.transform.scale.z)),
            _ => None,
        }
    }

    fn set_property(&mut self, path: &PropertyPath, value: PropertyValue, weight: f32) {
        match (path.path(), value) {
            ("transform.position", PropertyValue::Vec3(v)) => {
                self.transform.position = self.transform.position.lerp(v, weight);
            }
            ("transform.position.x", PropertyValue::Float(v)) => {
                self.transform.position.x = self.transform.position.x.lerp(v, weight);
            }
            ("transform.position.y", PropertyValue::Float(v)) => {
                self.transform.position.y = self.transform.position.y.lerp(v, weight);
            }
            ("transform.position.z", PropertyValue::Float(v)) => {
                self.transform.position.z = self.transform.position.z.lerp(v, weight);
            }
            ("transform.rotation", PropertyValue::Quat(q)) => {
                self.transform.rotation = self.transform.rotation.slerp(q, weight);
            }
            ("transform.scale", PropertyValue::Vec3(v)) => {
                self.transform.scale = self.transform.scale.lerp(v, weight);
            }
            ("transform.scale.x", PropertyValue::Float(v)) => {
                self.transform.scale.x = self.transform.scale.x.lerp(v, weight);
            }
            ("transform.scale.y", PropertyValue::Float(v)) => {
                self.transform.scale.y = self.transform.scale.y.lerp(v, weight);
            }
            ("transform.scale.z", PropertyValue::Float(v)) => {
                self.transform.scale.z = self.transform.scale.z.lerp(v, weight);
            }
            _ => {}
        }
    }
}

impl AsNode3d for Node3d {
    fn get_position(&self) -> Vec3 {
        self.transform.position
    }

    fn set_position(&mut self, position: Vec3) {
        self.transform.position = position;
    }

    fn get_rotation(&self) -> Quat {
        self.transform.rotation
    }

    fn set_rotation(&mut self, rotation: Quat) {
        self.transform.rotation = rotation;
    }

    fn get_scale(&self) -> Vec3 {
        self.transform.scale
    }

    fn set_scale(&mut self, scale: Vec3) {
        self.transform.scale = scale;
    }

    fn get_transform(&self) -> Transform3d {
        self.transform
    }

    fn get_global_transform(&self) -> Transform3d {
        self.global_transform
    }

    fn set_global_transform(&mut self, transform: Transform3d) {
        self.global_transform = transform;
    }
}
