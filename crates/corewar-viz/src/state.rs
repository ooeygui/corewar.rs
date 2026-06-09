//! Shared visualization state built on top of the renderer primitives.

use corewar_protocol::CycleEvent;

use crate::{
    camera::Camera,
    color::ColorPalette,
    renderer::{CoreState, RendererConfig},
};

/// Playback multiplier used by the visualization update loop.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PlaybackSpeed {
    #[default]
    X1,
    X2,
    X10,
    Max,
}

impl PlaybackSpeed {
    pub fn label(self) -> &'static str {
        match self {
            Self::X1 => "1x",
            Self::X2 => "2x",
            Self::X10 => "10x",
            Self::Max => "max",
        }
    }

    pub fn multiplier(self) -> f32 {
        match self {
            Self::X1 => 1.0,
            Self::X2 => 2.0,
            Self::X10 => 10.0,
            Self::Max => f32::INFINITY,
        }
    }

    pub fn faster(self) -> Self {
        match self {
            Self::X1 => Self::X2,
            Self::X2 => Self::X10,
            Self::X10 | Self::Max => Self::Max,
        }
    }

    pub fn slower(self) -> Self {
        match self {
            Self::Max => Self::X10,
            Self::X10 => Self::X2,
            Self::X2 | Self::X1 => Self::X1,
        }
    }

    pub fn cycle_budget(self, dt: f32, pending_cycles: usize) -> usize {
        match self {
            Self::Max => pending_cycles,
            _ if pending_cycles == 0 || dt <= 0.0 => 0,
            _ => (dt * 60.0 * self.multiplier()).floor().max(1.0) as usize,
        }
    }
}

/// Top-level visualization state for replaying cycle events into a renderable model.
#[derive(Debug, Clone)]
pub struct VizState {
    pub core: CoreState,
    pub camera: Camera,
    pub palette: ColorPalette,
    pub heat_decay: f32,
}

impl VizState {
    pub fn new(
        config: &RendererConfig,
        palette: ColorPalette,
        viewport_width: f32,
        viewport_height: f32,
    ) -> Self {
        Self {
            core: CoreState::new(config.grid_width, config.grid_height),
            camera: Camera::new(
                config.grid_width,
                config.grid_height,
                config.cell_size,
                viewport_width,
                viewport_height,
            ),
            palette,
            heat_decay: config.heat_decay,
        }
    }

    pub fn apply_events(&mut self, events: &[CycleEvent]) {
        self.core.apply_events(events);
    }

    pub fn decay_heat(&mut self) {
        self.core.decay_heat(self.heat_decay);
    }

    pub fn decay_heat_with_delta(&mut self, dt: f32) {
        if dt <= 0.0 {
            return;
        }

        let frames = dt * 60.0;
        self.core
            .decay_heat(self.heat_decay.clamp(0.0, 1.0).powf(frames));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn viz_state_applies_events_and_decay() {
        let config = RendererConfig::default();
        let mut state = VizState::new(&config, ColorPalette::generate(4), 800.0, 600.0);

        state.apply_events(&[CycleEvent::Execute {
            address: 0,
            warrior_id: 1,
        }]);
        state.decay_heat();

        assert_eq!(state.core.cells[0].owner, Some(1));
        assert!(state.core.cells[0].heat <= 1.0);
        assert_eq!(state.palette.len(), 4);
    }

    #[test]
    fn playback_speed_steps_in_both_directions() {
        assert_eq!(PlaybackSpeed::X1.faster(), PlaybackSpeed::X2);
        assert_eq!(PlaybackSpeed::X10.faster(), PlaybackSpeed::Max);
        assert_eq!(PlaybackSpeed::Max.slower(), PlaybackSpeed::X10);
        assert_eq!(PlaybackSpeed::X2.slower(), PlaybackSpeed::X1);
    }
}
