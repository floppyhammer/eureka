use crate::scene::components::*;
use hecs::World;

pub fn update_animations(ecs: &mut World, dt: f32) {
    use crate::animation::player::AnimationPlayer;
    use crate::animation::property::PropertyValue;
    use glam::FloatExt;

    let mut all_changes = Vec::new();

    // 收集所有动画播放器的变更
    for player in ecs.query_mut::<&mut AnimationPlayer>() {
        player.update(dt);
        all_changes.extend(player.take_changes());
    }

    // 应用变更
    for change in all_changes {
        let path = change.property_path.path();

        // 尝试应用到 3D Transform
        if let Ok(mut transform) = ecs.get::<&mut CTransform3d>(change.target_entity) {
            match (path, &change.value) {
                ("transform.position", PropertyValue::Vec3(v)) => {
                    transform.0.position = transform.0.position.lerp(*v, change.weight)
                }
                ("transform.position.x", PropertyValue::Float(v)) => {
                    transform.0.position.x = transform.0.position.x.lerp(*v, change.weight)
                }
                ("transform.position.y", PropertyValue::Float(v)) => {
                    transform.0.position.y = transform.0.position.y.lerp(*v, change.weight)
                }
                ("transform.position.z", PropertyValue::Float(v)) => {
                    transform.0.position.z = transform.0.position.z.lerp(*v, change.weight)
                }
                ("transform.rotation", PropertyValue::Quat(q)) => {
                    transform.0.rotation = transform.0.rotation.slerp(*q, change.weight)
                }
                ("transform.scale", PropertyValue::Vec3(v)) => {
                    transform.0.scale = transform.0.scale.lerp(*v, change.weight)
                }
                _ => {}
            }
        }

        // 尝试应用到 2D Transform
        if let Ok(mut transform) = ecs.get::<&mut CTransform2d>(change.target_entity) {
            match (path, &change.value) {
                ("transform.position", PropertyValue::Vec2(v)) => {
                    transform.0.position = transform.0.position.lerp(*v, change.weight)
                }
                ("transform.position.x", PropertyValue::Float(v)) => {
                    transform.0.position.x = transform.0.position.x.lerp(*v, change.weight)
                }
                ("transform.position.y", PropertyValue::Float(v)) => {
                    transform.0.position.y = transform.0.position.y.lerp(*v, change.weight)
                }
                ("transform.rotation", PropertyValue::Float(v)) => {
                    transform.0.rotation = transform.0.rotation.lerp(*v, change.weight)
                }
                ("transform.scale", PropertyValue::Vec2(v)) => {
                    transform.0.scale = transform.0.scale.lerp(*v, change.weight)
                }
                _ => {}
            }
        }
    }
}
