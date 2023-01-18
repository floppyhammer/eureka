use cgmath::Point2;
use winit::event_loop::EventLoop;
use eureka::App;
use eureka::resources::CubemapTexture;
use eureka::scene::{Camera3d, Light, Model, Sky};

fn main() {
    let mut event_loop = EventLoop::new();

    let mut app = App::new(&event_loop);

    let camera3d = Camera3d::new(
        (0.0, 0.0, 0.0),
        cgmath::Deg(-90.0),
        cgmath::Deg(0.0),
        &app.singletons.render_server,
    );
    app.add_node(Box::new(camera3d), None);

    let skybox_tex =
        CubemapTexture::load(&app.singletons.render_server, &app.singletons.asset_server.asset_dir.join("skybox.jpg")).unwrap();
    let sky = Box::new(Sky::new(&app.singletons.render_server, skybox_tex));
    app.add_node(sky, None);

    // Light.
    let light = Light::new(&app.singletons.render_server, &app.singletons.asset_server.asset_dir.join("light.png"));
    app.add_node(Box::new(light), None);

    // Model.
    let obj_model = Box::new(
        Model::load(&app.singletons.render_server, &app.singletons.asset_server.asset_dir.join("ferris/ferris3d_v1.0.obj")).unwrap(),
    );
    app.add_node(obj_model, None);

    app.run(&mut event_loop);
}
