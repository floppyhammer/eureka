use glam::{Quat, Vec3};
use eureka::core::App;
use eureka::render::Texture;
use eureka::scene::{AsNode3d, Camera3d, Model, PointLight, DirectionalLight, Sky};

fn main() {
    let mut app = App::new();

    app.setup(|app| {
        // Destructure fields of App to leverage Rust's "split borrows" feature.
        let singletons = app.singletons.as_mut().unwrap();
        let render_world = app.render_world.as_mut().unwrap();
        let world = &mut app.world;

        let camera3d = Camera3d::new(
            Vec3::new(-10.0, 0.0, 0.0),
            0.0,
            0.0,
            &singletons.render_server,
        );
        world.add_node(Box::new(camera3d), None);

        // 异步加载天空盒
        let skybox_path = singletons.asset_server.asset_dir.join("images/skybox.jpg");
        let sky = Sky::at_path(skybox_path);
        world.add_node(Box::new(sky), None);

        // Point light.
        let mut light = PointLight::new();
        light.set_position(Vec3::new(2.0, 5.0, 0.0));
        light.strength = 5.0;
        world.add_node(Box::new(light), None);

        // Directional light.
        let mut light = DirectionalLight::new();
        light.strength = 0.5;
        light.transform.rotation = Quat::from_rotation_x(-90.0f32.to_radians());
        world.add_node(Box::new(light), None);

        // --- 现在的用法非常自然且丝滑 ---

        // 1. 直接创建“代理”模型
        let ferris_path = singletons.asset_server.asset_dir.join("models/ferris/ferris3d_v1.0.obj");
        let mut ferris = Model::at_path(ferris_path);

        // 2. 立刻设置你想要的任何变换，不需要等待
        ferris.set_position(Vec3::new(0.0, 1.0, 0.0));
        ferris.set_scale(Vec3::new(1.2, 1.2, 1.2));

        // 3. 直接加入场景
        world.add_node(Box::new(ferris), None);

        // 地面模型同理
        let ground_path = singletons.asset_server.asset_dir.join("models/granite_ground/granite_ground.obj");
        let mut ground = Model::at_path(ground_path);
        ground.set_scale(Vec3::new(5.0, 1.0, 5.0));
        world.add_node(Box::new(ground), None);
    });

    app.run();
}
