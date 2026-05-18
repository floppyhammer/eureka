use cgmath::{Deg, Quaternion, Rotation, Rotation3, Vector3};
use eureka::core::App;
use eureka::math::color::ColorU;
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
            (-10.0, 0.0, 0.0),
            cgmath::Deg(0.0),
            cgmath::Deg(0.0),
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
        // let mut light = PointLight::new();
        // light.color = ColorU::new(0, 255, 0, 255);
        // light.set_position(Vector3::new(5.0, 5.0, 0.0));
        // light.strength = 5.0;
        // world.add_node(Box::new(light), None);

        // Directional light.
        let mut light = DirectionalLight::new();
        light.color = ColorU::new(0, 255, 0, 255);
        light.strength = 5.0;
        light.transform.rotation = Quaternion::from_angle_x(Deg(-90.0f32));
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
        obj_model.set_position(Vector3::new(0.0, 2.0, 0.0));
        world.add_node(Box::new(obj_model), None);

        // Model 2.
        let mut obj_model2 = Model::load(
            &mut render_world.texture_cache,
            &mut render_world.mesh_render_resources.material_cache,
            &mut render_world.mesh_cache,
            &singletons.render_server,
            &singletons
                .asset_server
                .asset_dir
                .join("models/viking_room/viking_room.obj"),
        )
        .unwrap();
        obj_model2.set_position(Vector3::new(5.0, 1.0, 0.0));
        obj_model2.set_rotation(Quaternion::from_angle_z(Deg(180.0)));
        world.add_node(Box::new(obj_model2), None);

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
        obj_model3.set_scale(Vector3::new(5.0, 1.0, 5.0));
        world.add_node(Box::new(obj_model3), None);
    });

    app.run();
}
