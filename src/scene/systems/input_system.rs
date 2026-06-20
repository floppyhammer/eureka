use crate::window::InputServer;
use hecs::World;

pub fn handle_input(ecs: &mut World, input_server: &mut InputServer) {
    // 为了满足借用检查器，我们先克隆事件。
    // 由于每帧的输入事件数量极少，这里的性能开销可以忽略不计。
    let events: Vec<crate::window::InputEvent> = input_server.events().cloned().collect();

    for event in &events {
        if event.consumed {
            continue;
        }
        for controller in ecs.query_mut::<&mut crate::scene::d3::camera3d::Camera3dController>() {
            controller.handle_input(event, input_server);
        }
    }
}
