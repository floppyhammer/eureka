use winit::dpi::{PhysicalPosition};
use winit::event::*;
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{CursorGrabMode, Window};

#[derive(Debug, Copy, Clone)]
pub enum InputEvent {
    MouseButton(MouseButton),
    MouseMotion(MouseMotion),
    MouseScroll(MouseScroll),
    Key(Key),
    Invalid,
}

#[derive(Debug, Copy, Clone)]
pub struct Key {
    pub(crate) key_code: KeyCode,
    pub(crate) pressed: bool,
}

#[derive(Debug, Copy, Clone)]
pub struct MouseButton {
    pub(crate) button: winit::event::MouseButton,
    pub(crate) pressed: bool,
    pub(crate) position: (f32, f32),
    consumed: bool,
}

#[derive(Debug, Copy, Clone)]
pub struct MouseScroll {
    pub(crate) delta: f32,
    consumed: bool,
}

#[derive(Debug, Copy, Clone)]
pub struct MouseMotion {
    pub(crate) delta: (f32, f32),
    pub(crate) position: (f32, f32),
    consumed: bool,
}

pub struct InputServer {
    pub(crate) mouse_position: (f32, f32),
    pub(crate) input_events: Vec<InputEvent>,
    pub(crate) cursor_captured: bool,
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

    pub fn set_cursor_capture(&mut self, capture: bool) {
        if self.cursor_captured != capture {
            self.cursor_captured = capture;
            self.cursor_state_changed = true;
        }
    }

    pub fn update(&mut self, window: &Window) {
        if self.cursor_state_changed {
            window.set_cursor_visible(!self.cursor_captured);
            if self.cursor_captured {
                // 现代 winit 推荐使用 Locked 模式，这会自动处理原始增量
                let _ = window.set_cursor_grab(CursorGrabMode::Locked)
                    .or_else(|_| window.set_cursor_grab(CursorGrabMode::Confined));
            } else {
                let _ = window.set_cursor_grab(CursorGrabMode::None);
            }
            self.cursor_state_changed = false;
        }
    }

    /// 处理来自 DeviceEvent 的原始鼠标移动（不受窗口边界限制，无反馈环）
    pub fn handle_device_event(&mut self, event: &DeviceEvent) {
        if let DeviceEvent::MouseMotion { delta } = event {
            if self.cursor_captured {
                self.input_events.push(InputEvent::MouseMotion(MouseMotion {
                    delta: (delta.0 as f32, delta.1 as f32),
                    position: self.mouse_position,
                    consumed: false,
                }));
            }
        }
    }

    pub fn prepare_input_event(&mut self, _window: &Window, event: &WindowEvent) {
        let input_event = match event {
            WindowEvent::KeyboardInput { event, .. } => InputEvent::Key {
                0: Key {
                    key_code: match event.physical_key {
                        PhysicalKey::Code(code) => code,
                        _ => return,
                    },
                    pressed: event.state == ElementState::Pressed,
                },
            },
            WindowEvent::MouseWheel { delta, .. } => {
                let scroll = match delta {
                    MouseScrollDelta::LineDelta(_, scroll) => scroll * 100.0,
                    MouseScrollDelta::PixelDelta(PhysicalPosition { y: scroll, .. }) => *scroll as f32,
                };
                InputEvent::MouseScroll(MouseScroll { delta: scroll, consumed: false })
            }
            WindowEvent::MouseInput { button, state, .. } => InputEvent::MouseButton(MouseButton {
                button: *button,
                pressed: *state == ElementState::Pressed,
                position: self.mouse_position,
                consumed: false,
            }),
            WindowEvent::CursorMoved { position, .. } => {
                let last_pos = self.mouse_position;
                self.mouse_position = (position.x as f32, position.y as f32);

                // 如果光标未被捕获，则作为 UI 移动处理
                if !self.cursor_captured {
                    InputEvent::MouseMotion(MouseMotion {
                        delta: (self.mouse_position.0 - last_pos.0, self.mouse_position.1 - last_pos.1),
                        position: self.mouse_position,
                        consumed: false,
                    })
                } else {
                    InputEvent::Invalid
                }
            }
            _ => InputEvent::Invalid,
        };

        if !matches!(input_event, InputEvent::Invalid) {
            self.input_events.push(input_event);
        }
    }
}
