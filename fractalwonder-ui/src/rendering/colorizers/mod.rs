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
pub use settings::{ColorSettings, ShadingSettings};
pub use smooth_iteration::SmoothIterationColorizer;

/// Colorize a single pixel using the provided palette and colorizer.
/// For progressive rendering (quick path, no pre/post processing).
pub fn colorize_with_palette(
    data: &ComputeData,
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

    colorizer.colorize_quick(data, palette)
}
