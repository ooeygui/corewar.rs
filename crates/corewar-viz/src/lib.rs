//! # CoreWar Visualization
//!
//! WebGPU-based renderer for visualizing CoreWar battles.
//! Supports arbitrarily large core memories with distinct colors
//! for hundreds of warriors.

pub mod color;
pub mod renderer;

pub use color::ColorPalette;
