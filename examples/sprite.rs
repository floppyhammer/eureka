use glam::Vec2;
use eureka::core::App;
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
        let world = &mut app.world;

        let mut camera = Camera2d::default();
        camera.transform.rotation = 35.0;
        world.add_node(Box::new(camera), None);

        // --- 现在的用法：异步加载，无需等待纹理创建 ---

        let tree_path = singletons.asset_server.asset_dir.join("images/happy-tree.png");
        let texture_path = singletons.asset_server.asset_dir.join("images/texture.jpg");

        // 1. 创建第一个精灵，设置自定义旋转逻辑
        let mut sprite1 = Sprite2d::at_path(tree_path.clone());
        sprite1.custom_update = Some(custom_update);
        world.add_node(Box::new(sprite1), None);

        // 2. 创建第二个精灵，直接设置位置。它会自动复用上面正在加载的纹理。
        let mut sprite2 = Sprite2d::at_path(tree_path);
        sprite2.set_position(Vec2::new(200f32, 200f32));
        world.add_node(Box::new(sprite2), None);

        // 3. 创建第三个精灵，使用不同的图片
        let mut sprite3 = Sprite2d::at_path(texture_path);
        sprite3.set_position(Vec2::new(400f32, 400f32));
        world.add_node(Box::new(sprite3), None);
    });

    app.run();
}
