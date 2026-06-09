//! Input translation layer for camera and visualization controls.

use std::collections::BTreeMap;

use glam::Vec2;

use crate::{
    app::{InputEvent, KeyCode, MouseButton, TouchPhase},
    camera::Camera,
};

const ZOOM_STEP: f32 = 1.1;
const CLICK_DRAG_THRESHOLD: f32 = 4.0;

/// Semantic actions emitted from raw platform input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputAction {
    TogglePause,
    IncreaseSpeed,
    DecreaseSpeed,
    ResetView,
    FitAll,
    SelectAddress(usize),
}

/// Tracks pointer/touch state and mutates the camera in response to input.
#[derive(Debug, Clone)]
pub struct InputHandler {
    pub is_dragging: bool,
    pub last_mouse_pos: Option<Vec2>,
    pub drag_start_pos: Option<Vec2>,
    pub active_mouse_button: Option<MouseButton>,
    pub active_touches: BTreeMap<u64, Vec2>,
    pub last_pinch_distance: Option<f32>,
}

impl Default for InputHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl InputHandler {
    pub fn new() -> Self {
        Self {
            is_dragging: false,
            last_mouse_pos: None,
            drag_start_pos: None,
            active_mouse_button: None,
            active_touches: BTreeMap::new(),
            last_pinch_distance: None,
        }
    }

    pub fn handle_event(&mut self, event: &InputEvent, camera: &mut Camera) -> Vec<InputAction> {
        match *event {
            InputEvent::MouseMove { position } => self.handle_mouse_move(position, camera),
            InputEvent::MouseDown { button, position } => self.handle_mouse_down(button, position),
            InputEvent::MouseUp { button, position } => {
                self.handle_mouse_up(button, position, camera)
            }
            InputEvent::Scroll { delta_y, position } => {
                self.handle_scroll(delta_y, position, camera);
                Vec::new()
            }
            InputEvent::KeyPress { key } => self.handle_key_press(key),
            InputEvent::Resize { width, height } => {
                camera.set_viewport_size(width, height);
                Vec::new()
            }
            InputEvent::Touch {
                id,
                phase,
                position,
            } => self.handle_touch(id, phase, position, camera),
        }
    }

    fn handle_mouse_move(&mut self, position: Vec2, camera: &mut Camera) -> Vec<InputAction> {
        if self.is_dragging {
            if let Some(last_pos) = self.last_mouse_pos {
                let delta = position - last_pos;
                camera.pan(-delta.x / camera.zoom_level, -delta.y / camera.zoom_level);
            }
        }

        self.last_mouse_pos = Some(position);
        Vec::new()
    }

    fn handle_mouse_down(&mut self, button: MouseButton, position: Vec2) -> Vec<InputAction> {
        self.last_mouse_pos = Some(position);

        if button == MouseButton::Left {
            self.is_dragging = true;
            self.drag_start_pos = Some(position);
            self.active_mouse_button = Some(button);
        }

        Vec::new()
    }

    fn handle_mouse_up(
        &mut self,
        button: MouseButton,
        position: Vec2,
        camera: &Camera,
    ) -> Vec<InputAction> {
        let mut actions = Vec::new();
        let was_dragging = self.is_dragging && self.active_mouse_button == Some(button);

        if button == MouseButton::Left {
            if was_dragging {
                if self
                    .drag_start_pos
                    .map(|start| start.distance(position) <= CLICK_DRAG_THRESHOLD)
                    .unwrap_or(false)
                {
                    if let Some(address) = camera.screen_to_core(position.x, position.y) {
                        actions.push(InputAction::SelectAddress(address));
                    }
                }
            }

            self.is_dragging = false;
            self.drag_start_pos = None;
            self.active_mouse_button = None;
        }

        self.last_mouse_pos = Some(position);
        actions
    }

    fn handle_scroll(&mut self, delta_y: f32, position: Vec2, camera: &mut Camera) {
        if delta_y == 0.0 {
            return;
        }

        let factor = if delta_y > 0.0 {
            ZOOM_STEP.powf(delta_y.abs())
        } else {
            ZOOM_STEP.powf(-delta_y.abs())
        };
        camera.zoom_at(factor, position.x, position.y);
    }

    fn handle_key_press(&mut self, key: KeyCode) -> Vec<InputAction> {
        match key {
            KeyCode::Space => vec![InputAction::TogglePause],
            KeyCode::Plus | KeyCode::Equals => vec![InputAction::IncreaseSpeed],
            KeyCode::Minus => vec![InputAction::DecreaseSpeed],
            KeyCode::R => vec![InputAction::ResetView],
            KeyCode::F => vec![InputAction::FitAll],
            KeyCode::Character(ch) => match ch.to_ascii_lowercase() {
                'r' => vec![InputAction::ResetView],
                'f' => vec![InputAction::FitAll],
                '+' => vec![InputAction::IncreaseSpeed],
                '-' => vec![InputAction::DecreaseSpeed],
                _ => Vec::new(),
            },
        }
    }

    fn handle_touch(
        &mut self,
        id: u64,
        phase: TouchPhase,
        position: Vec2,
        camera: &mut Camera,
    ) -> Vec<InputAction> {
        match phase {
            TouchPhase::Started => {
                self.active_touches.insert(id, position);
                if self.active_touches.len() == 1 {
                    self.last_mouse_pos = Some(position);
                }
                if self.active_touches.len() == 2 {
                    self.last_pinch_distance = self.pinch_distance();
                }
            }
            TouchPhase::Moved => {
                let previous = self.active_touches.insert(id, position);
                match self.active_touches.len() {
                    1 => {
                        if let Some(last_pos) = previous.or(self.last_mouse_pos) {
                            let delta = position - last_pos;
                            camera.pan(-delta.x / camera.zoom_level, -delta.y / camera.zoom_level);
                        }
                        self.last_mouse_pos = Some(position);
                    }
                    2 => {
                        if let Some(center) = self.pinch_center() {
                            if let (Some(previous_distance), Some(current_distance)) =
                                (self.last_pinch_distance, self.pinch_distance())
                            {
                                if previous_distance > 0.0 && current_distance > 0.0 {
                                    camera.zoom_at(
                                        current_distance / previous_distance,
                                        center.x,
                                        center.y,
                                    );
                                }
                            }
                        }
                        self.last_pinch_distance = self.pinch_distance();
                    }
                    _ => {}
                }
            }
            TouchPhase::Ended | TouchPhase::Cancelled => {
                self.active_touches.remove(&id);
                if self.active_touches.len() < 2 {
                    self.last_pinch_distance = None;
                }
                if self.active_touches.is_empty() {
                    self.last_mouse_pos = None;
                }
            }
        }

        Vec::new()
    }

    fn pinch_distance(&self) -> Option<f32> {
        let mut touches = self.active_touches.values();
        let first = *touches.next()?;
        let second = *touches.next()?;
        Some(first.distance(second))
    }

    fn pinch_center(&self) -> Option<Vec2> {
        let mut touches = self.active_touches.values();
        let first = *touches.next()?;
        let second = *touches.next()?;
        Some((first + second) * 0.5)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn click_selects_core_address() {
        let mut handler = InputHandler::new();
        let mut camera = Camera::new(10, 10, 8.0, 80.0, 80.0);

        handler.handle_event(
            &InputEvent::MouseDown {
                button: MouseButton::Left,
                position: Vec2::new(8.0, 8.0),
            },
            &mut camera,
        );
        let actions = handler.handle_event(
            &InputEvent::MouseUp {
                button: MouseButton::Left,
                position: Vec2::new(8.0, 8.0),
            },
            &mut camera,
        );

        assert_eq!(actions, vec![InputAction::SelectAddress(11)]);
    }

    #[test]
    fn drag_pans_camera() {
        let mut handler = InputHandler::new();
        let mut camera = Camera::new(10, 10, 8.0, 80.0, 80.0);

        handler.handle_event(
            &InputEvent::MouseDown {
                button: MouseButton::Left,
                position: Vec2::new(10.0, 10.0),
            },
            &mut camera,
        );
        handler.handle_event(
            &InputEvent::MouseMove {
                position: Vec2::new(20.0, 25.0),
            },
            &mut camera,
        );

        assert_eq!(camera.position, Vec2::new(-10.0, -15.0));
    }
}
