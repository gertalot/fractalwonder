use crate::adaptive_mandelbrot_renderer::AdaptiveMandelbrotRenderer;
use crate::app_data_renderer::AppDataRenderer;
use crate::computers::{MandelbrotComputer, TestImageComputer};
use crate::pixel_renderer::PixelRenderer;
use crate::renderer_info::RendererInfo;
use crate::renderer_trait::Renderer;
use fractalwonder_core::{AppData, BigFloat};
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
    pub create_renderer: fn() -> Box<dyn Renderer<Scalar = BigFloat, Data = AppData>>,
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
        create_info_provider: || Box::new(TestImageComputer::<f64>::new()),
        create_renderer: || {
            let computer = TestImageComputer::<BigFloat>::new();
            let pixel_renderer = PixelRenderer::new(computer);
            let app_renderer = AppDataRenderer::new(pixel_renderer, |d| AppData::TestImageData(*d));
            Box::new(app_renderer)
        },
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
        create_renderer: || Box::new(AdaptiveMandelbrotRenderer::new(1e10)),
    },
];

pub fn get_config(id: &str) -> Option<&'static RenderConfig> {
    RENDER_CONFIGS.iter().find(|c| c.id == id)
}

pub fn get_color_scheme<'a>(config: &'a RenderConfig, scheme_id: &str) -> Option<&'a ColorScheme> {
    config.color_schemes.iter().find(|cs| cs.id == scheme_id)
}

/// Create a renderer by ID, or return None if unknown
pub fn create_renderer(
    renderer_id: &str,
) -> Option<Box<dyn Renderer<Scalar = BigFloat, Data = AppData>>> {
    get_config(renderer_id).map(|config| (config.create_renderer)())
}
