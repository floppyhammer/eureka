use cgmath::Point2;
use std::fmt::{Debug, Formatter};
use winit::dpi::{PhysicalPosition, Position};
use winit::event::*;
use winit::window::Window;

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
    pub(crate) input_events: Vec<InputEvent>,
    cursor_captured: bool,
    cursor_state_changed: bool,
}

impl InputServer {
    pub fn new() -> Self {
        Self {
            mouse_position: (0.0f32, 0.0),
            input_events: Vec::new(),
            cursor_captured: false,
            cursor_state_changed: false,
        }
    }

    /// Capture or release cursor.
    pub fn set_cursor_capture(&mut self, capture: bool) {
        self.cursor_captured = capture;

        self.cursor_state_changed = true;
    }

    /// We should be able to update some states even no input event happens.
    pub fn update(&mut self, window: &Window) {
        if self.cursor_state_changed {
            window.set_cursor_visible(!self.cursor_captured);

            self.cursor_state_changed = false;
        }
    }

    /// Handle input events.
    pub fn prepare_input_event(&mut self, window: &Window, event: &WindowEvent) {
        self.input_events.clear();

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

                if self.cursor_captured {
                    // Use PhysicalPosition, or use LogicalPosition divided by ScaleFactor.
                    window
                        .set_cursor_position(Position::new(PhysicalPosition::new(
                            self.mouse_position.0,
                            self.mouse_position.1,
                        )))
                        .expect("Setting cursor position failed!");
                } else {
                    //let inner_size = window.inner_size();

                    // Move origin to bottom left.
                    //let y_position = inner_size.height as f64 - position.y;
                    // window.scale_factor()
                    self.mouse_position = ((position.x) as f32, (position.y) as f32);
                }

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

        self.input_events.push(input_event);
    }
}
