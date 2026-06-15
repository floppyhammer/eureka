use eureka::core::App;
use eureka::math::transform::Transform2d;
use eureka::scene::{
    ActiveCamera, Camera2dComponent, GlobalTransform, LabelComponent, Name, Transform2dComponent,
};
use glam::Vec2;

fn main() {
    let mut app = App::new();

    app.setup(|app| {
        let singletons = app.singletons.as_ref().unwrap();
        let world = &mut app.world;

        let font_path = singletons
            .asset_server
            .asset_dir
            .join("fonts/Arial Unicode MS Font.ttf")
            .into_os_string()
            .into_string()
            .unwrap();

        // 1. 2D 摄像机
        world.ecs.spawn((
            Name("UICamera".into()),
            Transform2dComponent(Transform2d::default()),
            GlobalTransform::default(),
            Camera2dComponent::default(),
            ActiveCamera,
        ));

        let mut text = "".to_string();
        text += "你好世界！\n"; // Chinese
        text += "こんにちは世界！\n"; // Japanese
        text += "مرحبا بالعالم!\n"; // Arabic
        text += "ওহে বিশ্ব!\n"; // Bengali
        text += "สวัสดีชาวโลก!\n"; // Thai
        text += "سلام دنیا!\n"; // Persian
        text += "नमस्ते दुनिया!\n"; // Hindi
        text += "Chào thế giới!\n"; // Vietnamese
        text += "שלום עולם!\n"; // Hebrew
        text += "ABCDEFG Hello!٠١٢مرحبا!你好\n"; // Mixed bi-directional languages.

        // 2. 各种文本标签
        let mut label1 = LabelComponent::new(&*text);
        label1.font_id = Some(font_path);

        world.ecs.spawn((
            Name("Label1".into()),
            label1,
            Transform2dComponent(Transform2d {
                position: Vec2::new(100.0, 100.0),
                ..Transform2d::default()
            }),
            GlobalTransform::default(),
        ));
    });

    app.run();
}
