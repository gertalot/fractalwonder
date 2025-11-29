//! Color scheme presets bundling palettes and colorizers.

use super::{ColorizerKind, Palette, SmoothIterationColorizer};

/// A color scheme preset combining a palette and colorizer.
#[derive(Clone, Debug)]
pub struct ColorSchemePreset {
    pub name: &'static str,
    pub palette: Palette,
    pub colorizer: ColorizerKind,
}

/// Get all available color scheme presets.
pub fn presets() -> Vec<ColorSchemePreset> {
    vec![
        ColorSchemePreset {
            name: "Classic",
            palette: Palette::ultra_fractal(),
            colorizer: ColorizerKind::SmoothIteration(SmoothIterationColorizer),
        },
        ColorSchemePreset {
            name: "Fire",
            palette: Palette::fire(),
            colorizer: ColorizerKind::SmoothIteration(SmoothIterationColorizer),
        },
        ColorSchemePreset {
            name: "Ocean",
            palette: Palette::ocean(),
            colorizer: ColorizerKind::SmoothIteration(SmoothIterationColorizer),
        },
        ColorSchemePreset {
            name: "Electric",
            palette: Palette::electric(),
            colorizer: ColorizerKind::SmoothIteration(SmoothIterationColorizer),
        },
        ColorSchemePreset {
            name: "Grayscale",
            palette: Palette::grayscale(),
            colorizer: ColorizerKind::SmoothIteration(SmoothIterationColorizer),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presets_not_empty() {
        assert!(!presets().is_empty());
    }

    #[test]
    fn all_presets_have_unique_names() {
        let presets = presets();
        let names: Vec<_> = presets.iter().map(|p| p.name).collect();
        let mut unique = names.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(names.len(), unique.len(), "Duplicate preset names found");
    }

    #[test]
    fn classic_preset_exists() {
        let presets = presets();
        assert!(presets.iter().any(|p| p.name == "Classic"));
    }
}
