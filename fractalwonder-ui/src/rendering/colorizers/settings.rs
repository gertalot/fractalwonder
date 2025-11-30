//! Color settings for the colorization pipeline.

use super::palettes;
use super::Palette;
use serde::{Deserialize, Serialize};

/// Settings for slope shading effect.
#[derive(Clone, Debug, PartialEq)]
pub struct ShadingSettings {
    /// Whether slope shading is enabled.
    pub enabled: bool,
    /// Light angle in radians. 0 = right, π/2 = top.
    pub light_angle: f64,
    /// Base height factor, auto-scaled by zoom level.
    pub height_factor: f64,
    /// Blend strength. 0.0 = no shading, 1.0 = full effect.
    pub blend: f64,
}

impl Default for ShadingSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            light_angle: std::f64::consts::FRAC_PI_4, // 45° (top-right)
            height_factor: 1.5,
            blend: 0.7,
        }
    }
}

impl ShadingSettings {
    /// Shading disabled.
    pub fn disabled() -> Self {
        Self::default()
    }

    /// Default enabled shading with top-right light.
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
}

impl Default for ColorOptions {
    fn default() -> Self {
        Self {
            palette_id: "classic".to_string(),
            shading_enabled: false,
            smooth_enabled: true,
            histogram_enabled: false,
            cycle_count: 32,
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
    fn enabled_shading_has_reasonable_defaults() {
        let settings = ShadingSettings::enabled();
        assert!(settings.enabled);
        assert!(settings.light_angle > 0.0);
        assert!(settings.height_factor > 0.0);
        assert!(settings.blend > 0.0 && settings.blend <= 1.0);
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
}
