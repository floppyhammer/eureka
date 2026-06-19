use crate::scene::components::*;
use hecs::World;

pub fn update_example_logic(ecs: &mut World, dt: f32) {
    // 摄像机移动系统
    for (_id, (transform, controller)) in ecs.query_mut::<(
        &mut CTransform3d,
        &mut crate::scene::d3::camera3d::Camera3dController,
    )>() {
        transform.0.rotation =
            glam::Quat::from_euler(glam::EulerRot::ZYX, 0.0, controller.yaw, controller.pitch);

        let forward = transform.0.rotation * glam::Vec3::NEG_Z;
        let right = transform.0.rotation * glam::Vec3::X;

        transform.0.position += forward
            * (controller.amount_forward - controller.amount_backward)
            * controller.speed
            * dt;
        transform.0.position +=
            right * (controller.amount_right - controller.amount_left) * controller.speed * dt;
        transform.0.position.y +=
            (controller.amount_up - controller.amount_down) * controller.speed * dt;
    }
}
