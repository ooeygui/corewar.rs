//! Color palette and heat utilities for warrior visualization.
//! Generates layered, dark-background-friendly colors for up to 256+ warriors.

use core::f32::consts::TAU;

const HUE_LAYER_SIZE: usize = 16;
const BACKGROUND: [f32; 4] = [0.05, 0.05, 0.08, 1.0];
const MIN_VISIBLE_LUMA: f32 = 0.28;
const BASE_LIGHTNESS: f32 = 0.74;
const BASE_CHROMA: f32 = 0.20;
const LIGHTNESS_LAYER_LIGHTNESS: f32 = 0.56;
const SATURATION_LAYER_CHROMA: f32 = 0.10;
const CANDIDATE_LIGHTNESS_BANDS: [f32; 6] = [0.50, 0.62, 0.68, 0.74, 0.80, 0.86];
const CANDIDATE_CHROMA_BANDS: [f32; 6] = [0.06, 0.10, 0.14, 0.18, 0.22, 0.26];
const CANDIDATE_HUE_OFFSETS: [f32; 6] = [
    0.0,
    1.0 / 64.0,
    1.0 / 32.0,
    3.0 / 64.0,
    1.0 / 16.0,
    3.0 / 32.0,
];

/// A color palette that generates distinct colors for many warriors.
#[derive(Debug, Clone)]
pub struct ColorPalette {
    colors: Vec<[f32; 4]>,
}

impl ColorPalette {
    /// Generate a palette with `n` layered, perceptually distinct colors.
    pub fn generate(n: usize) -> Self {
        let mut colors = Vec::with_capacity(n);

        push_seed_layer(&mut colors, n, BASE_LIGHTNESS, BASE_CHROMA);
        push_seed_layer(&mut colors, n, LIGHTNESS_LAYER_LIGHTNESS, BASE_CHROMA);
        push_seed_layer(&mut colors, n, BASE_LIGHTNESS, SATURATION_LAYER_CHROMA);

        if colors.len() < n {
            fill_palette_greedily(&mut colors, n);
        }

        Self { colors }
    }

    /// Get the color for a warrior by ID.
    pub fn get(&self, warrior_id: u32) -> [f32; 4] {
        self.colors[warrior_id as usize % self.colors.len()]
    }

    /// Number of colors in the palette.
    pub fn len(&self) -> usize {
        self.colors.len()
    }

    pub fn is_empty(&self) -> bool {
        self.colors.is_empty()
    }
}

/// Per-cell heat data used to emphasize recently accessed addresses.
#[derive(Debug, Clone, PartialEq)]
pub struct HeatMap {
    values: Vec<f32>,
}

impl HeatMap {
    pub fn new(size: usize) -> Self {
        Self {
            values: vec![0.0; size],
        }
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    pub fn record_access(&mut self, address: usize) {
        if let Some(value) = self.values.get_mut(address) {
            *value = 1.0;
        }
    }

    pub fn decay(&mut self, factor: f32) {
        let factor = factor.clamp(0.0, 1.0);
        for value in &mut self.values {
            *value *= factor;
        }
    }

    pub fn get_heat(&self, address: usize) -> f32 {
        self.values.get(address).copied().unwrap_or(0.0)
    }
}

/// Blend a base cell color with recent-access heat.
pub fn blend_with_heat(base_color: [f32; 4], heat: f32) -> [f32; 4] {
    let heat = heat.clamp(0.0, 1.0);
    let glow_strength = 0.45 * heat;

    [
        lift_channel(base_color[0], glow_strength),
        lift_channel(base_color[1], glow_strength),
        lift_channel(base_color[2], glow_strength),
        (base_color[3] * (0.75 + 0.25 * heat)).clamp(0.0, 1.0),
    ]
}

/// Background used for unowned cells in the core.
pub fn background_color() -> [f32; 4] {
    BACKGROUND
}

/// Dim a base color while keeping it readable on a dark background.
pub fn dimmed_color(base: [f32; 4], factor: f32) -> [f32; 4] {
    let factor = factor.clamp(0.0, 1.0);

    [
        (base[0] * factor).clamp(0.0, 1.0),
        (base[1] * factor).clamp(0.0, 1.0),
        (base[2] * factor).clamp(0.0, 1.0),
        (base[3] * (0.35 + 0.65 * factor)).clamp(0.0, 1.0),
    ]
}

fn push_seed_layer(colors: &mut Vec<[f32; 4]>, target_len: usize, lightness: f32, chroma: f32) {
    for hue_index in 0..HUE_LAYER_SIZE {
        if colors.len() >= target_len {
            return;
        }

        colors.push(color_from_oklch(lightness, chroma, hue_for_slot(hue_index)));
    }
}

fn fill_palette_greedily(colors: &mut Vec<[f32; 4]>, target_len: usize) {
    let mut candidates = Vec::with_capacity(
        HUE_LAYER_SIZE
            * CANDIDATE_LIGHTNESS_BANDS.len()
            * CANDIDATE_CHROMA_BANDS.len()
            * CANDIDATE_HUE_OFFSETS.len(),
    );

    for hue_index in 0..HUE_LAYER_SIZE {
        let base_hue = hue_for_slot(hue_index);
        for lightness in CANDIDATE_LIGHTNESS_BANDS {
            for chroma in CANDIDATE_CHROMA_BANDS {
                for hue_offset in CANDIDATE_HUE_OFFSETS {
                    candidates.push(color_from_oklch(
                        lightness,
                        chroma,
                        (base_hue + hue_offset) % 1.0,
                    ));
                }
            }
        }
    }

    while colors.len() < target_len && !candidates.is_empty() {
        let (best_index, _) = candidates
            .iter()
            .enumerate()
            .map(|(index, candidate)| (index, min_distance_squared(*candidate, colors)))
            .max_by(|(_, left), (_, right)| left.total_cmp(right))
            .expect("candidate set should not be empty");

        colors.push(candidates.swap_remove(best_index));
    }

    let mut fallback_index = colors.len();
    while colors.len() < target_len {
        let hue = (hue_for_slot(fallback_index % HUE_LAYER_SIZE)
            + (fallback_index / HUE_LAYER_SIZE) as f32 / 64.0)
            % 1.0;
        let lightness = CANDIDATE_LIGHTNESS_BANDS[fallback_index % CANDIDATE_LIGHTNESS_BANDS.len()];
        let chroma = CANDIDATE_CHROMA_BANDS
            [(fallback_index / HUE_LAYER_SIZE) % CANDIDATE_CHROMA_BANDS.len()];
        colors.push(color_from_oklch(lightness, chroma, hue));
        fallback_index += 1;
    }
}

fn min_distance_squared(candidate: [f32; 4], colors: &[[f32; 4]]) -> f32 {
    colors
        .iter()
        .map(|color| {
            let dr = candidate[0] - color[0];
            let dg = candidate[1] - color[1];
            let db = candidate[2] - color[2];
            dr * dr + dg * dg + db * db
        })
        .fold(f32::INFINITY, f32::min)
}

fn color_from_oklch(lightness: f32, chroma: f32, hue: f32) -> [f32; 4] {
    let rgb = oklch_to_srgb(lightness, chroma, hue);
    [rgb[0], rgb[1], rgb[2], 1.0]
}

fn hue_for_slot(index: usize) -> f32 {
    radical_inverse_base2(index as u32)
}

fn radical_inverse_base2(mut value: u32) -> f32 {
    let mut result = 0.0;
    let mut scale = 0.5;

    while value != 0 {
        result += (value & 1) as f32 * scale;
        scale *= 0.5;
        value >>= 1;
    }

    result
}

fn lift_channel(channel: f32, glow_strength: f32) -> f32 {
    (channel + (1.0 - channel) * glow_strength).clamp(0.0, 1.0)
}

fn oklch_to_srgb(lightness: f32, chroma: f32, hue: f32) -> [f32; 3] {
    let angle = hue * TAU;
    let mut adjusted_chroma = chroma.max(0.0);
    let mut rgb = oklab_to_srgb(
        lightness,
        adjusted_chroma * angle.cos(),
        adjusted_chroma * angle.sin(),
    );

    while adjusted_chroma > 0.0 && !in_gamut(rgb) {
        adjusted_chroma *= 0.9;
        rgb = oklab_to_srgb(
            lightness,
            adjusted_chroma * angle.cos(),
            adjusted_chroma * angle.sin(),
        );
    }

    ensure_visible(clamp_rgb(rgb))
}

fn oklab_to_srgb(lightness: f32, a: f32, b: f32) -> [f32; 3] {
    let l = (lightness + 0.396_337_78 * a + 0.215_803_76 * b).powi(3);
    let m = (lightness - 0.105_561_346 * a - 0.063_854_17 * b).powi(3);
    let s = (lightness - 0.089_484_18 * a - 1.291_485_5 * b).powi(3);

    let linear = [
        4.076_741_7 * l - 3.307_711_6 * m + 0.230_969_94 * s,
        -1.268_438 * l + 2.609_757_4 * m - 0.341_319_38 * s,
        -0.004_196_086_3 * l - 0.703_418_6 * m + 1.707_614_7 * s,
    ];

    linear.map(linear_to_srgb)
}

fn linear_to_srgb(value: f32) -> f32 {
    if value <= 0.003_130_8 {
        12.92 * value
    } else {
        1.055 * value.powf(1.0 / 2.4) - 0.055
    }
}

fn in_gamut(rgb: [f32; 3]) -> bool {
    rgb.iter().all(|channel| (0.0..=1.0).contains(channel))
}

fn clamp_rgb(rgb: [f32; 3]) -> [f32; 3] {
    rgb.map(|channel| channel.clamp(0.0, 1.0))
}

fn ensure_visible(rgb: [f32; 3]) -> [f32; 3] {
    let luma = 0.2126 * rgb[0] + 0.7152 * rgb[1] + 0.0722 * rgb[2];
    if luma >= MIN_VISIBLE_LUMA {
        return rgb;
    }

    let lift = ((MIN_VISIBLE_LUMA - luma) / MIN_VISIBLE_LUMA).clamp(0.0, 1.0) * 0.5;
    [
        lift_channel(rgb[0], lift),
        lift_channel(rgb[1], lift),
        lift_channel(rgb[2], lift),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    const MIN_RGB_DISTANCE: f32 = 0.08;

    #[test]
    fn test_palette_generates_distinct_colors() {
        let palette = ColorPalette::generate(256);
        assert_eq!(palette.len(), 256);

        let mut min_distance = f32::MAX;
        for i in 0..255 {
            for j in (i + 1)..256 {
                let distance = rgb_distance(palette.colors[i], palette.colors[j]);
                min_distance = min_distance.min(distance);
            }
        }

        assert!(
            min_distance > MIN_RGB_DISTANCE,
            "minimum RGB distance was {min_distance:.4}, expected > {MIN_RGB_DISTANCE:.4}"
        );
    }

    #[test]
    fn test_heat_decay_converges_to_zero() {
        let mut heat_map = HeatMap::new(8);
        heat_map.record_access(3);
        assert_eq!(heat_map.get_heat(3), 1.0);

        for _ in 0..128 {
            heat_map.decay(0.9);
        }

        assert!(heat_map.get_heat(3) < 0.00001);
        assert_eq!(heat_map.get_heat(99), 0.0);
    }

    #[test]
    fn test_color_blending_produces_valid_rgba_values() {
        let base = ColorPalette::generate(1).get(0);
        let heated = blend_with_heat(base, 1.0);
        let dimmed = dimmed_color(base, 0.3);
        let background = background_color();

        for color in [base, heated, dimmed, background] {
            for channel in color {
                assert!((0.0..=1.0).contains(&channel));
            }
        }

        assert!(rgb_distance(base, heated) > 0.0);
        assert!(heated[0] >= base[0]);
        assert!(heated[1] >= base[1]);
        assert!(heated[2] >= base[2]);
        assert!(dimmed[3] <= base[3]);
    }

    fn rgb_distance(left: [f32; 4], right: [f32; 4]) -> f32 {
        let dr = left[0] - right[0];
        let dg = left[1] - right[1];
        let db = left[2] - right[2];
        (dr * dr + dg * dg + db * db).sqrt()
    }
}
