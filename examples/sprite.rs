use glam::Vec2;
use eureka::core::App;
use eureka::render::Texture;
use eureka::scene::Sprite2d;
use eureka::scene::{Camera2d};
use eureka::scene::AsNodeUi;

fn custom_update(dt: f32, sprite: &mut Sprite2d) {
    sprite.set_rotation(sprite.get_rotation() + dt);
}

fn main() {
    let mut app = App::new();

    app.setup(|app| {
        let singletons = app.singletons.as_mut().unwrap();
        let render_world = app.render_world.as_mut().unwrap();
        let world = &mut app.world;

        let mut camera = Camera2d::default();
        camera.transform.rotation = 35.0;
        world.add_node(Box::new(camera), None);

        let img_tex = Texture::load(
            &singletons.render_server.device,
            &singletons.render_server.queue,
            &mut render_world.texture_cache,
            singletons.asset_server.asset_dir.join("images/happy-tree.png"),
        )
        .unwrap();

        let img_tex2 = Texture::load(
            &singletons.render_server.device,
            &singletons.render_server.queue,
            &mut render_world.texture_cache,
            singletons.asset_server.asset_dir.join("images/texture.jpg"),
        )
        .unwrap();

        let mut sprite1 = Sprite2d::new(&render_world.texture_cache, img_tex);
        sprite1.custom_update = Some(custom_update);
        world.add_node(Box::new(sprite1), None);

        let mut sprite2 = Sprite2d::new(&render_world.texture_cache, img_tex);
        sprite2.set_position(Vec2::new(200f32, 200f32));
        world.add_node(Box::new(sprite2), None);

        let mut sprite3 = Sprite2d::new(&render_world.texture_cache, img_tex2);
        sprite3.set_position(Vec2::new(400f32, 400f32));
        world.add_node(Box::new(sprite3), None);
    });

    app.run();
}
