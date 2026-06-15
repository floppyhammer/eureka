use std::collections::HashMap;
use std::sync::Arc;

use crate::animation::property::PropertyChange;
use crate::animation::{AnimationClip, PropertyPath, PropertyValue};
use hecs::Entity;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayState {
    Playing,
    Paused,
    Stopped,
}

#[derive(Debug, Clone)]
struct AnimationTrack {
    clip: Arc<AnimationClip>,
    time: f32,
    weight: f32,
}

#[derive(Debug, Clone)]
pub struct AnimationBinding {
    pub target_entity: Entity,
    pub property_path: PropertyPath,
    pub curve_name: String,
    pub weight: f32,
}

pub struct AnimationPlayer {
    clips: HashMap<String, Arc<AnimationClip>>,
    active_clip: Option<String>,
    tracks: Vec<AnimationTrack>,

    bindings: Vec<AnimationBinding>,

    play_state: PlayState,
    speed: f32,
    time: f32,
    loop_count: i32,

    blend_weight: f32,
    crossfade_duration: f32,
    crossfade_progress: f32,
    crossfading_to: Option<String>,

    pending_changes: Vec<PropertyChange>,
}

impl Default for AnimationPlayer {
    fn default() -> Self {
        Self {
            clips: HashMap::new(),
            active_clip: None,
            tracks: Vec::new(),
            bindings: Vec::new(),
            play_state: PlayState::Stopped,
            speed: 1.0,
            time: 0.0,
            loop_count: -1,
            blend_weight: 1.0,
            crossfade_duration: 0.2,
            crossfade_progress: 0.0,
            crossfading_to: None,
            pending_changes: Vec::new(),
        }
    }
}

impl AnimationPlayer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_clip(&mut self, clip: AnimationClip) {
        self.clips.insert(clip.name.clone(), Arc::new(clip));
    }

    pub fn play(&mut self, clip_name: &str, loop_count: i32) {
        if self.clips.contains_key(clip_name) {
            self.active_clip = Some(clip_name.to_string());
            self.time = 0.0;
            self.loop_count = loop_count;
            self.play_state = PlayState::Playing;
            self.tracks.clear();
            if let Some(clip) = self.clips.get(clip_name) {
                self.tracks.push(AnimationTrack {
                    clip: clip.clone(),
                    time: 0.0,
                    weight: 1.0,
                });
            }
        }
    }

    pub fn play_with_crossfade(&mut self, clip_name: &str, loop_count: i32, duration: f32) {
        if self.clips.contains_key(clip_name) {
            self.crossfade_duration = duration;
            self.crossfade_progress = 0.0;
            self.crossfading_to = Some(clip_name.to_string());
            self.loop_count = loop_count;
        }
    }

    pub fn pause(&mut self) {
        self.play_state = PlayState::Paused;
    }

    pub fn resume(&mut self) {
        self.play_state = PlayState::Playing;
    }

    pub fn stop(&mut self) {
        self.play_state = PlayState::Stopped;
        self.time = 0.0;
        self.tracks.clear();
        self.crossfading_to = None;
        self.crossfade_progress = 0.0;
    }

    pub fn set_speed(&mut self, speed: f32) {
        self.speed = speed;
    }

    pub fn bind_to(&mut self, entity_id: Entity, property: &str, curve_name: &str) {
        self.bindings.push(AnimationBinding {
            target_entity: entity_id,
            property_path: PropertyPath::parse(property),
            curve_name: curve_name.to_string(),
            weight: 1.0,
        });
    }

    pub fn unbind_from(&mut self, entity_id: Entity) {
        self.bindings.retain(|b| b.target_entity != entity_id);
    }

    pub fn set_binding_weight(&mut self, entity_id: Entity, weight: f32) {
        if let Some(binding) = self
            .bindings
            .iter_mut()
            .find(|b| b.target_entity == entity_id)
        {
            binding.weight = weight;
        }
    }

    fn update_crossfade(&mut self, dt: f32) {
        let target_clip_name = self.crossfading_to.clone();

        if let Some(target_clip_name) = target_clip_name {
            self.crossfade_progress += dt / self.crossfade_duration;

            if self.crossfade_progress >= 1.0 {
                let loop_count = self.loop_count;
                self.play(&target_clip_name, loop_count);
                self.crossfading_to = None;
                self.crossfade_progress = 0.0;
            } else if self.tracks.len() == 1 {
                if let Some(clip) = self.clips.get(&target_clip_name) {
                    self.tracks.push(AnimationTrack {
                        clip: clip.clone(),
                        time: 0.0,
                        weight: self.crossfade_progress,
                    });
                    self.tracks[0].weight = 1.0 - self.crossfade_progress;
                }
            } else if self.tracks.len() == 2 {
                self.tracks[0].weight = 1.0 - self.crossfade_progress;
                self.tracks[1].weight = self.crossfade_progress;
            }
        }
    }

    fn apply_animation(&mut self) {
        self.pending_changes.clear();

        for binding in &self.bindings {
            let mut final_value: Option<f32> = None;

            for track in &self.tracks {
                if let Some(value) = track.clip.curves.get(&binding.curve_name) {
                    let evaluated = value.evaluate(track.time);
                    if let Some(fv) = final_value {
                        final_value = Some(fv + evaluated * track.weight);
                    } else {
                        final_value = Some(evaluated * track.weight);
                    }
                }
            }

            if let Some(value) = final_value {
                self.pending_changes.push(PropertyChange {
                    target_entity: binding.target_entity,
                    property_path: binding.property_path.clone(),
                    value: PropertyValue::Float(value),
                    weight: binding.weight,
                });
            }
        }
    }

    pub fn take_changes(&mut self) -> Vec<PropertyChange> {
        std::mem::take(&mut self.pending_changes)
    }

    pub fn update(&mut self, dt: f32) {
        if self.play_state == PlayState::Playing {
            self.time += dt * self.speed;
            self.update_crossfade(dt);

            for track in &mut self.tracks {
                track.time += dt * self.speed;

                let duration = track.clip.duration;
                if duration > 0.0 {
                    if track.clip.loop_count == -1 {
                        track.time %= duration;
                    } else if track.time >= duration {
                        track.time = duration;
                        self.play_state = PlayState::Stopped;
                    }
                }
            }
        }

        self.apply_animation();
    }
}
