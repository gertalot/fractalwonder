use crate::rendering::colorizers::{Colorizer, RendererColorizers, COLORIZERS};
use fractalwonder_compute::RendererInfo;

pub struct RendererPresentationConfig {
    pub id: &'static str,
    pub display_name: &'static str,
    pub create_info_provider: fn() -> Box<dyn RendererInfo<Scalar = f64>>,
}

pub static RENDERER_CONFIGS: &[RendererPresentationConfig] = &[
    RendererPresentationConfig {
        id: "test_image",
        display_name: "Test Image",
        create_info_provider: || Box::new(fractalwonder_compute::TestImageComputer::<f64>::new()),
    },
    RendererPresentationConfig {
        id: "mandelbrot",
        display_name: "Mandelbrot",
        create_info_provider: || Box::new(fractalwonder_compute::MandelbrotComputer::<f64>::new()),
    },
];

/// Get renderer config by ID
pub fn get_renderer_config(id: &str) -> Option<&'static RendererPresentationConfig> {
    RENDERER_CONFIGS.iter().find(|c| c.id == id)
}

/// Get colorizers for a specific renderer
pub fn get_colorizers_for_renderer(renderer_id: &str) -> Option<&'static RendererColorizers> {
    COLORIZERS.iter().find(|c| c.renderer_id == renderer_id)
}

/// Get specific colorizer by renderer and colorizer ID
pub fn get_colorizer(renderer_id: &str, colorizer_id: &str) -> Option<Colorizer> {
    let renderer_colorizers = get_colorizers_for_renderer(renderer_id)?;
    renderer_colorizers
        .colorizers
        .iter()
        .find(|c| c.id == colorizer_id)
        .map(|c| c.colorizer)
}

/// Get default colorizer ID for a renderer
pub fn get_default_colorizer_id(renderer_id: &str) -> Option<&'static str> {
    let renderer_colorizers = get_colorizers_for_renderer(renderer_id)?;
    renderer_colorizers
        .colorizers
        .iter()
        .find(|c| c.is_default)
        .map(|c| c.id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_renderer_config() {
        assert!(get_renderer_config("mandelbrot").is_some());
        assert!(get_renderer_config("test_image").is_some());
        assert!(get_renderer_config("unknown").is_none());
    }

    #[test]
    fn test_get_colorizers_for_renderer() {
        let mandelbrot_colorizers = get_colorizers_for_renderer("mandelbrot").unwrap();
        assert_eq!(mandelbrot_colorizers.colorizers.len(), 3);

        let test_image_colorizers = get_colorizers_for_renderer("test_image").unwrap();
        assert_eq!(test_image_colorizers.colorizers.len(), 2);
    }

    #[test]
    fn test_get_colorizer() {
        assert!(get_colorizer("mandelbrot", "default").is_some());
        assert!(get_colorizer("mandelbrot", "fire").is_some());
        assert!(get_colorizer("mandelbrot", "opal").is_some());
        assert!(get_colorizer("mandelbrot", "unknown").is_none());
        assert!(get_colorizer("unknown", "default").is_none());
    }

    #[test]
    fn test_get_default_colorizer_id() {
        assert_eq!(get_default_colorizer_id("mandelbrot"), Some("default"));
        assert_eq!(get_default_colorizer_id("test_image"), Some("default"));
        assert_eq!(get_default_colorizer_id("unknown"), None);
    }

    #[test]
    fn test_all_renderer_configs_have_colorizers() {
        for config in RENDERER_CONFIGS.iter() {
            assert!(
                get_colorizers_for_renderer(config.id).is_some(),
                "Renderer {} has no colorizers",
                config.id
            );
        }
    }
}
