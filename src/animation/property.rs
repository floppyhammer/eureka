use glam::{Quat, Vec2, Vec3, Vec4};
use hecs::Entity;
use std::any::Any;

#[derive(Debug, Clone)]
pub struct PropertyChange {
    pub target_entity: Entity,
    pub property_path: PropertyPath,
    pub value: PropertyValue,
    pub weight: f32,
}

#[derive(Debug, Clone)]
pub struct PropertyPath {
    path: String,
}

impl PropertyPath {
    pub fn parse(path: &str) -> Self {
        Self {
            path: path.to_string(),
        }
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn get_component(&self) -> (String, Option<usize>) {
        let parts: Vec<&str> = self.path.split('.').collect();
        if parts.len() >= 3 {
            let base_path = parts[0..parts.len() - 1].join(".");
            let component = match *parts.last().unwrap() {
                "x" | "r" => Some(0),
                "y" | "g" => Some(1),
                "z" | "b" => Some(2),
                "w" | "a" => Some(3),
                _ => None,
            };
            (base_path, component)
        } else {
            (self.path.clone(), None)
        }
    }
}

#[derive(Debug, Clone)]
pub enum PropertyValue {
    Float(f32),
    Vec2(Vec2),
    Vec3(Vec3),
    Vec4(Vec4),
    Quat(Quat),
}

impl PropertyValue {
    pub fn as_float(&self) -> Option<f32> {
        match self {
            PropertyValue::Float(v) => Some(*v),
            _ => None,
        }
    }

    pub fn as_vec2(&self) -> Option<Vec2> {
        match self {
            PropertyValue::Vec2(v) => Some(*v),
            _ => None,
        }
    }

    pub fn as_vec3(&self) -> Option<Vec3> {
        match self {
            PropertyValue::Vec3(v) => Some(*v),
            _ => None,
        }
    }

    pub fn as_vec4(&self) -> Option<Vec4> {
        match self {
            PropertyValue::Vec4(v) => Some(*v),
            _ => None,
        }
    }

    pub fn as_quat(&self) -> Option<Quat> {
        match self {
            PropertyValue::Quat(v) => Some(*v),
            _ => None,
        }
    }
}

pub trait PropertyProvider: Any + 'static {
    fn get_property(&self, path: &PropertyPath) -> Option<PropertyValue>;
    fn set_property(&mut self, path: &PropertyPath, value: PropertyValue, weight: f32);
}

impl dyn PropertyProvider {
    pub fn is<T: PropertyProvider>(&self) -> bool {
        self.as_any().is::<T>()
    }

    pub fn downcast_ref<T: PropertyProvider>(&self) -> Option<&T> {
        self.as_any().downcast_ref::<T>()
    }

    pub fn downcast_mut<T: PropertyProvider>(&mut self) -> Option<&mut T> {
        self.as_any_mut().downcast_mut::<T>()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
