use std::collections::HashSet;
use winit::dpi::PhysicalPosition;
use winit::event::*;
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{CursorGrabMode, Window};

#[derive(Debug, Clone)]
pub struct InputEvent {
    pub content: InputContent,
    pub consumed: bool,
}

#[derive(Debug, Copy, Clone)]
pub enum InputContent {
    MouseButton(MouseButton),
    MouseMotion(MouseMotion),
    MouseScroll(MouseScroll),
    Key(Key),
}

#[derive(Debug, Copy, Clone)]
pub struct Key {
    pub key_code: KeyCode,
    pub pressed: bool,
}

#[derive(Debug, Copy, Clone)]
pub struct MouseButton {
    pub button: winit::event::MouseButton,
    pub pressed: bool,
    pub position: (f32, f32),
}

#[derive(Debug, Copy, Clone)]
pub struct MouseScroll {
    pub delta: f32,
}

#[derive(Debug, Copy, Clone)]
pub struct MouseMotion {
    pub delta: (f32, f32),
    pub position: (f32, f32),
}

pub struct InputServer {
    pub(crate) mouse_position: (f32, f32),
    pub(crate) input_events: Vec<InputEvent>,
    pub(crate) cursor_captured: bool,
    cursor_state_changed: bool,

    // 状态查询缓存
    pressed_keys: HashSet<KeyCode>,
    pressed_mouse_buttons: HashSet<winit::event::MouseButton>,
}

impl InputServer {
    pub fn new() -> Self {
        Self {
            mouse_position: (0.0f32, 0.0),
            input_events: Vec::new(),
            cursor_captured: false,
            cursor_state_changed: false,
            pressed_keys: HashSet::new(),
            pressed_mouse_buttons: HashSet::new(),
        }
    }

    pub fn is_key_down(&self, key: KeyCode) -> bool {
        self.pressed_keys.contains(&key)
    }

    pub fn is_mouse_button_down(&self, button: winit::event::MouseButton) -> bool {
        self.pressed_mouse_buttons.contains(&button)
    }

    pub fn get_mouse_position(&self) -> (f32, f32) {
        self.mouse_position
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
                let _ = window
                    .set_cursor_grab(CursorGrabMode::Locked)
                    .or_else(|_| window.set_cursor_grab(CursorGrabMode::Confined));
            } else {
                let _ = window.set_cursor_grab(CursorGrabMode::None);
            }
            self.cursor_state_changed = false;
        }
    }

    pub fn clear_events(&mut self) {
        self.input_events.clear();
    }

    /// 处理来自 DeviceEvent 的原始鼠标移动（不受窗口边界限制，无反馈环）
    pub fn handle_device_event(&mut self, event: &DeviceEvent) {
        if let DeviceEvent::MouseMotion { delta } = event {
            if self.cursor_captured {
                self.input_events.push(InputEvent {
                    content: InputContent::MouseMotion(MouseMotion {
                        delta: (delta.0 as f32, delta.1 as f32),
                        position: self.mouse_position,
                    }),
                    consumed: false,
                });
            }
        }
    }

    pub fn prepare_input_event(&mut self, _window: &Window, event: &WindowEvent) {
        let content = match event {
            WindowEvent::KeyboardInput { event, .. } => {
                if let PhysicalKey::Code(code) = event.physical_key {
                    if event.state == ElementState::Pressed {
                        self.pressed_keys.insert(code);
                    } else {
                        self.pressed_keys.remove(&code);
                    }
                    Some(InputContent::Key(Key {
                        key_code: code,
                        pressed: event.state == ElementState::Pressed,
                    }))
                } else {
                    None
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let scroll = match delta {
                    MouseScrollDelta::LineDelta(_, scroll) => scroll * 100.0,
                    MouseScrollDelta::PixelDelta(PhysicalPosition { y: scroll, .. }) => {
                        *scroll as f32
                    }
                };
                Some(InputContent::MouseScroll(MouseScroll { delta: scroll }))
            }
            WindowEvent::MouseInput { button, state, .. } => {
                if *state == ElementState::Pressed {
                    self.pressed_mouse_buttons.insert(*button);
                } else {
                    self.pressed_mouse_buttons.remove(button);
                }
                Some(InputContent::MouseButton(MouseButton {
                    button: *button,
                    pressed: *state == ElementState::Pressed,
                    position: self.mouse_position,
                }))
            }
            WindowEvent::CursorMoved { position, .. } => {
                let last_pos = self.mouse_position;
                self.mouse_position = (position.x as f32, position.y as f32);

                if !self.cursor_captured {
                    Some(InputContent::MouseMotion(MouseMotion {
                        delta: (
                            self.mouse_position.0 - last_pos.0,
                            self.mouse_position.1 - last_pos.1,
                        ),
                        position: self.mouse_position,
                    }))
                } else {
                    None
                }
            }
            _ => None,
        };

        if let Some(content) = content {
            self.input_events.push(InputEvent {
                content,
                consumed: false,
            });
        }
    }

    pub fn events(&self) -> impl Iterator<Item = &InputEvent> {
        self.input_events.iter()
    }
}
