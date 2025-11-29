//! Color settings for the colorization pipeline.

use super::Palette;

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
            cycle_count: 1.0,
            shading: ShadingSettings::default(),
        }
    }
}

impl ColorSettings {
    /// Create settings with the given palette and default shading.
    pub fn with_palette(palette: Palette) -> Self {
        Self {
            palette,
            cycle_count: 1.0,
            shading: ShadingSettings::default(),
        }
    }

    /// Create settings with shading enabled.
    pub fn with_shading(palette: Palette) -> Self {
        Self {
            palette,
            cycle_count: 1.0,
            shading: ShadingSettings::enabled(),
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
        assert_eq!(settings.cycle_count, 1.0);
        assert!(!settings.shading.enabled);
    }

    #[test]
    fn with_shading_enables_shading() {
        let settings = ColorSettings::with_shading(Palette::grayscale());
        assert!(settings.shading.enabled);
    }
}
