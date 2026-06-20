use crate::core::singleton::Singletons;
use crate::scene::components::*;
use hecs::World;

pub fn update_labels(ecs: &mut World, singletons: &mut Singletons) {
    use crate::math::transform::Transform2d;
    use crate::scene::d2::label::LabelComponent;

    for (label, global) in ecs.query_mut::<(&mut LabelComponent, &GlobalTransform)>() {
        // 确保字体被请求
        if let Some(font_id) = &label.font_id {
            singletons.asset_server.request_font(font_id);
        }

        let (_, rotation, translation) = global.0.to_scale_rotation_translation();
        let rotation_z = rotation.to_euler(glam::EulerRot::XYZ).2;
        let current_global_transform = Transform2d {
            position: translation.truncate(),
            rotation: rotation_z,
            scale: glam::Vec2::ONE,
        };

        let transform_changed = (current_global_transform.position
            - label.last_global_transform.position)
            .length_squared()
            > 0.0001
            || (current_global_transform.rotation - label.last_global_transform.rotation).abs()
                > 0.0001;

        if label.text_is_dirty
            || label.atlas.as_ref().map_or(true, |a| a.texture.is_none())
            || transform_changed
        {
            let atlas = singletons.font_server.get_atlas(
                label.text.as_str(),
                label.font_id.clone(),
                current_global_transform,
                label.leading,
            );

            if atlas.texture.is_some() {
                label.text_is_dirty = false;
                label.last_global_transform = current_global_transform;
            }
            label.atlas = Some(atlas);
        }
    }
}
