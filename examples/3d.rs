use cgmath::{Deg, Quaternion, Rotation, Rotation3, Vector3};
use eureka::render::Texture;
use eureka::scene::{Camera3d, PointLight, Model, Sky};
use eureka::App;

fn main() {
    let mut app = App::new();

    let camera3d = Camera3d::new(
        (-10.0, 0.0, 0.0),
        cgmath::Deg(0.0),
        cgmath::Deg(0.0),
        &app.singletons.render_server,
    );
    app.add_node(Box::new(camera3d), None);

    let skybox_tex = Texture::load_cube(
        &app.singletons.render_server,
        &mut app.render_world.texture_cache,
        &app.singletons
            .asset_server
            .asset_dir
            .join("images/skybox.jpg"),
    )
    .unwrap();
    let sky = Box::new(Sky::new(skybox_tex));
    app.add_node(sky, None);

    // Light.
    let mut light = PointLight::new();
    light.transform.position = Vector3::new(0.0, 5.0, 0.0);
    app.add_node(Box::new(light), None);

    // Model1.
    let mut obj_model = Box::new(
        Model::load(
            &mut app.render_world.texture_cache,
            &mut app.render_world.mesh_render_resources.material_cache,
            &mut app.render_world.mesh_cache,
            &app.singletons.render_server,
            &app.singletons
                .asset_server
                .asset_dir
                .join("models/ferris/ferris3d_v1.0.obj"),
        )
        .unwrap(),
    );
    obj_model.transform.position = Vector3::new(0.0, 0.0, 0.0);
    app.add_node(obj_model, None);

    // Model 2.
    let mut obj_model2 = Box::new(
        Model::load(
            &mut app.render_world.texture_cache,
            &mut app.render_world.mesh_render_resources.material_cache,
            &mut app.render_world.mesh_cache,
            &app.singletons.render_server,
            &app.singletons
                .asset_server
                .asset_dir
                .join("models/viking_room/viking_room.obj"),
        )
        .unwrap(),
    );
    obj_model2.transform.position = Vector3::new(5.0, 1.0, 0.0);
    obj_model2.transform.rotation = Quaternion::from_angle_z(Deg(180.0));
    app.add_node(obj_model2, None);

    // Model 3.
    let mut obj_model3 = Box::new(
        Model::load(
            &mut app.render_world.texture_cache,
            &mut app.render_world.mesh_render_resources.material_cache,
            &mut app.render_world.mesh_cache,
            &app.singletons.render_server,
            &app.singletons
                .asset_server
                .asset_dir
                .join("models/granite_ground/granite_ground.obj"),
        )
        .unwrap(),
    );
    obj_model3.transform.scale = Vector3::new(5.0, 1.0, 5.0);
    app.add_node(obj_model3, None);

    app.run();
}
