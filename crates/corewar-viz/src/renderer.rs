//! WebGPU renderer for core memory visualization.
//!
//! Renders the core as a 2D grid where each cell represents one memory address.
//! Colors indicate warrior ownership, brightness indicates recency of access.

/// Configuration for the renderer.
pub struct RendererConfig {
    /// How many cells wide the grid should be.
    pub grid_width: u32,
    /// Background color for unowned cells.
    pub background_color: [f32; 4],
    /// How quickly the heat map fades (0.0 = instant, 1.0 = permanent).
    pub heat_decay: f32,
}

impl Default for RendererConfig {
    fn default() -> Self {
        Self {
            grid_width: 100,
            background_color: [0.05, 0.05, 0.08, 1.0],
            heat_decay: 0.95,
        }
    }
}

/// The WebGPU renderer (placeholder - full implementation requires GPU context).
pub struct Renderer {
    pub config: RendererConfig,
    // TODO: wgpu::Device, Queue, Pipeline, Buffers
}

impl Renderer {
    pub fn new(config: RendererConfig) -> Self {
        Self { config }
    }
}
