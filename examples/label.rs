use eureka::core::App;
use eureka::scene::Camera2d;
use eureka::scene::Label;

fn main() {
    let mut app = App::new();

    app.setup(|app| {
        let singletons = app.singletons.as_mut().unwrap();
        let world = &mut app.world;

        world.add_node(Box::new(Camera2d::default()), None);

        let font_path = singletons
            .asset_server
            .asset_dir
            .join("fonts/Arial Unicode MS Font.ttf")
            .into_os_string()
            .into_string()
            .unwrap();

        singletons.text_server.load_font(
            &font_path,
            &mut singletons.asset_server,
        );

        let mut text = "".to_string();
        text += "🌤你好世界！\n"; // Chinese
        text += "こんにちは世界！\n"; // Japanese
        text += "مرحبا بالعالم!\n"; // Arabic
        text += "ওহে বিশ্ব!\n"; // Bengali
        text += "สวัสดีชาวโลก!\n"; // Thai
        text += "سلام دنیا!\n"; // Persian
        text += "नमस्ते दुनिया!\n"; // Hindi
        text += "Chào thế giới!\n"; // Vietnamese
        text += "שלום عולם!\n"; // Hebrew
        text += "ABCDEFG Hello!٠١٢مرحبا!你好\n"; // Mixed bi-directional languages.

        let mut label = Label::default();
        label.set_text(text);
        label.set_font(font_path);

        world.add_node(Box::new(label), None);
    });

    app.run();
}
