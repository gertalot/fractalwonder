pub mod mandelbrot;
pub mod test_image;

use fractalwonder_core::ComputeData;

pub use mandelbrot::colorize as colorize_mandelbrot;
pub use test_image::colorize as colorize_test_image;

/// Colorizer function type - converts compute data to RGBA pixels.
/// The bool parameter is xray_enabled.
pub type Colorizer = fn(&ComputeData, bool) -> [u8; 4];

/// Dispatch colorization based on ComputeData variant.
/// xray_enabled controls whether glitched pixels are shown in cyan.
pub fn colorize(data: &ComputeData, xray_enabled: bool) -> [u8; 4] {
    match data {
        ComputeData::TestImage(d) => colorize_test_image(d),
        ComputeData::Mandelbrot(d) => colorize_mandelbrot(d, xray_enabled),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fractalwonder_core::{MandelbrotData, TestImageData};

    #[test]
    fn colorize_dispatches_test_image() {
        let data = ComputeData::TestImage(TestImageData {
            is_on_origin: false,
            is_on_x_axis: false,
            is_on_y_axis: false,
            is_on_major_tick_x: false,
            is_on_medium_tick_x: false,
            is_on_minor_tick_x: false,
            is_on_major_tick_y: false,
            is_on_medium_tick_y: false,
            is_on_minor_tick_y: false,
            is_light_cell: true,
        });
        let color = colorize(&data, false);
        // Should be light background (light cell with no special features)
        assert_eq!(color, [245, 245, 245, 255]);
    }

    #[test]
    fn colorize_dispatches_mandelbrot() {
        let data = ComputeData::Mandelbrot(MandelbrotData {
            iterations: 0,
            max_iterations: 1000,
            escaped: false,
            glitched: false,
        });
        let color = colorize(&data, false);
        // Should be black (in set)
        assert_eq!(color, [0, 0, 0, 255]);
    }
}
