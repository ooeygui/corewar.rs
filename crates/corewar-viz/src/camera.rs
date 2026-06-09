//! Camera controls for navigating the rendered core grid.

use glam::{Mat4, Vec2, Vec3};

/// Camera state for panning and zooming across the core surface.
#[derive(Debug, Clone)]
pub struct Camera {
    pub position: Vec2,
    pub zoom_level: f32,
    pub viewport_size: Vec2,
    pub grid_width: u32,
    pub grid_height: u32,
    pub cell_size: f32,
}

impl Camera {
    pub fn new(
        grid_width: u32,
        grid_height: u32,
        cell_size: f32,
        viewport_width: f32,
        viewport_height: f32,
    ) -> Self {
        Self {
            position: Vec2::ZERO,
            zoom_level: 1.0,
            viewport_size: Vec2::new(viewport_width.max(1.0), viewport_height.max(1.0)),
            grid_width,
            grid_height,
            cell_size,
        }
    }

    pub fn pan(&mut self, dx: f32, dy: f32) {
        self.position += Vec2::new(dx, dy);
    }

    pub fn zoom(&mut self, factor: f32) {
        self.zoom_level = (self.zoom_level * factor).clamp(0.1, 32.0);
    }

    pub fn zoom_at(&mut self, factor: f32, screen_x: f32, screen_y: f32) {
        let cursor = Vec2::new(screen_x, screen_y);
        let world_before = self.position + cursor / self.zoom_level;
        self.zoom(factor);
        let world_after = self.position + cursor / self.zoom_level;
        self.position += world_before - world_after;
    }

    pub fn set_viewport_size(&mut self, viewport_width: f32, viewport_height: f32) {
        self.viewport_size = Vec2::new(viewport_width.max(1.0), viewport_height.max(1.0));
    }

    pub fn reset_view(&mut self) {
        self.position = Vec2::ZERO;
        self.zoom_level = 1.0;
    }

    pub fn fit_all(&mut self) {
        let world_width = (self.grid_width as f32 * self.cell_size).max(1.0);
        let world_height = (self.grid_height as f32 * self.cell_size).max(1.0);
        let zoom_x = self.viewport_size.x / world_width;
        let zoom_y = self.viewport_size.y / world_height;
        self.zoom_level = zoom_x.min(zoom_y).clamp(0.1, 32.0);

        let padded_world =
            self.viewport_size / self.zoom_level - Vec2::new(world_width, world_height);
        self.position = Vec2::new(
            -(padded_world.x.max(0.0) * 0.5),
            -(padded_world.y.max(0.0) * 0.5),
        );
    }

    pub fn screen_to_core(&self, x: f32, y: f32) -> Option<usize> {
        if x < 0.0 || y < 0.0 || x > self.viewport_size.x || y > self.viewport_size.y {
            return None;
        }

        let world = self.position + Vec2::new(x, y) / self.zoom_level;
        if world.x < 0.0 || world.y < 0.0 {
            return None;
        }

        let column = (world.x / self.cell_size).floor() as u32;
        let row = (world.y / self.cell_size).floor() as u32;

        if column >= self.grid_width || row >= self.grid_height {
            return None;
        }

        Some((row * self.grid_width + column) as usize)
    }

    pub fn view_matrix(&self) -> Mat4 {
        let projection = Mat4::orthographic_rh_gl(
            0.0,
            self.viewport_size.x,
            self.viewport_size.y,
            0.0,
            -1.0,
            1.0,
        );
        let translation =
            Mat4::from_translation(Vec3::new(-self.position.x, -self.position.y, 0.0));
        let zoom = Mat4::from_scale(Vec3::new(self.zoom_level, self.zoom_level, 1.0));

        projection * zoom * translation
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn screen_to_core_maps_cells() {
        let camera = Camera::new(10, 10, 8.0, 160.0, 160.0);
        assert_eq!(camera.screen_to_core(9.0, 9.0), Some(11));
        assert_eq!(camera.screen_to_core(-1.0, 9.0), None);
    }

    #[test]
    fn zoom_at_preserves_cursor_target() {
        let mut camera = Camera::new(10, 10, 8.0, 160.0, 160.0);
        let before = camera.screen_to_core(40.0, 40.0);
        camera.zoom_at(2.0, 40.0, 40.0);

        assert_eq!(camera.screen_to_core(40.0, 40.0), before);
    }

    #[test]
    fn fit_all_centers_small_core() {
        let mut camera = Camera::new(10, 10, 8.0, 160.0, 240.0);
        camera.fit_all();

        assert!(camera.zoom_level >= 2.0);
        assert!(camera.position.y < 0.0);
    }
}
