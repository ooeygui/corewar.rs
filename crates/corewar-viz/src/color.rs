//! Color palette generation for warrior visualization.
//! Generates perceptually distinct colors for up to 256+ warriors.

/// A color palette that generates distinct colors for many warriors.
pub struct ColorPalette {
    colors: Vec<[f32; 4]>,
}

impl ColorPalette {
    /// Generate a palette with `n` perceptually distinct colors.
    /// Uses golden ratio hue distribution in HSL space for maximum distinction.
    pub fn generate(n: usize) -> Self {
        let golden_ratio_conjugate = 0.618033988749895;
        let mut hue = 0.0_f32;
        let mut colors = Vec::with_capacity(n);

        for _ in 0..n {
            let (r, g, b) = hsl_to_rgb(hue, 0.7, 0.55);
            colors.push([r, g, b, 1.0]);
            hue = (hue + golden_ratio_conjugate) % 1.0;
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

/// Convert HSL to RGB (all values in 0.0..1.0 range).
fn hsl_to_rgb(h: f32, s: f32, l: f32) -> (f32, f32, f32) {
    if s == 0.0 {
        return (l, l, l);
    }

    let q = if l < 0.5 {
        l * (1.0 + s)
    } else {
        l + s - l * s
    };
    let p = 2.0 * l - q;

    let r = hue_to_rgb(p, q, h + 1.0 / 3.0);
    let g = hue_to_rgb(p, q, h);
    let b = hue_to_rgb(p, q, h - 1.0 / 3.0);

    (r, g, b)
}

fn hue_to_rgb(p: f32, q: f32, mut t: f32) -> f32 {
    if t < 0.0 {
        t += 1.0;
    }
    if t > 1.0 {
        t -= 1.0;
    }
    if t < 1.0 / 6.0 {
        return p + (q - p) * 6.0 * t;
    }
    if t < 1.0 / 2.0 {
        return q;
    }
    if t < 2.0 / 3.0 {
        return p + (q - p) * (2.0 / 3.0 - t) * 6.0;
    }
    p
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_palette_generates_distinct_colors() {
        let palette = ColorPalette::generate(256);
        assert_eq!(palette.len(), 256);

        // Verify all colors are distinct
        for i in 0..255 {
            for j in (i + 1)..256 {
                assert_ne!(
                    palette.get(i as u32),
                    palette.get(j as u32),
                    "Colors {} and {} should be distinct",
                    i,
                    j
                );
            }
        }
    }
}
