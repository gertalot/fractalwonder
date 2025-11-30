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

/// All settings that affect colorization (not compute).
#[derive(Clone, Debug)]
pub struct ColorSettings {
    /// Color palette for mapping iteration values to colors.
    pub palette: Palette,
    /// Number of times to cycle through the palette.
    pub cycle_count: f64,
    /// Slope shading settings.
    pub shading: ShadingSettings,
}

impl Default for ColorSettings {
    fn default() -> Self {
        Self {
            palette: Palette::ultra_fractal(),
            cycle_count: 32.0, // Cycle palette for better contrast at deep zooms
            shading: ShadingSettings::default(),
        }
    }
}

impl ColorSettings {
    /// Create settings with the given palette and default shading.
    pub fn with_palette(palette: Palette) -> Self {
        Self {
            palette,
            cycle_count: 32.0,
            shading: ShadingSettings::default(),
        }
    }

    /// Create settings with shading enabled.
    pub fn with_shading(palette: Palette) -> Self {
        Self {
            palette,
            cycle_count: 32.0,
            shading: ShadingSettings::enabled(),
        }
    }
}

/// User-configurable color options for the UI.
/// Converted to ColorSettings for rendering.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ColorOptions {
    /// Palette ID (e.g., "classic", "fire").
    pub palette_id: String,
    /// Whether 3D slope shading is enabled.
    pub shading_enabled: bool,
    /// Whether smooth iteration coloring is enabled.
    pub smooth_enabled: bool,
    /// Number of palette cycles (power of 2: 1, 2, 4, ..., 128).
    pub cycle_count: u32,
}

impl Default for ColorOptions {
    fn default() -> Self {
        Self {
            palette_id: "classic".to_string(),
            shading_enabled: false,
            smooth_enabled: true,
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

    /// Convert to ColorSettings for rendering.
    pub fn to_color_settings(&self) -> ColorSettings {
        let palette = palettes()
            .into_iter()
            .find(|p| p.id == self.palette_id)
            .map(|p| p.palette)
            .unwrap_or_else(Palette::ultra_fractal);

        ColorSettings {
            palette,
            cycle_count: self.cycle_count as f64,
            shading: if self.shading_enabled {
                ShadingSettings::enabled()
            } else {
                ShadingSettings::disabled()
            },
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
    fn color_settings_default_has_palette() {
        let settings = ColorSettings::default();
        assert_eq!(settings.cycle_count, 32.0);
        assert!(!settings.shading.enabled);
    }

    #[test]
    fn with_shading_enables_shading() {
        let settings = ColorSettings::with_shading(Palette::grayscale());
        assert!(settings.shading.enabled);
    }

    #[test]
    fn color_options_default_values() {
        let options = ColorOptions::default();
        assert_eq!(options.palette_id, "classic");
        assert!(!options.shading_enabled);
        assert!(options.smooth_enabled);
        assert_eq!(options.cycle_count, 32);
    }

    #[test]
    fn color_options_to_color_settings_uses_palette() {
        let options = ColorOptions {
            palette_id: "fire".to_string(),
            ..Default::default()
        };
        let settings = options.to_color_settings();
        // Fire palette starts dark, sample at 0 should be near black
        let sample = settings.palette.sample(0.0);
        assert_eq!(sample, [0, 0, 0]);
    }

    #[test]
    fn color_options_to_color_settings_shading() {
        let options = ColorOptions {
            shading_enabled: true,
            ..Default::default()
        };
        let settings = options.to_color_settings();
        assert!(settings.shading.enabled);
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
