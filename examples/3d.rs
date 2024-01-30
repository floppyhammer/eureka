use cgmath::Vector3;
use eureka::render::Texture;
use eureka::scene::{Camera3d, Light, Model, Sky};
use eureka::App;

fn main() {
    let mut app = App::new();

    let camera3d = Camera3d::new(
        (0.0, 0.0, 0.0),
        cgmath::Deg(-90.0),
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
    let mut light = Light::new();
    light.transform.position = Vector3::new(0.0, 2.0, 0.0);
    app.add_node(Box::new(light), None);

    // Model.
    let obj_model = Box::new(
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
    app.add_node(obj_model, None);

    app.run();
}
