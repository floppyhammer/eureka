use crate::animation::property::{PropertyPath, PropertyProvider, PropertyValue};
use crate::math::transform::Transform2d;
use glam::{FloatExt, Vec2};

pub struct NodeUi {
    pub transform: Transform2d,
    pub global_transform: Transform2d,

    pub size: Vec2,
}

impl Default for NodeUi {
    fn default() -> Self {
        Self {
            transform: Transform2d::default(),
            global_transform: Transform2d::default(),
            size: Vec2::new(128.0_f32, 128.0),
        }
    }
}

pub trait AsNodeUi {
    fn get_size(&self) -> Vec2;

    fn set_size(&mut self, size: Vec2);

    fn get_position(&self) -> Vec2;

    fn set_position(&mut self, position: Vec2);

    fn get_rotation(&self) -> f32;

    fn set_rotation(&mut self, rotation: f32);

    fn get_transform(&self) -> Transform2d;

    fn get_global_transform(&self) -> Transform2d;

    fn set_global_transform(&mut self, transform: Transform2d);
}

impl PropertyProvider for NodeUi {
    fn get_property(&self, path: &PropertyPath) -> Option<PropertyValue> {
        match path.path() {
            "transform.position" => Some(PropertyValue::Vec2(self.transform.position)),
            "transform.position.x" => Some(PropertyValue::Float(self.transform.position.x)),
            "transform.position.y" => Some(PropertyValue::Float(self.transform.position.y)),
            "transform.rotation" => Some(PropertyValue::Float(self.transform.rotation)),
            "size" => Some(PropertyValue::Vec2(self.size)),
            "size.x" => Some(PropertyValue::Float(self.size.x)),
            "size.y" => Some(PropertyValue::Float(self.size.y)),
            _ => None,
        }
    }

    fn set_property(&mut self, path: &PropertyPath, value: PropertyValue, weight: f32) {
        match (path.path(), value) {
            ("transform.position", PropertyValue::Vec2(v)) => {
                self.transform.position = self.transform.position.lerp(v, weight);
            }
            ("transform.position.x", PropertyValue::Float(v)) => {
                self.transform.position.x = self.transform.position.x.lerp(v, weight);
            }
            ("transform.position.y", PropertyValue::Float(v)) => {
                self.transform.position.y = self.transform.position.y.lerp(v, weight);
            }
            ("transform.rotation", PropertyValue::Float(v)) => {
                self.transform.rotation = self.transform.rotation.lerp(v, weight);
            }
            ("size", PropertyValue::Vec2(v)) => {
                self.size = self.size.lerp(v, weight);
            }
            ("size.x", PropertyValue::Float(v)) => {
                self.size.x = self.size.x.lerp(v, weight);
            }
            ("size.y", PropertyValue::Float(v)) => {
                self.size.y = self.size.y.lerp(v, weight);
            }
            _ => {}
        }
    }
}
