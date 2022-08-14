use cgmath::Point2;
use winit::dpi::PhysicalPosition;
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
    consumed: bool,
}

pub struct MouseScroll {
    pub(crate) delta: f32,
    consumed: bool,
}

pub struct MouseMotion {
    pub(crate) delta: (f32, f32),
    pub(crate) position: (f32, f32),
    consumed: bool,
}

pub struct InputServer {
    pub(crate) mouse_position: (f32, f32),
    input_events: Vec<InputEvent>,
}

impl InputServer {
    pub fn new() -> Self {
        Self {
            mouse_position: (0.0f32, 0.0),
            input_events: Vec::new(),
        }
    }

    /// Handle input events.
    pub fn create_input_event(&mut self, event: &DeviceEvent) -> InputEvent {
        // Convert to our own input event.
        let input_event = match event {
            DeviceEvent::Key(KeyboardInput {
                virtual_keycode: Some(key),
                state,
                ..
            }) => InputEvent::Key {
                0: Key {
                    key: *key,
                    pressed: *state == ElementState::Pressed,
                },
            },
            DeviceEvent::MouseWheel { delta, .. } => {
                let scroll = match delta {
                    // I'm assuming a line is about 100 pixels.
                    MouseScrollDelta::LineDelta(_, scroll) => scroll * 100.0,
                    MouseScrollDelta::PixelDelta(PhysicalPosition { y: scroll, .. }) => {
                        *scroll as f32
                    }
                };

                InputEvent::MouseScroll {
                    0: MouseScroll {
                        delta: scroll,
                        consumed: false,
                    },
                }
            }
            DeviceEvent::Button {
                button: button_id,
                state,
            } => InputEvent::MouseButton {
                0: MouseButton {
                    button: *button_id,
                    pressed: *state == ElementState::Pressed,
                    position: self.mouse_position,
                    consumed: false,
                },
            },
            DeviceEvent::MouseMotion { delta } => InputEvent::MouseMotion {
                0: MouseMotion {
                    delta: (delta.0 as f32, delta.1 as f32),
                    position: self.mouse_position,
                    consumed: false,
                },
            },
            _ => InputEvent::Invalid,
        };

        input_event
    }
}
