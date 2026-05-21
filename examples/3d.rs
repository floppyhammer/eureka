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

        let skybox_tex = Texture::load_cube(
            &singletons.render_server,
            &mut render_world.texture_cache,
            &singletons.asset_server.asset_dir.join("images/skybox.jpg"),
        )
        .unwrap();
        let sky = Sky::new(skybox_tex);
        world.add_node(Box::new(sky), None);

        // Point light.
        let mut light = PointLight::new();
        // light.color = ColorU::new(0, 255, 0, 255);
        light.set_position(Vec3::new(2.0, 5.0, 0.0));
        light.strength = 5.0;
        world.add_node(Box::new(light), None);

        // Directional light.
        let mut light = DirectionalLight::new();
        // light.color = ColorU::new(255, 0, 0, 255);
        light.strength = 0.5;
        light.transform.rotation = Quat::from_rotation_x(-90.0f32.to_radians());
        world.add_node(Box::new(light), None);

        // Model 1.
        let mut obj_model = Model::load(
            &mut render_world.texture_cache,
            &mut render_world.mesh_render_resources.material_cache,
            &mut render_world.mesh_cache,
            &singletons.render_server,
            &singletons
                .asset_server
                .asset_dir
                .join("models/ferris/ferris3d_v1.0.obj"),
        )
        .unwrap();
        obj_model.set_position(Vec3::new(0.0, 1.0, 0.0));
        world.add_node(Box::new(obj_model), None);

        // Model 3.
        let mut obj_model3 = Model::load(
            &mut render_world.texture_cache,
            &mut render_world.mesh_render_resources.material_cache,
            &mut render_world.mesh_cache,
            &singletons.render_server,
            &singletons
                .asset_server
                .asset_dir
                .join("models/granite_ground/granite_ground.obj"),
        )
        .unwrap();
        obj_model3.set_scale(Vec3::new(5.0, 1.0, 5.0));
        world.add_node(Box::new(obj_model3), None);
    });

    app.run();
}
