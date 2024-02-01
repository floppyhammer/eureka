use cgmath::{Deg, Quaternion, Rotation, Rotation3, Vector3};
use eureka::core::App;
use eureka::render::Texture;
use eureka::scene::{AsNode3d, Camera3d, DirectionalLight, Model, PointLight, Sky};

fn main() {
    let mut app = App::new();

    let camera3d = Camera3d::new(
        (-10.0, 0.0, 0.0),
        cgmath::Deg(0.0),
        cgmath::Deg(0.0),
        &app.singletons.render_server,
    );
    app.add_node(camera3d, None);

    let skybox_tex = Texture::load_cube(
        &app.singletons.render_server,
        &mut app.render_world.texture_cache,
        &app.singletons
            .asset_server
            .asset_dir
            .join("images/skybox.jpg"),
    )
    .unwrap();
    let sky = Sky::new(skybox_tex);
    app.add_node(sky, None);

    // Light 1.
    let mut light = PointLight::new();
    light.transform.position = Vector3::new(4.0, 5.0, 0.0);
    light.strength = 5.0;
    app.add_node(light, None);
    //
    // // Light 2.
    // let mut light = PointLight::new();
    // light.transform.position = Vector3::new(-4.0, 5.0, 0.0);
    // light.strength = 2.0;
    // app.add_node(light, None);

    // Light 3.
    // let mut light = DirectionalLight::new();
    // // light.transform.rotation = Quaternion::from_angle_x(Deg(180.0));
    // light.strength = 2.0;
    // app.add_node(light, None);

    // Model1.
    let mut obj_model = Model::load(
        &mut app.render_world.texture_cache,
        &mut app.render_world.mesh_render_resources.material_cache,
        &mut app.render_world.mesh_cache,
        &app.singletons.render_server,
        &app.singletons
            .asset_server
            .asset_dir
            .join("models/ferris/ferris3d_v1.0.obj"),
    )
    .unwrap();
    obj_model.set_position(&Vector3::new(0.0, 2.0, 0.0));
    app.add_node(obj_model, None);

    // Model 2.
    let mut obj_model2 = Model::load(
        &mut app.render_world.texture_cache,
        &mut app.render_world.mesh_render_resources.material_cache,
        &mut app.render_world.mesh_cache,
        &app.singletons.render_server,
        &app.singletons
            .asset_server
            .asset_dir
            .join("models/viking_room/viking_room.obj"),
    )
    .unwrap();
    obj_model2.set_position(&Vector3::new(5.0, 1.0, 0.0));
    obj_model2.set_rotation(&Quaternion::from_angle_z(Deg(180.0)));
    app.add_node(obj_model2, None);

    // Model 3.
    let mut obj_model3 = Model::load(
        &mut app.render_world.texture_cache,
        &mut app.render_world.mesh_render_resources.material_cache,
        &mut app.render_world.mesh_cache,
        &app.singletons.render_server,
        &app.singletons
            .asset_server
            .asset_dir
            .join("models/granite_ground/granite_ground.obj"),
    )
    .unwrap();
    obj_model3.set_scale(&Vector3::new(5.0, 1.0, 5.0));
    app.add_node(obj_model3, None);

    app.run();
}
