//! Color settings for the colorization pipeline.

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
}
