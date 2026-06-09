//! High-level visualization application state.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use corewar_protocol::{CellInfo, CycleEvent, WarriorInfo};
use glam::Vec2;

use crate::{
    color::ColorPalette,
    input::{InputAction, InputHandler},
    renderer::Renderer,
    state::{PlaybackSpeed, VizState},
    ui_overlay::{SelectedCellInfo, UiOverlay, WarriorProcessInfo},
};

/// Abstract mouse buttons used by the visualization frontends.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Middle,
    Right,
    Other(u16),
}

/// Normalized keyboard input used by the visualization app.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyCode {
    Space,
    Plus,
    Equals,
    Minus,
    R,
    F,
    Character(char),
}

/// Basic touch event phases for mobile frontends.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TouchPhase {
    Started,
    Moved,
    Ended,
    Cancelled,
}

/// Platform-agnostic input events for the visualization.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InputEvent {
    MouseMove {
        position: Vec2,
    },
    MouseDown {
        button: MouseButton,
        position: Vec2,
    },
    MouseUp {
        button: MouseButton,
        position: Vec2,
    },
    Scroll {
        delta_y: f32,
        position: Vec2,
    },
    KeyPress {
        key: KeyCode,
    },
    Resize {
        width: f32,
        height: f32,
    },
    Touch {
        id: u64,
        phase: TouchPhase,
        position: Vec2,
    },
}

#[derive(Debug, Clone, Default)]
struct CellDetails {
    instruction_summary: String,
}

/// Top-level visualization application orchestrating state, input, and rendering.
pub struct App<'window> {
    pub renderer: Renderer<'window>,
    pub viz_state: VizState,
    pub input_handler: InputHandler,
    pub ui_overlay: UiOverlay,
    pub is_paused: bool,
    pub playback_speed: PlaybackSpeed,
    pending_cycles: VecDeque<Vec<CycleEvent>>,
    cycle_accumulator: f32,
    selected_address: Option<usize>,
    cell_details: Vec<CellDetails>,
    process_counts: BTreeMap<u32, usize>,
}

impl<'window> App<'window> {
    pub fn new(
        renderer: Renderer<'window>,
        palette: ColorPalette,
        viewport_width: f32,
        viewport_height: f32,
    ) -> Self {
        let mut viz_state =
            VizState::new(&renderer.config, palette, viewport_width, viewport_height);
        viz_state.camera.fit_all();

        let cell_details = vec![CellDetails::default(); viz_state.core.len()];
        let mut app = Self {
            renderer,
            viz_state,
            input_handler: InputHandler::new(),
            ui_overlay: UiOverlay::new(),
            is_paused: false,
            playback_speed: PlaybackSpeed::default(),
            pending_cycles: VecDeque::new(),
            cycle_accumulator: 0.0,
            selected_address: None,
            cell_details,
            process_counts: BTreeMap::new(),
        };
        app.sync_renderer();
        app.refresh_overlay();
        app
    }

    pub fn queue_cycle_events<I>(&mut self, events: I)
    where
        I: IntoIterator<Item = Vec<CycleEvent>>,
    {
        self.pending_cycles.extend(events);
    }

    pub fn apply_snapshot(&mut self, cells: &[CellInfo]) {
        if self.cell_details.len() != self.viz_state.core.len() {
            self.cell_details = vec![CellDetails::default(); self.viz_state.core.len()];
        }

        for cell in &mut self.viz_state.core.cells {
            cell.owner = None;
            cell.heat = 0.0;
        }
        for details in &mut self.cell_details {
            details.instruction_summary.clear();
        }

        for cell in cells {
            if let Some(core_cell) = self.viz_state.core.cells.get_mut(cell.address) {
                core_cell.owner = cell.owner;
                core_cell.heat = 0.0;
            }
            if let Some(details) = self.cell_details.get_mut(cell.address) {
                details.instruction_summary = cell.instruction_summary.clone();
            }
        }

        self.sync_renderer();
        self.refresh_overlay();
    }

    pub fn set_warriors(&mut self, warriors: &[WarriorInfo]) {
        self.process_counts.clear();
        for warrior in warriors {
            self.process_counts
                .insert(warrior.id, warrior.process_count);
        }
        self.refresh_overlay();
    }

    pub fn update(&mut self, dt: f32) {
        if !self.is_paused {
            let cycles_to_process = match self.playback_speed {
                PlaybackSpeed::Max => self.pending_cycles.len(),
                _ => {
                    self.cycle_accumulator += dt.max(0.0) * 60.0 * self.playback_speed.multiplier();
                    let cycles = self.cycle_accumulator.floor() as usize;
                    self.cycle_accumulator -= cycles as f32;
                    cycles
                }
            };

            for _ in 0..cycles_to_process {
                let Some(events) = self.pending_cycles.pop_front() else {
                    break;
                };
                self.apply_cycle_events(&events);
            }

            self.viz_state.decay_heat_with_delta(dt);
            self.sync_renderer();
        }

        self.refresh_overlay();
    }

    pub fn handle_event(&mut self, event: InputEvent) {
        let actions = self
            .input_handler
            .handle_event(&event, &mut self.viz_state.camera);
        for action in actions {
            self.apply_input_action(action);
        }
        self.refresh_overlay();
    }

    pub fn render(&mut self) {
        self.sync_renderer();
        self.renderer.render();
    }

    fn apply_input_action(&mut self, action: InputAction) {
        match action {
            InputAction::TogglePause => self.is_paused = !self.is_paused,
            InputAction::IncreaseSpeed => self.playback_speed = self.playback_speed.faster(),
            InputAction::DecreaseSpeed => self.playback_speed = self.playback_speed.slower(),
            InputAction::ResetView => self.viz_state.camera.reset_view(),
            InputAction::FitAll => self.viz_state.camera.fit_all(),
            InputAction::SelectAddress(address) => self.selected_address = Some(address),
        }
    }

    fn apply_cycle_events(&mut self, events: &[CycleEvent]) {
        for event in events {
            match *event {
                CycleEvent::ProcessCreated { warrior_id, .. } => {
                    *self.process_counts.entry(warrior_id).or_default() += 1;
                }
                CycleEvent::ProcessKilled { warrior_id, .. } => {
                    let count = self.process_counts.entry(warrior_id).or_default();
                    *count = count.saturating_sub(1);
                }
                _ => {}
            }
        }

        self.viz_state.apply_events(events);
    }

    fn refresh_overlay(&mut self) {
        self.ui_overlay
            .set_cycle_count(self.viz_state.core.current_cycle);
        self.ui_overlay
            .set_speed_indicator(self.playback_speed.label());
        self.ui_overlay
            .set_process_counts(self.process_overlay_entries());
        if self.ui_overlay.process_counts.is_empty() {
            self.ui_overlay.warriors_alive = self.infer_alive_warriors();
        }
        self.ui_overlay
            .set_selected_cell(self.selected_cell_info(self.selected_address));
        self.ui_overlay.update_minimap(&self.viz_state.camera);
    }

    fn process_overlay_entries(&self) -> Vec<WarriorProcessInfo> {
        self.process_counts
            .iter()
            .map(|(&warrior_id, &process_count)| WarriorProcessInfo {
                warrior_id,
                process_count,
                color: self.palette_color(warrior_id),
            })
            .collect()
    }

    fn selected_cell_info(&self, address: Option<usize>) -> Option<SelectedCellInfo> {
        let address = address?;
        let owner = self.viz_state.core.cells.get(address)?.owner;
        let instruction = self
            .cell_details
            .get(address)
            .map(|details| details.instruction_summary.as_str())
            .filter(|summary| !summary.is_empty())
            .unwrap_or("Unknown instruction")
            .to_owned();

        Some(SelectedCellInfo {
            address,
            instruction,
            owner,
        })
    }

    fn infer_alive_warriors(&self) -> usize {
        self.viz_state
            .core
            .cells
            .iter()
            .filter_map(|cell| cell.owner)
            .collect::<BTreeSet<_>>()
            .len()
    }

    fn palette_color(&self, warrior_id: u32) -> [f32; 4] {
        if self.viz_state.palette.is_empty() {
            [1.0, 1.0, 1.0, 1.0]
        } else {
            self.viz_state.palette.get(warrior_id)
        }
    }

    fn sync_renderer(&mut self) {
        self.renderer.set_core_state(self.viz_state.core.clone());
    }
}
