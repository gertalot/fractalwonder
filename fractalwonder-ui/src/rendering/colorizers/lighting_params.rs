//! Blinn-Phong lighting parameters.

use serde::{Deserialize, Serialize};

/// Blinn-Phong lighting parameters for 3D shading.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LightingParams {
    pub ambient: f64,
    pub diffuse: f64,
    pub specular: f64,
    pub shininess: f64,
    pub strength: f64,
    pub azimuth: f64,
    pub elevation: f64,
}

impl Default for LightingParams {
    fn default() -> Self {
        Self {
            ambient: 0.75,
            diffuse: 0.5,
            specular: 0.9,
            shininess: 64.0,
            strength: 1.5,
            azimuth: -std::f64::consts::FRAC_PI_2,
            elevation: std::f64::consts::FRAC_PI_4,
        }
    }
}

impl LightingParams {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lighting_params_default_values() {
        let params = LightingParams::default();
        assert!((params.ambient - 0.75).abs() < 0.001);
        assert!((params.elevation - std::f64::consts::FRAC_PI_4).abs() < 0.001);
    }

    #[test]
    fn lighting_params_serialization() {
        let params = LightingParams::default();
        let json = serde_json::to_string(&params).unwrap();
        let parsed: LightingParams = serde_json::from_str(&json).unwrap();
        assert_eq!(params, parsed);
    }
}
