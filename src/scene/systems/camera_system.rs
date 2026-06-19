use crate::core::singleton::Singletons;
use crate::scene::components::*;
use crate::scene::{Camera2dComponent, Camera3dComponent};
use glam::UVec2;
use hecs::World;

pub fn update_cameras(ecs: &mut World, singletons: &Singletons) {
    let width = singletons.render_context.surface_config.width as f32;
    let height = singletons.render_context.surface_config.height as f32;

    if width <= 0.0 || height <= 0.0 {
        return;
    }

    // 同步 2D 摄像机投影
    for (_id, camera) in ecs.query_mut::<&mut Camera2dComponent>() {
        camera.viewport_size = UVec2::new(width as u32, height as u32);
    }

    // 同步 3D 摄像机视口
    for (_id, (camera, _global)) in ecs.query_mut::<(&mut Camera3dComponent, &GlobalTransform)>() {
        camera.viewport_size = UVec2::new(width as u32, height as u32);
        camera.frame_count = camera.frame_count.wrapping_add(1);
    }
}
