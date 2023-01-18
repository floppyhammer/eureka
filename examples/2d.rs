use cgmath::Point2;
use eureka::resources::Texture;
use eureka::scene::label::Label;
use eureka::scene::sprite2d::Sprite2d;
use eureka::scene::{Camera2d, VectorSprite};
use eureka::App;
use winit::event_loop::EventLoop;

fn main() {
    let mut event_loop = EventLoop::new();

    let mut app = App::new(&event_loop);

    app.add_node(Box::new(Camera2d::new()), None);

    let vec_sprite = Box::new(VectorSprite::new(&app.singletons.render_server));
    app.add_node(vec_sprite, None);

    let sprite_tex = Texture::load(
        &app.singletons.render_server.device,
        &app.singletons.render_server.queue,
        app.singletons.asset_server.asset_dir.join("happy-tree.png"),
    )
    .unwrap();
    let sprite = Box::new(Sprite2d::new(&app.singletons.render_server, sprite_tex));
    app.add_node(sprite, None);

    app.run(&mut event_loop);
}
