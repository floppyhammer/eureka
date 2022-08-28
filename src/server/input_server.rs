use cgmath::Point2;
use std::fmt::{Debug, Formatter};
use winit::dpi::PhysicalPosition;
use winit::event::*;

#[derive(Debug)]
pub enum InputEvent {
    MouseButton(MouseButton),
    MouseMotion(MouseMotion),
    MouseScroll(MouseScroll),
    Key(Key),
    Invalid,
}

#[derive(Debug)]
pub struct Key {
    pub(crate) key: VirtualKeyCode,
    pub(crate) pressed: bool,
}

#[derive(Debug)]
pub struct MouseButton {
    pub(crate) button: winit::event::MouseButton,
    pub(crate) pressed: bool,
    pub(crate) position: (f32, f32),
    consumed: bool,
}

#[derive(Debug)]
pub struct MouseScroll {
    pub(crate) delta: f32,
    consumed: bool,
}

#[derive(Debug)]
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
    pub fn create_input_event(&mut self, event: &WindowEvent) -> InputEvent {
        // Convert to our own input event.
        let input_event = match event {
            WindowEvent::KeyboardInput {
                input:
                    KeyboardInput {
                        state,
                        virtual_keycode: Some(key),
                        ..
                    },
                ..
            } => InputEvent::Key {
                0: Key {
                    key: *key,
                    pressed: *state == ElementState::Pressed,
                },
            },
            WindowEvent::MouseWheel { delta, .. } => {
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
            WindowEvent::MouseInput { button, state, .. } => InputEvent::MouseButton {
                0: MouseButton {
                    button: *button,
                    pressed: *state == ElementState::Pressed,
                    position: self.mouse_position,
                    consumed: false,
                },
            },
            WindowEvent::CursorMoved { position, .. } => {
                let relative = (
                    position.x as f32 - self.mouse_position.0,
                    position.y as f32 - self.mouse_position.1,
                );

                //let inner_size = window.inner_size();

                // Move origin to bottom left.
                //let y_position = inner_size.height as f64 - position.y;
                // window.scale_factor()
                self.mouse_position = ((position.x) as f32, (position.y) as f32);

                InputEvent::MouseMotion {
                    0: MouseMotion {
                        delta: (relative.0 as f32, relative.1 as f32),
                        position: self.mouse_position,
                        consumed: false,
                    },
                }
            }
            _ => InputEvent::Invalid,
        };

        println!("Input event: {:?}", input_event);

        input_event
    }
}
