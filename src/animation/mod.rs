pub mod clip;
pub mod curve;
pub mod player;
pub mod property;

pub use clip::AnimationClip;
pub use curve::{AnimationCurve, Interpolation, Keyframe};
pub use player::AnimationPlayer;
pub use property::{PropertyPath, PropertyProvider, PropertyValue};