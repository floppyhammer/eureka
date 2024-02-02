use eureka::core::App;
use eureka::scene::Camera2d;
use eureka::scene::Label;

fn main() {
    let mut app = App::new();

    app.add_node(Camera2d::default(), None);

    let mut text = "".to_string();
    text += "🌤你好世界！\n"; // Chinese
    text += "こんにちは世界！\n"; // Japanese
    text += "مرحبا بالعالم!\n"; // Arabic
    text += "ওহে বিশ্ব!\n"; // Bengali
    text += "สวัสดีชาวโลก!\n"; // Thai
    text += "سلام دنیا!\n"; // Persian
    text += "नमस्ते दुनिया!\n"; // Hindi
    text += "Chào thế giới!\n"; // Vietnamese
    text += "שלום עולם!\n"; // Hebrew
    text += "ABCDEFG Hello!٠١٢مرحبا!你好\n"; // Mixed languages.

    let mut label = Label::default();
    label.set_text(text);

    app.add_node(label, None);

    app.run();
}
