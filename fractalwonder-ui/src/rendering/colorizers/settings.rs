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
    /// Number of palette cycles (power of 2: 1, 2, 4, ..., 128).
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
    /// Valid cycle counts: powers of 2 from 1 to 1024.
    pub fn is_valid_cycle_count(n: u32) -> bool {
        n > 0 && n <= 1024 && n.is_power_of_two()
    }

    /// Double cycle count (max 1024).
    pub fn cycle_up(&mut self) {
        if self.cycle_count < 1024 {
            self.cycle_count *= 2;
        }
    }

    /// Halve cycle count (min 1).
    pub fn cycle_down(&mut self) {
        if self.cycle_count > 1 {
            self.cycle_count /= 2;
        }
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
    fn color_options_cycle_power_of_two() {
        assert!(ColorOptions::is_valid_cycle_count(1));
        assert!(ColorOptions::is_valid_cycle_count(2));
        assert!(ColorOptions::is_valid_cycle_count(32));
        assert!(ColorOptions::is_valid_cycle_count(128));
        assert!(ColorOptions::is_valid_cycle_count(1024));
        assert!(!ColorOptions::is_valid_cycle_count(3));
        assert!(!ColorOptions::is_valid_cycle_count(0));
        assert!(!ColorOptions::is_valid_cycle_count(2048));
    }
}
