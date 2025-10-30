use crate::rendering::colorizers::{
    mandelbrot_default_colorizer, mandelbrot_fire_colorizer, mandelbrot_opal_colorizer,
    test_image_default_colorizer, test_image_pastel_colorizer,
};
use crate::rendering::renderer_info::RendererInfo;
use crate::rendering::{
    AppData, AppDataRenderer, Colorizer, MandelbrotComputer, PixelRenderer, Renderer,
    TestImageComputer,
};

pub struct ColorScheme {
    pub id: &'static str,
    pub display_name: &'static str,
    pub colorizer: Colorizer<AppData>,
}

pub struct RenderConfig {
    pub id: &'static str,
    pub display_name: &'static str,
    pub color_schemes: &'static [ColorScheme],
    pub default_color_scheme_id: &'static str,
    pub create_renderer: fn() -> Box<dyn Renderer<Coord = f64, Data = AppData>>,
    pub create_info_provider: fn() -> Box<dyn RendererInfo<Coord = f64>>,
}

fn create_test_image_renderer() -> Box<dyn Renderer<Coord = f64, Data = AppData>> {
    let computer = TestImageComputer::new();
    let pixel_renderer = PixelRenderer::new(computer);
    let app_renderer = AppDataRenderer::new(pixel_renderer, |d| AppData::TestImageData(*d));
    Box::new(app_renderer)
}

fn create_mandelbrot_renderer() -> Box<dyn Renderer<Coord = f64, Data = AppData>> {
    let computer = MandelbrotComputer::new();
    let pixel_renderer = PixelRenderer::new(computer);
    let app_renderer = AppDataRenderer::new(pixel_renderer, |d| AppData::MandelbrotData(*d));
    Box::new(app_renderer)
}

pub static RENDER_CONFIGS: &[RenderConfig] = &[
    RenderConfig {
        id: "test_image",
        display_name: "Test Image",
        color_schemes: &[
            ColorScheme {
                id: "default",
                display_name: "Default",
                colorizer: test_image_default_colorizer,
            },
            ColorScheme {
                id: "pastel",
                display_name: "Pastel",
                colorizer: test_image_pastel_colorizer,
            },
        ],
        default_color_scheme_id: "default",
        create_renderer: create_test_image_renderer,
        create_info_provider: || Box::new(TestImageComputer::new()),
    },
    RenderConfig {
        id: "mandelbrot",
        display_name: "Mandelbrot",
        color_schemes: &[
            ColorScheme {
                id: "default",
                display_name: "Default",
                colorizer: mandelbrot_default_colorizer,
            },
            ColorScheme {
                id: "fire",
                display_name: "Fire",
                colorizer: mandelbrot_fire_colorizer,
            },
            ColorScheme {
                id: "opal",
                display_name: "Opal",
                colorizer: mandelbrot_opal_colorizer,
            },
        ],
        default_color_scheme_id: "default",
        create_renderer: create_mandelbrot_renderer,
        create_info_provider: || Box::new(MandelbrotComputer::new()),
    },
];

pub fn get_config(id: &str) -> Option<&'static RenderConfig> {
    RENDER_CONFIGS.iter().find(|c| c.id == id)
}

pub fn get_color_scheme<'a>(config: &'a RenderConfig, scheme_id: &str) -> Option<&'a ColorScheme> {
    config.color_schemes.iter().find(|cs| cs.id == scheme_id)
}
