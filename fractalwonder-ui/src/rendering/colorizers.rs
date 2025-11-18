#[cfg(test)]
use fractalwonder_core::MandelbrotData;
use fractalwonder_core::{AppData, TestImageData};

/// Colorizer function type - converts Data to RGBA
pub type Colorizer<D> = fn(&D) -> (u8, u8, u8, u8);

/// Colorize TestImageData - Default scheme
pub fn test_image_default_colorizer(data: &AppData) -> (u8, u8, u8, u8) {
    match data {
        AppData::TestImageData(d) => test_image_data_to_rgba(d),
        #[allow(unreachable_patterns)]
        _ => (0, 0, 0, 255), // Black for wrong type
    }
}

fn test_image_data_to_rgba(data: &TestImageData) -> (u8, u8, u8, u8) {
    // Circle distance < 0.1 means on a circle -> red
    if data.circle_distance < 0.1 {
        return (255, 0, 0, 255); // Red circle
    }

    // Checkerboard pattern
    if data.checkerboard {
        (255, 255, 255, 255) // White
    } else {
        (204, 204, 204, 255) // Light grey
    }
}

/// Colorize TestImageData - Pastel scheme
pub fn test_image_pastel_colorizer(data: &AppData) -> (u8, u8, u8, u8) {
    match data {
        AppData::TestImageData(d) => {
            let base_hue = if d.checkerboard { 200.0 } else { 50.0 }; // Blue vs Yellow
            let lightness = 0.7 + (d.circle_distance.sin() * 0.2); // 0.5-0.9 range
            let saturation = 0.4; // Pastel = low saturation

            // HSL to RGB conversion (simplified)
            let c = (1.0 - (2.0 * lightness - 1.0).abs()) * saturation;
            let h_prime = base_hue / 60.0;
            let h_mod: f64 = h_prime % 2.0;
            let x = c * (1.0 - (h_mod - 1.0).abs());

            let (r1, g1, b1) = match h_prime as i32 {
                0..=1 => (c, x, 0.0),
                2..=3 => (0.0, c, x),
                4..=5 => (x, 0.0, c),
                _ => (c, x, 0.0),
            };

            let m = lightness - c / 2.0;
            let r = ((r1 + m) * 255.0) as u8;
            let g = ((g1 + m) * 255.0) as u8;
            let b = ((b1 + m) * 255.0) as u8;

            (r, g, b, 255)
        }
        #[allow(unreachable_patterns)]
        _ => (0, 0, 0, 255),
    }
}

/// Colorize MandelbrotData - Default grayscale scheme
pub fn mandelbrot_default_colorizer(data: &AppData) -> (u8, u8, u8, u8) {
    match data {
        AppData::MandelbrotData(d) => {
            if !d.escaped {
                return (0, 0, 0, 255); // Set interior is black
            }

            // Grayscale gradient based on iteration count
            let normalized = (d.iterations as f64 / 256.0).min(1.0);
            let intensity = (normalized * 255.0) as u8;

            (intensity, intensity, intensity, 255)
        }
        #[allow(unreachable_patterns)]
        _ => (0, 0, 0, 255),
    }
}

/// Colorize MandelbrotData - Fire scheme (black → red → orange → yellow)
pub fn mandelbrot_fire_colorizer(data: &AppData) -> (u8, u8, u8, u8) {
    match data {
        AppData::MandelbrotData(d) => {
            if !d.escaped {
                return (0, 0, 0, 255); // Set interior is black
            }

            // Fire gradient: Black -> Red -> Orange -> Yellow -> White
            let t = (d.iterations as f64 / 256.0).min(1.0);

            let r = (t * 255.0) as u8;
            let g = if t > 0.5 {
                ((t - 0.5) * 2.0 * 255.0) as u8
            } else {
                0
            };
            let b = if t > 0.8 {
                ((t - 0.8) * 5.0 * 255.0) as u8
            } else {
                0
            };

            (r, g, b, 255)
        }
        #[allow(unreachable_patterns)]
        _ => (0, 0, 0, 255),
    }
}

/// Colorize MandelbrotData - Opal scheme (black → deep blue → cyan → white)
pub fn mandelbrot_opal_colorizer(data: &AppData) -> (u8, u8, u8, u8) {
    match data {
        AppData::MandelbrotData(d) => {
            if !d.escaped {
                return (0, 0, 0, 255); // Set interior is black
            }

            // Opal gradient: Black -> Deep Blue -> Cyan -> White
            let t = (d.iterations as f64 / 256.0).min(1.0);

            let r = if t > 0.6 {
                ((t - 0.6) * 2.5 * 255.0) as u8
            } else {
                0
            };
            let g = if t > 0.4 {
                ((t - 0.4) * 1.67 * 255.0) as u8
            } else {
                0
            };
            let b = (t * 255.0) as u8;

            (r, g, b, 255)
        }
        #[allow(unreachable_patterns)]
        _ => (0, 0, 0, 255),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_colorizer_on_circle() {
        let data = AppData::TestImageData(TestImageData::new(true, 0.05));
        let color = test_image_default_colorizer(&data);
        assert_eq!(color, (255, 0, 0, 255)); // Red
    }

    #[test]
    fn test_colorizer_checkerboard_white() {
        let data = AppData::TestImageData(TestImageData::new(true, 5.0));
        let color = test_image_default_colorizer(&data);
        assert_eq!(color, (255, 255, 255, 255)); // White
    }

    #[test]
    fn test_colorizer_checkerboard_grey() {
        let data = AppData::TestImageData(TestImageData::new(false, 5.0));
        let color = test_image_default_colorizer(&data);
        assert_eq!(color, (204, 204, 204, 255)); // Grey
    }

    #[test]
    fn test_test_image_pastel_colorizer() {
        let data = AppData::TestImageData(TestImageData {
            checkerboard: true,
            circle_distance: 0.5,
        });
        let (r, g, b, a) = test_image_pastel_colorizer(&data);
        assert_eq!(a, 255); // Always opaque
        assert!(r < 255 && g < 255 && b < 255); // Not pure white
    }

    #[test]
    fn test_mandelbrot_default_colorizer() {
        let data = AppData::MandelbrotData(MandelbrotData {
            iterations: 128,
            escaped: true,
        });
        let (r, g, b, a) = mandelbrot_default_colorizer(&data);
        assert_eq!(a, 255); // Always opaque
        assert_eq!(r, g); // Grayscale
        assert_eq!(g, b);
    }

    #[test]
    fn test_mandelbrot_fire_colorizer() {
        let data = AppData::MandelbrotData(MandelbrotData {
            iterations: 64,
            escaped: true,
        });
        let (r, _g, b, a) = mandelbrot_fire_colorizer(&data);
        assert_eq!(a, 255); // Always opaque
        assert!(r > b); // Fire has more red than blue
    }

    #[test]
    fn test_mandelbrot_opal_colorizer() {
        let data = AppData::MandelbrotData(MandelbrotData {
            iterations: 64,
            escaped: true,
        });
        let (r, _g, b, a) = mandelbrot_opal_colorizer(&data);
        assert_eq!(a, 255); // Always opaque
        assert!(b > r); // Opal has more blue than red
    }

    #[test]
    fn test_mandelbrot_set_interior_is_black() {
        let data = AppData::MandelbrotData(MandelbrotData {
            iterations: 256,
            escaped: false,
        });

        let (r, g, b, _) = mandelbrot_default_colorizer(&data);
        assert_eq!((r, g, b), (0, 0, 0));

        let (r, g, b, _) = mandelbrot_fire_colorizer(&data);
        assert_eq!((r, g, b), (0, 0, 0));

        let (r, g, b, _) = mandelbrot_opal_colorizer(&data);
        assert_eq!((r, g, b), (0, 0, 0));
    }
}
