use cgmath::Vector2;
use eureka::render::Texture;
// use eureka::scene::button::Button;
use eureka::scene::Sprite2d;
use eureka::scene::Camera2d;
use eureka::vector_image::VectorTexture;
use eureka::App;
use eureka::asset::AssetServer;
use eureka::asset::Image;

fn main() {
    let mut app = App::new();

    app.add_node(Box::new(Camera2d::default()), None);

    // let v_tex = VectorTexture::from_file(
    //     app.singletons
    //         .asset_server
    //         .asset_dir
    //         .join("svgs/features.svg"),
    //     &app.singletons.render_server,
    // );
    // let mut vec_sprite = Box::new(VectorSprite::new(&app.singletons.render_server));
    // vec_sprite.set_texture(v_tex);
    // app.add_node(vec_sprite, None);

    // let handle = app.singletons.asset_server.load::<Image>("images/happy-tree");
    // let img = handle.unwrap().read();

    let img_tex = Texture::load(
        &app.singletons.render_server.device,
        &app.singletons.render_server.queue,
        &mut app.render_world.texture_cache,
        app.singletons
            .asset_server
            .asset_dir
            .join("images/happy-tree.png"),
    )
    .unwrap();

    let sprite1 = Box::new(Sprite2d::new(&app.render_world.texture_cache, img_tex));
    app.add_node(sprite1, None);

    let mut sprite2 = Box::new(Sprite2d::new(&app.render_world.texture_cache, img_tex));
    sprite2.transform.position = Vector2::new(200f32, -200f32);
    app.add_node(sprite2, None);

    let mut sprite3 = Box::new(Sprite2d::new(&app.render_world.texture_cache, img_tex));
    sprite3.transform.position = Vector2::new(400f32, -400f32);
    app.add_node(sprite3, None);

    // let mut button = Box::new(Button::new(&app.singletons.render_server));
    // button.transform.position = Vector2::new(200.0, 200.0);
    // app.add_node(button, None);

    app.run();
}
