//! Color settings for the colorization pipeline.

use super::palettes;
use super::Palette;
use serde::{Deserialize, Serialize};

/// Default transfer bias (1.0 = linear, no bias).
pub const DEFAULT_TRANSFER_BIAS: f32 = 1.0;

/// Minimum transfer bias value.
pub const MIN_TRANSFER_BIAS: f32 = 0.1;

/// Maximum transfer bias value.
pub const MAX_TRANSFER_BIAS: f32 = 20.0;

/// Step size for transfer bias adjustment.
pub const TRANSFER_BIAS_STEP: f32 = 1.0;

/// Apply transfer bias to a normalized value in [0, 1].
/// - bias < 1.0: More colors near the set boundary (glow effect)
/// - bias = 1.0: Linear (no bias)
/// - bias > 1.0: More colors in outer regions
#[inline]
pub fn apply_transfer_bias(t: f64, bias: f32) -> f64 {
    t.clamp(0.0, 1.0).powf(bias as f64)
}

/// Settings for derivative-based Blinn-Phong lighting.
#[derive(Clone, Debug)]
pub struct ShadingSettings {
    pub enabled: bool,
    /// Light azimuth angle in radians (0 = right, π/2 = top)
    pub light_azimuth: f64,
    /// Light elevation angle in radians (0 = horizon, π/2 = overhead)
    pub light_elevation: f64,
    /// Ambient light level [0, 1]
    pub ambient: f64,
    /// Diffuse reflection strength [0, 1]
    pub diffuse: f64,
    /// Specular reflection strength [0, 1]
    pub specular: f64,
    /// Specular exponent (shininess)
    pub shininess: f64,
}

impl Default for ShadingSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            light_azimuth: std::f64::consts::FRAC_PI_4, // 45°
            light_elevation: std::f64::consts::FRAC_PI_4, // 45°
            ambient: 0.15,
            diffuse: 0.7,
            specular: 0.3,
            shininess: 32.0,
        }
    }
}

impl ShadingSettings {
    pub fn disabled() -> Self {
        Self::default()
    }

    pub fn enabled() -> Self {
        Self {
            enabled: true,
            ..Self::default()
        }
    }
}

/// User-configurable color options. Used directly by the colorizer.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ColorOptions {
    /// Palette ID (e.g., "classic", "fire").
    pub palette_id: String,
    /// Whether 3D slope shading is enabled.
    pub shading_enabled: bool,
    /// Whether smooth iteration coloring is enabled.
    pub smooth_enabled: bool,
    /// Whether histogram equalization is enabled.
    pub histogram_enabled: bool,
    /// Number of palette cycles (1 to 1024).
    pub cycle_count: u32,
    /// Transfer bias for color distribution (0.1 to 10.0).
    /// < 1.0: More colors near set boundary (glow)
    /// = 1.0: Linear (no bias)
    /// > 1.0: More colors in outer regions
    #[serde(default = "default_transfer_bias")]
    pub transfer_bias: f32,
    /// Whether GPU rendering is enabled.
    #[serde(default = "default_use_gpu")]
    pub use_gpu: bool,
}

fn default_transfer_bias() -> f32 {
    DEFAULT_TRANSFER_BIAS
}

fn default_use_gpu() -> bool {
    true
}

impl Default for ColorOptions {
    fn default() -> Self {
        Self {
            palette_id: "classic".to_string(),
            shading_enabled: false,
            smooth_enabled: true,
            histogram_enabled: false,
            cycle_count: 32,
            transfer_bias: DEFAULT_TRANSFER_BIAS,
            use_gpu: true,
        }
    }
}

impl ColorOptions {
    /// Valid cycle counts: 1 to 1024.
    pub fn is_valid_cycle_count(n: u32) -> bool {
        (1..=1024).contains(&n)
    }

    /// Increase cycle count by given amount (max 1024).
    pub fn cycle_up_by(&mut self, amount: u32) {
        self.cycle_count = (self.cycle_count + amount).min(1024);
    }

    /// Decrease cycle count by given amount (min 1).
    pub fn cycle_down_by(&mut self, amount: u32) {
        self.cycle_count = self.cycle_count.saturating_sub(amount).max(1);
    }

    /// Increase cycle count by 1 (max 1024).
    pub fn cycle_up(&mut self) {
        self.cycle_up_by(1);
    }

    /// Decrease cycle count by 1 (min 1).
    pub fn cycle_down(&mut self) {
        self.cycle_down_by(1);
    }

    /// Get the palette for this options.
    pub fn palette(&self) -> Palette {
        palettes()
            .into_iter()
            .find(|p| p.id == self.palette_id)
            .map(|p| p.palette)
            .unwrap_or_else(Palette::ultra_fractal)
    }

    /// Get shading settings.
    pub fn shading(&self) -> ShadingSettings {
        if self.shading_enabled {
            ShadingSettings::enabled()
        } else {
            ShadingSettings::disabled()
        }
    }

    /// Increase transfer bias (more colors in outer regions).
    pub fn bias_up(&mut self) {
        self.transfer_bias = (self.transfer_bias + TRANSFER_BIAS_STEP).min(MAX_TRANSFER_BIAS);
        self.transfer_bias = self.transfer_bias.round();
    }

    /// Decrease transfer bias (more colors near set boundary / glow).
    pub fn bias_down(&mut self) {
        self.transfer_bias = (self.transfer_bias - TRANSFER_BIAS_STEP).max(MIN_TRANSFER_BIAS);
        // Round to integer, but preserve MIN if we're at the minimum
        if self.transfer_bias > MIN_TRANSFER_BIAS {
            self.transfer_bias = self.transfer_bias.round().max(MIN_TRANSFER_BIAS);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_shading_is_disabled() {
        let settings = ShadingSettings::default();
        assert!(!settings.enabled);
    }

    #[test]
    fn color_options_default_values() {
        let options = ColorOptions::default();
        assert_eq!(options.palette_id, "classic");
        assert!(!options.shading_enabled);
        assert!(options.smooth_enabled);
        assert!(!options.histogram_enabled);
        assert_eq!(options.cycle_count, 32);
    }

    #[test]
    fn color_options_palette_lookup() {
        let options = ColorOptions {
            palette_id: "fire".to_string(),
            ..Default::default()
        };
        // Fire palette starts dark, sample at 0 should be near black
        let sample = options.palette().sample(0.0);
        assert_eq!(sample, [0, 0, 0]);
    }

    #[test]
    fn color_options_shading() {
        let options = ColorOptions {
            shading_enabled: true,
            ..Default::default()
        };
        assert!(options.shading().enabled);
    }

    #[test]
    fn color_options_cycle_count_valid_range() {
        assert!(ColorOptions::is_valid_cycle_count(1));
        assert!(ColorOptions::is_valid_cycle_count(2));
        assert!(ColorOptions::is_valid_cycle_count(3));
        assert!(ColorOptions::is_valid_cycle_count(32));
        assert!(ColorOptions::is_valid_cycle_count(128));
        assert!(ColorOptions::is_valid_cycle_count(500));
        assert!(ColorOptions::is_valid_cycle_count(1024));
        assert!(!ColorOptions::is_valid_cycle_count(0));
        assert!(!ColorOptions::is_valid_cycle_count(1025));
        assert!(!ColorOptions::is_valid_cycle_count(2048));
    }

    #[test]
    fn color_options_cycle_up_down_linear() {
        let mut options = ColorOptions {
            cycle_count: 10,
            ..Default::default()
        };

        options.cycle_up();
        assert_eq!(options.cycle_count, 11);

        options.cycle_down();
        assert_eq!(options.cycle_count, 10);

        // Test fast increment/decrement
        options.cycle_up_by(50);
        assert_eq!(options.cycle_count, 60);

        options.cycle_down_by(50);
        assert_eq!(options.cycle_count, 10);
    }

    #[test]
    fn color_options_cycle_upper_bound() {
        let mut options = ColorOptions {
            cycle_count: 1020,
            ..Default::default()
        };
        options.cycle_up_by(50);
        assert_eq!(options.cycle_count, 1024);

        options.cycle_up();
        assert_eq!(options.cycle_count, 1024); // Still capped
    }

    #[test]
    fn color_options_cycle_lower_bound() {
        let mut options = ColorOptions {
            cycle_count: 30,
            ..Default::default()
        };
        options.cycle_down_by(50);
        assert_eq!(options.cycle_count, 1);

        options.cycle_down();
        assert_eq!(options.cycle_count, 1); // Still at min
    }

    #[test]
    fn transfer_bias_default_is_one() {
        let options = ColorOptions::default();
        assert!((options.transfer_bias - 1.0).abs() < 0.001);
    }

    #[test]
    fn transfer_bias_up_down() {
        let mut options = ColorOptions::default();
        assert!((options.transfer_bias - 1.0).abs() < 0.001);

        options.bias_up();
        assert!((options.transfer_bias - 2.0).abs() < 0.001);

        options.bias_down();
        assert!((options.transfer_bias - 1.0).abs() < 0.001);

        // Going below 1.0 clamps to MIN (0.1)
        options.bias_down();
        assert!((options.transfer_bias - MIN_TRANSFER_BIAS).abs() < 0.001);
    }

    #[test]
    fn transfer_bias_respects_bounds() {
        let mut options = ColorOptions {
            transfer_bias: MIN_TRANSFER_BIAS,
            ..Default::default()
        };
        options.bias_down();
        assert!((options.transfer_bias - MIN_TRANSFER_BIAS).abs() < 0.001);

        options.transfer_bias = MAX_TRANSFER_BIAS;
        options.bias_up();
        assert!((options.transfer_bias - MAX_TRANSFER_BIAS).abs() < 0.001);
    }

    #[test]
    fn apply_transfer_bias_at_boundaries() {
        // All bias values should map 0 -> 0 and 1 -> 1
        for bias in [0.1, 0.5, 1.0, 2.0, 5.0, 10.0] {
            assert!(
                (apply_transfer_bias(0.0, bias) - 0.0).abs() < 0.001,
                "bias {} should map 0 to 0",
                bias
            );
            assert!(
                (apply_transfer_bias(1.0, bias) - 1.0).abs() < 0.001,
                "bias {} should map 1 to 1",
                bias
            );
        }
    }

    #[test]
    fn apply_transfer_bias_ordering() {
        // At t=0.5:
        // - bias < 1: result > 0.5 (more colors near boundary)
        // - bias = 1: result = 0.5 (linear)
        // - bias > 1: result < 0.5 (more colors in outer regions)
        let low_bias = apply_transfer_bias(0.5, 0.5);
        let linear = apply_transfer_bias(0.5, 1.0);
        let high_bias = apply_transfer_bias(0.5, 2.0);

        assert!(low_bias > linear, "Low bias should expand low values");
        assert!(
            (linear - 0.5).abs() < 0.001,
            "Linear bias should be identity"
        );
        assert!(high_bias < linear, "High bias should compress low values");
    }
}
