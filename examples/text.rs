use cgmath::{Point2, Vector2};
use eureka::scene::label::Label;
use eureka::scene::Camera2d;
use eureka::App;
use winit::event_loop::EventLoop;

fn main() {
    let mut app = App::new();

    app.add_node(Box::new(Camera2d::new()), None);

    let mut label_i18n = Box::new(Label::new(&app.singletons.render_server));
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
    label_i18n.set_text(text);

    app.add_node(label_i18n, None);

    app.run();
}
