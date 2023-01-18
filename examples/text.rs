use cgmath::Point2;
use winit::event_loop::EventLoop;
use eureka::App;
use eureka::scene::Camera2d;
use eureka::scene::label::Label;

fn main() {
    let mut event_loop = EventLoop::new();

    let mut app = App::new(&event_loop);

    app.add_node(Box::new(Camera2d::new()), None);

    let mut label_i18n = Box::new(Label::new(&app.singletons.render_server));
    label_i18n.transform.position = Point2::new(0.0, 600.0);
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
    text += "ABCDEFG Hello!٠١٢مرحبا!你好\n"; // Mixed languages.
    label_i18n.set_text(text);

    app.add_node(label_i18n, None);

    app.run(&mut event_loop);
}
