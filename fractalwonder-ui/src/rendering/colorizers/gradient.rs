//! Color gradients with positioned stops and midpoints.

use super::color_space::{
    linear_rgb_to_oklab, linear_to_srgb, oklab_to_linear_rgb, srgb_to_linear,
};
use serde::{Deserialize, Serialize};

const LUT_SIZE: usize = 4096;

/// A color stop in the gradient.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ColorStop {
    pub position: f64,
    pub color: [u8; 3],
}

/// Color gradient with stops and midpoints.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Gradient {
    pub stops: Vec<ColorStop>,
    pub midpoints: Vec<f64>,
}

impl Gradient {
    /// Create a gradient from color stops with default midpoints (0.5).
    /// Stops are sorted by position. Requires at least one stop.
    pub fn new(mut stops: Vec<ColorStop>) -> Self {
        assert!(
            !stops.is_empty(),
            "Gradient must have at least one color stop"
        );
        stops.sort_by(|a, b| {
            a.position
                .partial_cmp(&b.position)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let midpoint_count = if stops.len() > 1 { stops.len() - 1 } else { 0 };
        Self {
            stops,
            midpoints: vec![0.5; midpoint_count],
        }
    }

    /// Generate a 4096-entry LUT using OKLAB interpolation.
    pub fn to_lut(&self) -> Vec<[u8; 3]> {
        if self.stops.is_empty() {
            return vec![[0, 0, 0]; LUT_SIZE];
        }
        if self.stops.len() == 1 {
            return vec![self.stops[0].color; LUT_SIZE];
        }

        // Convert stops to OKLAB
        let oklab_stops: Vec<(f64, (f64, f64, f64))> = self
            .stops
            .iter()
            .map(|stop| {
                let r = srgb_to_linear(stop.color[0] as f64 / 255.0);
                let g = srgb_to_linear(stop.color[1] as f64 / 255.0);
                let b = srgb_to_linear(stop.color[2] as f64 / 255.0);
                (stop.position, linear_rgb_to_oklab(r, g, b))
            })
            .collect();

        (0..LUT_SIZE)
            .map(|i| {
                let t = i as f64 / (LUT_SIZE - 1) as f64;
                self.sample_oklab(&oklab_stops, t)
            })
            .collect()
    }

    fn sample_oklab(&self, oklab_stops: &[(f64, (f64, f64, f64))], t: f64) -> [u8; 3] {
        debug_assert_eq!(
            self.midpoints.len(),
            self.stops.len().saturating_sub(1),
            "Midpoints length must equal stops length minus 1"
        );

        // Find segment
        let mut seg = 0;
        while seg < oklab_stops.len() - 1 && oklab_stops[seg + 1].0 < t {
            seg += 1;
        }
        if seg >= oklab_stops.len() - 1 {
            seg = oklab_stops.len() - 2;
        }

        let (pos0, (l0, a0, b0)) = oklab_stops[seg];
        let (pos1, (l1, a1, b1)) = oklab_stops[seg + 1];

        // Local t in segment
        let seg_t = if (pos1 - pos0).abs() < 1e-10 {
            0.0
        } else {
            ((t - pos0) / (pos1 - pos0)).clamp(0.0, 1.0)
        };

        // Apply midpoint bias
        let midpoint = self.midpoints.get(seg).copied().unwrap_or(0.5);
        let biased_t = apply_midpoint_bias(seg_t, midpoint);

        // Interpolate in OKLAB
        let l = l0 + biased_t * (l1 - l0);
        let a = a0 + biased_t * (a1 - a0);
        let b = b0 + biased_t * (b1 - b0);

        // Convert back to sRGB
        let (r, g, b) = oklab_to_linear_rgb(l, a, b);
        [
            (linear_to_srgb(r) * 255.0).round().clamp(0.0, 255.0) as u8,
            (linear_to_srgb(g) * 255.0).round().clamp(0.0, 255.0) as u8,
            (linear_to_srgb(b) * 255.0).round().clamp(0.0, 255.0) as u8,
        ]
    }
}

/// Apply midpoint bias to interpolation factor.
/// midpoint=0.5 is linear interpolation.
/// midpoint<0.5: blend center shifts left, making colors transition faster (brighter earlier)
/// midpoint>0.5: blend center shifts right, making colors transition slower (darker earlier)
fn apply_midpoint_bias(t: f64, midpoint: f64) -> f64 {
    if (midpoint - 0.5).abs() < 1e-10 {
        return t;
    }

    // Use log-based bias for smooth adjustment
    if t <= 0.0 {
        return 0.0;
    }
    if t >= 1.0 {
        return 1.0;
    }

    // Map midpoint to exponent: midpoint=0.5 -> exp=1, midpoint<0.5 -> exp>1, midpoint>0.5 -> exp<1
    let exp = (0.5_f64).ln() / midpoint.clamp(0.01, 0.99).ln();
    t.powf(exp)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gradient_two_stops_endpoints() {
        let gradient = Gradient::new(vec![
            ColorStop {
                position: 0.0,
                color: [0, 0, 0],
            },
            ColorStop {
                position: 1.0,
                color: [255, 255, 255],
            },
        ]);
        let lut = gradient.to_lut();
        assert_eq!(lut[0], [0, 0, 0]);
        assert_eq!(lut[4095], [255, 255, 255]);
    }

    #[test]
    fn gradient_midpoint_affects_distribution() {
        let gradient_normal = Gradient::new(vec![
            ColorStop {
                position: 0.0,
                color: [0, 0, 0],
            },
            ColorStop {
                position: 1.0,
                color: [255, 255, 255],
            },
        ]);

        let mut gradient_biased = Gradient::new(vec![
            ColorStop {
                position: 0.0,
                color: [0, 0, 0],
            },
            ColorStop {
                position: 1.0,
                color: [255, 255, 255],
            },
        ]);
        gradient_biased.midpoints[0] = 0.25; // Shift toward dark

        let lut_normal = gradient_normal.to_lut();
        let lut_biased = gradient_biased.to_lut();

        // At midpoint of LUT, biased should be brighter (midpoint shifted left)
        let mid = LUT_SIZE / 2;
        assert!(
            lut_biased[mid][0] > lut_normal[mid][0],
            "Biased gradient should be brighter at midpoint"
        );
    }

    #[test]
    fn gradient_three_stops() {
        let gradient = Gradient::new(vec![
            ColorStop {
                position: 0.0,
                color: [255, 0, 0],
            },
            ColorStop {
                position: 0.5,
                color: [0, 255, 0],
            },
            ColorStop {
                position: 1.0,
                color: [0, 0, 255],
            },
        ]);
        let lut = gradient.to_lut();

        // Check endpoints
        assert_eq!(lut[0], [255, 0, 0]);
        assert_eq!(lut[4095], [0, 0, 255]);

        // At midpoint, should be close to green
        let mid = LUT_SIZE / 2;
        assert!(
            lut[mid][1] > lut[mid][0],
            "Green should dominate at midpoint"
        );
        assert!(
            lut[mid][1] > lut[mid][2],
            "Green should dominate at midpoint"
        );
    }

    #[test]
    fn midpoint_bias_identity() {
        assert!((apply_midpoint_bias(0.0, 0.5) - 0.0).abs() < 0.001);
        assert!((apply_midpoint_bias(0.5, 0.5) - 0.5).abs() < 0.001);
        assert!((apply_midpoint_bias(1.0, 0.5) - 1.0).abs() < 0.001);
    }

    #[test]
    fn midpoint_bias_shifts() {
        // Midpoint < 0.5: output > input (brighter earlier)
        assert!(apply_midpoint_bias(0.5, 0.25) > 0.5);
        // Midpoint > 0.5: output < input (darker earlier)
        assert!(apply_midpoint_bias(0.5, 0.75) < 0.5);
    }
}
