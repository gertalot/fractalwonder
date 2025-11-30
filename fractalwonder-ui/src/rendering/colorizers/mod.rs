pub mod color_space;
pub mod colorizer;
pub mod palette;
pub mod presets;
pub mod settings;
pub mod shading;
pub mod smooth_iteration;

use fractalwonder_core::ComputeData;

pub use colorizer::{Colorizer, ColorizerKind};
pub use palette::Palette;
pub use presets::{presets, ColorSchemePreset};
pub use settings::{ColorOptions, ColorSettings, ShadingSettings};
pub use shading::apply_slope_shading;
pub use smooth_iteration::SmoothIterationColorizer;

/// A palette entry with ID, display name, and palette instance.
#[derive(Clone, Debug)]
pub struct PaletteEntry {
    pub id: &'static str,
    pub name: &'static str,
    pub palette: Palette,
}

/// Get all available color palettes.
pub fn palettes() -> Vec<PaletteEntry> {
    vec![
        PaletteEntry {
            id: "classic",
            name: "Classic",
            palette: Palette::ultra_fractal(),
        },
        PaletteEntry {
            id: "fire",
            name: "Fire",
            palette: Palette::fire(),
        },
        PaletteEntry {
            id: "ocean",
            name: "Ocean",
            palette: Palette::ocean(),
        },
        PaletteEntry {
            id: "electric",
            name: "Electric",
            palette: Palette::electric(),
        },
        PaletteEntry {
            id: "grayscale",
            name: "Grayscale",
            palette: Palette::grayscale(),
        },
        PaletteEntry {
            id: "rainbow",
            name: "Rainbow",
            palette: Palette::rainbow(),
        },
        PaletteEntry {
            id: "neon",
            name: "Neon",
            palette: Palette::neon(),
        },
        PaletteEntry {
            id: "twilight",
            name: "Twilight",
            palette: Palette::twilight(),
        },
        PaletteEntry {
            id: "candy",
            name: "Candy",
            palette: Palette::candy(),
        },
        PaletteEntry {
            id: "inferno",
            name: "Inferno",
            palette: Palette::inferno(),
        },
        PaletteEntry {
            id: "aurora",
            name: "Aurora",
            palette: Palette::aurora(),
        },
    ]
}

/// Colorize a single pixel using the provided settings and colorizer.
/// For progressive rendering (quick path, no pre/post processing).
pub fn colorize_with_palette(
    data: &ComputeData,
    settings: &ColorSettings,
    colorizer: &ColorizerKind,
    xray_enabled: bool,
) -> [u8; 4] {
    // Handle xray mode for glitched pixels
    if xray_enabled {
        if let ComputeData::Mandelbrot(m) = data {
            if m.glitched {
                if m.max_iterations == 0 {
                    return [0, 255, 255, 255];
                }
                let normalized = m.iterations as f64 / m.max_iterations as f64;
                let brightness = (64.0 + normalized * 191.0) as u8;
                return [0, brightness, brightness, 255];
            }
        }
    }

    colorizer.colorize_quick(data, settings)
}

/// Colorize using discrete iteration count (no smooth interpolation).
pub fn colorize_discrete(
    data: &ComputeData,
    settings: &ColorSettings,
    xray_enabled: bool,
) -> [u8; 4] {
    // Handle xray mode for glitched pixels
    if xray_enabled {
        if let ComputeData::Mandelbrot(m) = data {
            if m.glitched {
                if m.max_iterations == 0 {
                    return [0, 255, 255, 255];
                }
                let normalized = m.iterations as f64 / m.max_iterations as f64;
                let brightness = (64.0 + normalized * 191.0) as u8;
                return [0, brightness, brightness, 255];
            }
        }
    }

    match data {
        ComputeData::Mandelbrot(m) => {
            if !m.escaped {
                return [0, 0, 0, 255];
            }
            if m.max_iterations == 0 {
                return [0, 0, 0, 255];
            }
            let normalized = m.iterations as f64 / m.max_iterations as f64;
            let t = (normalized * settings.cycle_count).fract();
            let [r, g, b] = settings.palette.sample(t);
            [r, g, b, 255]
        }
        ComputeData::TestImage(_) => [128, 128, 128, 255],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn palettes_returns_11_entries() {
        let palettes = palettes();
        assert_eq!(palettes.len(), 11);
    }

    #[test]
    fn all_palettes_have_unique_ids() {
        let palettes = palettes();
        let ids: Vec<_> = palettes.iter().map(|p| p.id).collect();
        let mut unique = ids.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(ids.len(), unique.len(), "Duplicate palette IDs found");
    }

    #[test]
    fn classic_palette_exists() {
        let palettes = palettes();
        assert!(palettes.iter().any(|p| p.id == "classic"));
    }
}
