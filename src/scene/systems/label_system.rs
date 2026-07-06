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

        if label.text_is_dirty
            || label.atlas.as_ref().map_or(true, |a| a.texture.is_none())
        {
            let atlas = singletons.font_server.get_atlas(
                label.text.as_str(),
                label.font_id.clone(),
                label.leading,
            );

            if atlas.texture.is_some() {
                label.text_is_dirty = false;
            }
            label.atlas = Some(atlas);
        }
    }
}
