use cgmath::{Point2, Vector2};
use eureka::render::Texture;
use eureka::scene::button::Button;
use eureka::scene::label::Label;
use eureka::scene::sprite2d::Sprite2d;
use eureka::scene::{Camera2d, VectorSprite};
use eureka::App;
use winit::event_loop::EventLoop;
use eureka::vector_image::VectorTexture;

fn main() {
    let mut event_loop = EventLoop::new();

    let mut app = App::new(&event_loop);

    app.add_node(Box::new(Camera2d::new()), None);

    let v_tex = VectorTexture::from_file(
        app.singletons
            .asset_server
            .asset_dir
            .join("svgs/features.svg"),
        &app.singletons.render_server,
    );
    let mut vec_sprite = Box::new(VectorSprite::new(&app.singletons.render_server));
    vec_sprite.set_texture(v_tex);
    app.add_node(vec_sprite, None);

    let img_tex = Texture::load(
        &app.singletons.render_server.device,
        &app.singletons.render_server.queue,
        app.singletons
            .asset_server
            .asset_dir
            .join("images/happy-tree.png"),
    )
        .unwrap();
    let sprite = Box::new(Sprite2d::new(&app.singletons.render_server, img_tex));
    app.add_node(sprite, None);

    let mut button = Box::new(Button::new(&app.singletons.render_server));
    button.transform.position = Vector2::new(200.0, 200.0);
    app.add_node(button, None);

    app.run(&mut event_loop);
}
