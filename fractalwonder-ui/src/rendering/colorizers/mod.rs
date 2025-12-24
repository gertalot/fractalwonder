pub mod color_space;
pub mod colorizer;
pub mod curve;
pub mod palette;
pub mod pipeline;
pub mod settings;
pub mod shading;
pub mod smooth_iteration;

use fractalwonder_core::ComputeData;

pub use colorizer::{Colorizer, ColorizerKind};
pub use curve::{Curve, CurvePoint};
pub use palette::Palette;
pub use pipeline::ColorPipeline;
pub use settings::{apply_transfer_bias, ColorOptions, ShadingSettings};
pub use shading::apply_slope_shading;
pub use smooth_iteration::{SmoothIterationColorizer, SmoothIterationContext};

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
            id: "stripey_inferno",
            name: "Stripey Inferno",
            palette: Palette::stripey_inferno(),
        },
        PaletteEntry {
            id: "aurora",
            name: "Aurora",
            palette: Palette::aurora(),
        },
    ]
}

/// Colorize a single pixel.
/// Palette should be pre-cached by calling `options.palette()` once before iterating.
pub fn colorize_with_palette(
    data: &ComputeData,
    options: &ColorOptions,
    palette: &Palette,
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

    colorizer.colorize(data, options, palette)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn palettes_returns_12_entries() {
        let palettes = palettes();
        assert_eq!(palettes.len(), 12);
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
