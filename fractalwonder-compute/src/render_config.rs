use crate::computers::{MandelbrotComputer, TestImageComputer};
use crate::renderer_info::RendererInfo;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct ColorScheme {
    pub id: &'static str,
    pub display_name: &'static str,
}

pub struct RenderConfig {
    pub id: &'static str,
    pub display_name: &'static str,
    pub color_schemes: &'static [ColorScheme],
    pub default_color_scheme_id: &'static str,
    pub create_info_provider: fn() -> Box<dyn RendererInfo<Scalar = f64>>,
}

pub static RENDER_CONFIGS: &[RenderConfig] = &[
    RenderConfig {
        id: "test_image",
        display_name: "Test Image",
        color_schemes: &[
            ColorScheme {
                id: "default",
                display_name: "Default",
            },
            ColorScheme {
                id: "pastel",
                display_name: "Pastel",
            },
        ],
        default_color_scheme_id: "default",
        create_info_provider: || Box::new(TestImageComputer::new()),
    },
    RenderConfig {
        id: "mandelbrot",
        display_name: "Mandelbrot",
        color_schemes: &[
            ColorScheme {
                id: "default",
                display_name: "Default",
            },
            ColorScheme {
                id: "fire",
                display_name: "Fire",
            },
            ColorScheme {
                id: "opal",
                display_name: "Opal",
            },
        ],
        default_color_scheme_id: "default",
        create_info_provider: || Box::new(MandelbrotComputer::new()),
    },
];

pub fn get_config(id: &str) -> Option<&'static RenderConfig> {
    RENDER_CONFIGS.iter().find(|c| c.id == id)
}

pub fn get_color_scheme<'a>(config: &'a RenderConfig, scheme_id: &str) -> Option<&'a ColorScheme> {
    config.color_schemes.iter().find(|cs| cs.id == scheme_id)
}
