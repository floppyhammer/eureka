use eureka::core::App;
use eureka::render::Texture;
use eureka::scene::{AsNode3d, Camera3d, DirectionalLight, Model, PointLight, Sky, Sprite2d};
use glam::{Quat, Vec3};

fn custom_update(dt: f32, model: &mut Model) {
    let rotation_delta = Quat::from_rotation_y(dt);

    let new_rotation = rotation_delta * model.get_rotation();

    model.set_rotation(new_rotation);
}

fn custom_update2(dt: f32, model: &mut Model) {
    static mut TIME_ELAPSED: f32 = 0.0;

    // 修改和读取必须在 unsafe 块中进行
    unsafe {
        TIME_ELAPSED += dt;

        let speed = 2.0;
        let amplitude = 0.5;
        let new_y = (TIME_ELAPSED * speed).sin() * amplitude;

        let mut current_pos = model.get_position();
        current_pos.y = new_y;
        model.set_position(current_pos);
    }
}

fn main() {
    let mut app = App::new();

    app.setup(|app| {
        // Destructure fields of App to leverage Rust's "split borrows" feature.
        let singletons = app.singletons.as_mut().unwrap();
        let world = &mut app.world;

        let camera3d = Camera3d::new(
            Vec3::new(-10.0, 0.0, 0.0),
            0.0,
            0.0,
            &singletons.render_server,
        );
        world.add_node(Box::new(camera3d), None);

        // Add a skybox
        let skybox_path = singletons.asset_server.asset_dir.join("images/skybox.jpg");
        let sky = Sky::at_path(skybox_path);
        world.add_node(Box::new(sky), None);

        // Add a point light
        let mut light = PointLight::new();
        light.set_position(Vec3::new(2.0, 5.0, 0.0));
        light.strength = 5.0;
        world.add_node(Box::new(light), None);

        // Add a directional light
        // let mut light = DirectionalLight::new();
        // light.strength = 1.5;
        // light.transform.rotation = Quat::from_rotation_x(-135.0f32.to_radians());
        // world.add_node(Box::new(light), None);

        // Add a crab
        let ferris_path = singletons
            .asset_server
            .asset_dir
            .join("models/ferris/ferris3d_v1.0.obj");
        let mut ferris = Model::at_path(ferris_path);
        ferris.set_position(Vec3::new(0.0, 0.1, 0.0));
        ferris.set_scale(Vec3::new(1.0, 1.0, 1.0));
        ferris.custom_update = Some(custom_update2);
        world.add_node(Box::new(ferris), None);

        // let cube = singletons
        //     .asset_server
        //     .asset_dir
        //     .join("models/cube/cube.obj");
        // let mut cube = Model::at_path(cube);
        // cube.set_position(Vec3::new(2.0, 1.2, 2.0));
        // cube.set_scale(Vec3::new(0.5, 0.5, 0.5));
        // cube.custom_update = Some(custom_update);
        // world.add_node(Box::new(cube), None);

        // Add ground
        let ground_path = singletons
            .asset_server
            .asset_dir
            .join("models/granite_ground/granite_ground.obj");
        let mut ground = Model::at_path(ground_path);
        ground.set_scale(Vec3::new(5.0, 1.0, 5.0));
        world.add_node(Box::new(ground), None);
    });

    app.run();
}
