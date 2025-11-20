use fractalwonder_core::{AppData, TestImageData};

pub type Colorizer = fn(&AppData) -> (u8, u8, u8, u8);

#[derive(Clone, Copy)]
pub struct ColorizerInfo {
    pub id: &'static str,
    pub display_name: &'static str,
    pub is_default: bool,
    pub colorizer: Colorizer,
}

pub struct RendererColorizers {
    pub renderer_id: &'static str,
    pub colorizers: &'static [ColorizerInfo],
}

/// Single static registry - all colorizers organized by renderer
pub static COLORIZERS: &[RendererColorizers] = &[
    RendererColorizers {
        renderer_id: "mandelbrot",
        colorizers: &[
            ColorizerInfo {
                id: "default",
                display_name: "Default",
                is_default: true,
                colorizer: mandelbrot_default_colorizer,
            },
            ColorizerInfo {
                id: "fire",
                display_name: "Fire",
                is_default: false,
                colorizer: mandelbrot_fire_colorizer,
            },
            ColorizerInfo {
                id: "opal",
                display_name: "Opal",
                is_default: false,
                colorizer: mandelbrot_opal_colorizer,
            },
        ],
    },
    RendererColorizers {
        renderer_id: "test_image",
        colorizers: &[
            ColorizerInfo {
                id: "default",
                display_name: "Default",
                is_default: true,
                colorizer: test_image_default_colorizer,
            },
            ColorizerInfo {
                id: "pastel",
                display_name: "Pastel",
                is_default: false,
                colorizer: test_image_pastel_colorizer,
            },
        ],
    },
];

// === Mandelbrot Colorizers ===

fn mandelbrot_default_colorizer(data: &AppData) -> (u8, u8, u8, u8) {
    match data {
        AppData::MandelbrotData(d) => {
            if !d.escaped {
                return (0, 0, 0, 255);
            }
            let normalized = (d.iterations as f64 / 256.0).min(1.0);
            let intensity = (normalized * 255.0) as u8;
            (intensity, intensity, intensity, 255)
        }
        _ => (0, 0, 0, 255),
    }
}

fn mandelbrot_fire_colorizer(data: &AppData) -> (u8, u8, u8, u8) {
    match data {
        AppData::MandelbrotData(d) => {
            if !d.escaped {
                return (0, 0, 0, 255);
            }
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
        _ => (0, 0, 0, 255),
    }
}

fn mandelbrot_opal_colorizer(data: &AppData) -> (u8, u8, u8, u8) {
    match data {
        AppData::MandelbrotData(d) => {
            if !d.escaped {
                return (0, 0, 0, 255);
            }
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
        _ => (0, 0, 0, 255),
    }
}

// === Test Image Colorizers ===

fn test_image_default_colorizer(data: &AppData) -> (u8, u8, u8, u8) {
    match data {
        AppData::TestImageData(d) => test_image_data_to_rgba(d),
        _ => (0, 0, 0, 255),
    }
}

fn test_image_data_to_rgba(data: &TestImageData) -> (u8, u8, u8, u8) {
    if data.circle_distance < 0.1 {
        return (255, 0, 0, 255);
    }
    if data.checkerboard {
        (255, 255, 255, 255)
    } else {
        (204, 204, 204, 255)
    }
}

fn test_image_pastel_colorizer(data: &AppData) -> (u8, u8, u8, u8) {
    match data {
        AppData::TestImageData(d) => {
            let base_hue = if d.checkerboard { 200.0 } else { 50.0 };
            let lightness = 0.7 + (d.circle_distance.sin() * 0.2);
            let saturation = 0.4;

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
        _ => (0, 0, 0, 255),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fractalwonder_core::MandelbrotData;

    #[test]
    fn test_colorizer_registry_has_all_renderers() {
        assert_eq!(COLORIZERS.len(), 2);
        assert!(COLORIZERS.iter().any(|r| r.renderer_id == "mandelbrot"));
        assert!(COLORIZERS.iter().any(|r| r.renderer_id == "test_image"));
    }

    #[test]
    fn test_each_renderer_has_default_colorizer() {
        for renderer_colorizers in COLORIZERS.iter() {
            let has_default = renderer_colorizers.colorizers.iter().any(|c| c.is_default);
            assert!(
                has_default,
                "Renderer {} missing default colorizer",
                renderer_colorizers.renderer_id
            );
        }
    }

    #[test]
    fn test_colorizer_on_circle() {
        let data = AppData::TestImageData(TestImageData::new(true, 0.05));
        let color = test_image_default_colorizer(&data);
        assert_eq!(color, (255, 0, 0, 255));
    }

    #[test]
    fn test_colorizer_checkerboard_white() {
        let data = AppData::TestImageData(TestImageData::new(true, 5.0));
        let color = test_image_default_colorizer(&data);
        assert_eq!(color, (255, 255, 255, 255));
    }

    #[test]
    fn test_colorizer_checkerboard_grey() {
        let data = AppData::TestImageData(TestImageData::new(false, 5.0));
        let color = test_image_default_colorizer(&data);
        assert_eq!(color, (204, 204, 204, 255));
    }

    #[test]
    fn test_mandelbrot_default_colorizer() {
        let data = AppData::MandelbrotData(MandelbrotData {
            iterations: 128,
            escaped: true,
        });
        let (r, g, b, a) = mandelbrot_default_colorizer(&data);
        assert_eq!(a, 255);
        assert_eq!(r, g);
        assert_eq!(g, b);
    }

    #[test]
    fn test_mandelbrot_fire_colorizer() {
        let data = AppData::MandelbrotData(MandelbrotData {
            iterations: 64,
            escaped: true,
        });
        let (r, _g, b, a) = mandelbrot_fire_colorizer(&data);
        assert_eq!(a, 255);
        assert!(r > b);
    }

    #[test]
    fn test_mandelbrot_opal_colorizer() {
        let data = AppData::MandelbrotData(MandelbrotData {
            iterations: 64,
            escaped: true,
        });
        let (r, _g, b, a) = mandelbrot_opal_colorizer(&data);
        assert_eq!(a, 255);
        assert!(b > r);
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
