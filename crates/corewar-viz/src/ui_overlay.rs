//! HUD state rendered above the core visualization.

use glam::Vec2;

use crate::camera::Camera;

/// Process count display information for a warrior.
#[derive(Debug, Clone, PartialEq)]
pub struct WarriorProcessInfo {
    pub warrior_id: u32,
    pub process_count: usize,
    pub color: [f32; 4],
}

/// Details for the currently selected core cell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectedCellInfo {
    pub address: usize,
    pub instruction: String,
    pub owner: Option<u32>,
}

/// Minimap state for large cores.
#[derive(Debug, Clone, PartialEq)]
pub struct MinimapState {
    pub visible: bool,
    pub core_dimensions: Vec2,
    pub viewport_origin: Vec2,
    pub viewport_size: Vec2,
}

impl Default for MinimapState {
    fn default() -> Self {
        Self {
            visible: false,
            core_dimensions: Vec2::ZERO,
            viewport_origin: Vec2::ZERO,
            viewport_size: Vec2::ZERO,
        }
    }
}

/// Overlay state consumed by the renderer's UI pass.
#[derive(Debug, Clone, Default)]
pub struct UiOverlay {
    pub cycle_count: u64,
    pub warriors_alive: usize,
    pub process_counts: Vec<WarriorProcessInfo>,
    pub selected_cell: Option<SelectedCellInfo>,
    pub minimap: MinimapState,
    pub speed_indicator: String,
}

impl UiOverlay {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_cycle_count(&mut self, cycle_count: u64) {
        self.cycle_count = cycle_count;
    }

    pub fn set_process_counts(&mut self, process_counts: Vec<WarriorProcessInfo>) {
        self.warriors_alive = process_counts
            .iter()
            .filter(|entry| entry.process_count > 0)
            .count();
        self.process_counts = process_counts;
    }

    pub fn set_selected_cell(&mut self, selected_cell: Option<SelectedCellInfo>) {
        self.selected_cell = selected_cell;
    }

    pub fn set_speed_indicator(&mut self, speed_indicator: impl Into<String>) {
        self.speed_indicator = speed_indicator.into();
    }

    pub fn update_minimap(&mut self, camera: &Camera) {
        let core_dimensions = Vec2::new(
            camera.grid_width as f32 * camera.cell_size,
            camera.grid_height as f32 * camera.cell_size,
        );
        let viewport_world_size = camera.viewport_size / camera.zoom_level;
        let world_min = camera.position.max(Vec2::ZERO);
        let world_max = (camera.position + viewport_world_size).min(core_dimensions);
        let viewport_origin = if core_dimensions.x > 0.0 && core_dimensions.y > 0.0 {
            Vec2::new(
                world_min.x / core_dimensions.x,
                world_min.y / core_dimensions.y,
            )
        } else {
            Vec2::ZERO
        };
        let viewport_size = if core_dimensions.x > 0.0 && core_dimensions.y > 0.0 {
            Vec2::new(
                ((world_max.x - world_min.x).max(0.0) / core_dimensions.x).clamp(0.0, 1.0),
                ((world_max.y - world_min.y).max(0.0) / core_dimensions.y).clamp(0.0, 1.0),
            )
        } else {
            Vec2::ZERO
        };

        self.minimap = MinimapState {
            visible: viewport_world_size.x < core_dimensions.x
                || viewport_world_size.y < core_dimensions.y,
            core_dimensions,
            viewport_origin,
            viewport_size,
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minimap_visibility_tracks_zoomed_view() {
        let mut overlay = UiOverlay::new();
        let mut camera = Camera::new(100, 100, 8.0, 160.0, 160.0);
        camera.zoom(2.0);

        overlay.update_minimap(&camera);

        assert!(overlay.minimap.visible);
        assert!(overlay.minimap.viewport_size.x < 1.0);
        assert!(overlay.minimap.viewport_size.y < 1.0);
    }
}
