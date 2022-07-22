use winit::event::*;

pub enum InputEvent {
    MouseButton(MouseButton),
    MouseMotion(MouseMotion),
    MouseScroll(MouseScroll),
    Key(Key),
    Invalid,
}

pub struct Key {
    pub(crate) key: VirtualKeyCode,
    pub(crate) pressed: bool,
}

pub struct MouseButton {
    pub(crate) button: u32,
    pub(crate) pressed: bool,
    pub(crate) position: (f32, f32),
}

impl MouseButton {
    pub fn new() -> MouseButton {
        MouseButton{
            button: 0,
            pressed: true,
            position: (0.0, 0.0),
        }
    }
}

pub struct MouseScroll {
    pub(crate) delta: f32,
}

pub struct MouseMotion {
    pub(crate) delta: (f32, f32),
    pub(crate) position: (f32, f32),
}
