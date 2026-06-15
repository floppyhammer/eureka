use std::collections::HashMap;
use std::path::Path;

use crate::animation::curve::AnimationCurve;

#[derive(Debug, Clone)]
pub struct AnimationClip {
    pub name: String,
    pub duration: f32,
    pub loop_count: i32,
    pub curves: HashMap<String, AnimationCurve<f32>>,
}

impl AnimationClip {
    pub fn new(name: String) -> Self {
        Self {
            name,
            duration: 0.0,
            loop_count: -1,
            curves: HashMap::new(),
        }
    }

    pub fn with_duration(mut self, duration: f32) -> Self {
        self.duration = duration;
        self
    }

    pub fn with_loop_count(mut self, loop_count: i32) -> Self {
        self.loop_count = loop_count;
        self
    }

    pub fn add_curve(mut self, name: String, curve: AnimationCurve<f32>) -> Self {
        self.curves.insert(name, curve);
        self.duration = self
            .curves
            .values()
            .map(|c| c.duration())
            .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .unwrap_or(0.0);
        self
    }

    pub fn evaluate(&self, time: f32) -> HashMap<String, f32> {
        let mut values = HashMap::new();
        for (name, curve) in &self.curves {
            values.insert(name.clone(), curve.evaluate(time));
        }
        values
    }

    pub fn from_gltf(_animation: &gltf::Animation) -> Self {
        todo!("GLTF animation loading not yet implemented");
    }

    pub fn from_file(_path: &Path) -> Self {
        todo!("File loading not yet implemented");
    }
}
