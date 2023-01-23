use cgmath::Point2;
use eureka::resources::Texture;
use eureka::scene::label::Label;
use eureka::scene::sprite2d::Sprite2d;
use eureka::scene::{Camera2d, VectorSprite};
use eureka::App;
use winit::event_loop::EventLoop;
use eureka::servers::VectorTexture;

fn main() {
    let mut event_loop = EventLoop::new();

    let mut app = App::new(&event_loop);

    app.add_node(Box::new(Camera2d::new()), None);

    let v_tex = VectorTexture::from_file(app.singletons.asset_server.asset_dir.join("svgs/features.svg"), &app.singletons.render_server);

    let mut vec_sprite = Box::new(VectorSprite::new(&app.singletons.render_server));
    vec_sprite.set_texture(v_tex);

    app.add_node(vec_sprite, None);

    let sprite_tex = Texture::load(
        &app.singletons.render_server.device,
        &app.singletons.render_server.queue,
        app.singletons.asset_server.asset_dir.join("images/happy-tree.png"),
    )
        .unwrap();
    let sprite = Box::new(Sprite2d::new(&app.singletons.render_server, sprite_tex));
    app.add_node(sprite, None);

    app.run(&mut event_loop);
}
