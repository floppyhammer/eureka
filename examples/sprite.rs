use eureka::core::App;
use eureka::scene::AsNodeUi;
use eureka::scene::Camera2d;
use eureka::scene::Sprite2d;
use glam::Vec2;

fn custom_update(dt: f32, sprite: &mut Sprite2d) {
    sprite.set_rotation(sprite.get_rotation() + dt);
}

fn main() {
    let mut app = App::new();

    app.setup(|app| {
        let singletons = app.singletons.as_mut().unwrap();
        let world = &mut app.world;

        // Add a 2D camera
        let camera = Camera2d::default();
        world.add_node(Box::new(camera), None);

        // Texture paths
        let tree_path = singletons
            .asset_server
            .asset_dir
            .join("images/happy-tree.png");
        let texture_path = singletons.asset_server.asset_dir.join("images/texture.jpg");

        // Add a sprite with texture
        let mut sprite1 = Sprite2d::at_path(tree_path.clone());
        sprite1.set_position(Vec2::new(200f32, 200f32));
        sprite1.custom_update = Some(custom_update);
        let sprite1_id = world.add_node(Box::new(sprite1), None);

        // Add another sprite with the same texture (only loaded once)
        let mut sprite2 = Sprite2d::at_path(tree_path);
        sprite2.set_position(Vec2::new(200f32, 200f32));
        world.add_node(Box::new(sprite2), Some(sprite1_id));

        // Add third sprite with a different texture
        let mut sprite3 = Sprite2d::at_path(texture_path);
        sprite3.set_position(Vec2::new(400f32, 400f32));
        world.add_node(Box::new(sprite3), None);
    });

    app.run();
}
