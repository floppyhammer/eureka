use glam::{Quat, Vec2, Vec3, Vec4};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Interpolation {
    Linear,
    Smooth,
    Step,
}

#[derive(Debug, Clone)]
pub struct Keyframe<T> {
    pub time: f32,
    pub value: T,
    pub interpolation: Interpolation,
}

impl<T> Keyframe<T> {
    pub fn new(time: f32, value: T) -> Self {
        Self {
            time,
            value,
            interpolation: Interpolation::Linear,
        }
    }

    pub fn with_interpolation(mut self, interpolation: Interpolation) -> Self {
        self.interpolation = interpolation;
        self
    }
}

#[derive(Debug, Clone)]
pub struct AnimationCurve<T> {
    pub keyframes: Vec<Keyframe<T>>,
}

impl<T> AnimationCurve<T>
where
    T: Interpolate,
{
    pub fn new(keyframes: Vec<Keyframe<T>>) -> Self {
        let mut curve = Self { keyframes };
        curve
            .keyframes
            .sort_by(|a, b| a.time.partial_cmp(&b.time).unwrap());
        curve
    }

    pub fn evaluate(&self, time: f32) -> T {
        if self.keyframes.is_empty() {
            return T::default();
        }

        if time <= self.keyframes[0].time {
            return self.keyframes[0].value.clone();
        }

        let last_idx = self.keyframes.len() - 1;
        if time >= self.keyframes[last_idx].time {
            return self.keyframes[last_idx].value.clone();
        }

        for i in 0..last_idx {
            let curr = &self.keyframes[i];
            let next = &self.keyframes[i + 1];

            if time >= curr.time && time <= next.time {
                let t = (time - curr.time) / (next.time - curr.time);

                return match curr.interpolation {
                    Interpolation::Linear => T::interpolate(&curr.value, &next.value, t),
                    Interpolation::Smooth => {
                        let prev = if i > 0 {
                            &self.keyframes[i - 1].value
                        } else {
                            &curr.value
                        };
                        let next_next = if i + 2 <= last_idx {
                            &self.keyframes[i + 2].value
                        } else {
                            &next.value
                        };
                        T::interpolate_smooth(prev, &curr.value, &next.value, next_next, t)
                    }
                    Interpolation::Step => curr.value.clone(),
                };
            }
        }

        self.keyframes[last_idx].value.clone()
    }

    pub fn duration(&self) -> f32 {
        if self.keyframes.is_empty() {
            0.0
        } else {
            self.keyframes.last().unwrap().time
        }
    }
}

pub trait Interpolate: Clone + Default {
    fn interpolate(a: &Self, b: &Self, t: f32) -> Self;
    fn interpolate_smooth(_prev: &Self, _a: &Self, _b: &Self, _next: &Self, _t: f32) -> Self {
        Self::interpolate(_a, _b, _t)
    }
}

macro_rules! impl_interpolate {
    ($type:ty) => {
        impl Interpolate for $type {
            fn interpolate(a: &Self, b: &Self, t: f32) -> Self {
                a + (b - a) * t
            }

            fn interpolate_smooth(prev: &Self, a: &Self, b: &Self, next: &Self, t: f32) -> Self {
                let t2 = t * t;
                let t3 = t2 * t;
                let m0 = (a - prev) * 0.5 + (b - a) * 0.5;
                let m1 = (b - a) * 0.5 + (next - b) * 0.5;
                (2.0 * t3 - 3.0 * t2 + 1.0) * a
                    + (t3 - 2.0 * t2 + t) * m0
                    + (-2.0 * t3 + 3.0 * t2) * b
                    + (t3 - t2) * m1
            }
        }
    };
}

impl_interpolate!(f32);
impl_interpolate!(Vec2);
impl_interpolate!(Vec3);
impl_interpolate!(Vec4);

impl Interpolate for Quat {
    fn interpolate(a: &Self, b: &Self, t: f32) -> Self {
        a.slerp(*b, t)
    }

    fn interpolate_smooth(prev: &Self, a: &Self, b: &Self, next: &Self, t: f32) -> Self {
        let t2 = t * t;
        let t3 = t2 * t;
        let m0 = a.slerp(*prev, 0.5).slerp(a.slerp(*b, 0.5), 0.5);
        let m1 = b.slerp(*a, 0.5).slerp(b.slerp(*next, 0.5), 0.5);
        let q0 = a.slerp(*b, t);
        let q1 = m0.slerp(m1, t);
        q0.slerp(q1, 3.0 * t2 - 2.0 * t3)
    }
}
