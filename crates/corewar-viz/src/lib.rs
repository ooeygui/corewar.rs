//! # CoreWar Visualization
//!
//! WebGPU-based renderer for visualizing CoreWar battles.
//! Supports arbitrarily large core memories with distinct colors
//! for hundreds of warriors.

pub mod app;
pub mod camera;
pub mod color;
pub mod error;
pub mod input;
#[cfg(not(target_arch = "wasm32"))]
pub mod network;
#[cfg(target_arch = "wasm32")]
pub mod network_wasm;
pub mod renderer;
pub mod state;
pub mod sync;
pub mod ui_overlay;
#[cfg(target_arch = "wasm32")]
pub mod web;

pub use app::{App, InputEvent, KeyCode, MouseButton, TouchPhase};
pub use camera::Camera;
pub use color::{background_color, blend_with_heat, dimmed_color, ColorPalette, HeatMap};
pub use error::NetworkError;
pub use input::{InputAction, InputHandler};
#[cfg(not(target_arch = "wasm32"))]
pub use network::NetworkClient;
#[cfg(target_arch = "wasm32")]
pub use network_wasm::NetworkClient;
pub use renderer::{CellState, CoreState, Renderer, RendererConfig};
pub use state::{PlaybackSpeed, VizState};
pub use sync::StateSynchronizer;
pub use ui_overlay::{MinimapState, SelectedCellInfo, UiOverlay, WarriorProcessInfo};
