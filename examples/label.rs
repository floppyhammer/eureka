use eureka::core::App;
use eureka::math::transform::Transform2d;
use eureka::scene::{
    ActiveCamera, CTransform2d, Camera2dComponent, GlobalTransform, LabelComponent, Name,
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
            CTransform2d(Transform2d::default()),
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
        let mut label1 = LabelComponent::new("Raster Text: Hello World! 你好世界！");
        label1.font_id = Some(font_path.clone());
        label1.mode = eureka::scene::d2::label::TextMode::Raster;

        world.ecs.spawn((
            Name("LabelRaster".into()),
            label1,
            CTransform2d(Transform2d {
                position: Vec2::new(100.0, 100.0),
                ..Transform2d::default()
            }),
            GlobalTransform::default(),
        ));

        // 模拟一个 MSDF 字体请求 (在文件名中包含 msdf 以触发 MSDF 模式)
        // 注意：这只是为了演示并行架构，实际效果取决于是否有真正的 MSDF 生成逻辑
        let mut label2 = LabelComponent::new("MSDF Text: Hello World! 你好世界！");
        label2.font_id = Some(font_path.replace(".ttf", "_msdf.ttf"));
        label2.mode = eureka::scene::d2::label::TextMode::Msdf;

        world.ecs.spawn((
            Name("LabelMsdf".into()),
            label2,
            CTransform2d(Transform2d {
                position: Vec2::new(100.0, 300.0),
                ..Transform2d::default()
            }),
            GlobalTransform::default(),
        ));
    });

    app.add_update(|app, _dt| {
        let mut query = app.world.ecs.query::<(&Name, &mut CTransform2d)>();
        let time = app.singletons.as_ref().unwrap().time.get_elapsed() as f32;

        for (name, transform) in query.iter() {
            if name.0 == "LabelMsdf" || name.0 == "LabelRaster" {
                // 让文字在 0.5 到 6.0 倍之间平滑缩放
                let scale_val = 3.25 + (time * 1.0).sin() * 2.75;
                transform.0.scale = Vec2::splat(scale_val);
            }
        }
    });

    app.run();
}
