//! Unified Palette struct containing gradient, curves, lighting, and flags.

use super::{ColorStop, Curve, Gradient, LightingParams};
use serde::{Deserialize, Serialize};

/// A complete palette configuration.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Palette {
    pub id: String,
    pub name: String,
    pub gradient: Gradient,
    pub transfer_curve: Curve,
    pub histogram_enabled: bool,
    pub smooth_enabled: bool,
    pub shading_enabled: bool,
    pub falloff_curve: Curve,
    pub lighting: LightingParams,
}

impl Default for Palette {
    fn default() -> Self {
        Self {
            id: "default".to_string(),
            name: "Default".to_string(),
            gradient: Gradient::new(vec![
                ColorStop {
                    position: 0.0,
                    color: [0, 0, 0],
                },
                ColorStop {
                    position: 1.0,
                    color: [255, 255, 255],
                },
            ]),
            transfer_curve: Curve::linear(),
            histogram_enabled: false,
            smooth_enabled: true,
            shading_enabled: false,
            falloff_curve: Curve::linear(),
            lighting: LightingParams::default(),
        }
    }
}

impl Palette {
    /// Generate a color lookup table from the gradient.
    pub fn to_lut(&self) -> Vec<[u8; 3]> {
        self.gradient.to_lut()
    }

    /// Apply the transfer curve to a normalized value.
    pub fn apply_transfer(&self, t: f64) -> f64 {
        self.transfer_curve.evaluate(t)
    }

    /// Apply the falloff curve to a distance value.
    pub fn apply_falloff(&self, t: f64) -> f64 {
        self.falloff_curve.evaluate(t)
    }

    /// Factory default palettes built into the binary.
    pub fn factory_defaults() -> Vec<Palette> {
        vec![
            Self::classic(),
            Self::fire(),
            Self::ocean(),
            Self::electric(),
            Self::grayscale(),
            Self::rainbow(),
            Self::neon(),
            Self::twilight(),
            Self::candy(),
            Self::inferno(),
            Self::aurora(),
        ]
    }

    fn classic() -> Self {
        Self {
            id: "classic".to_string(),
            name: "Classic".to_string(),
            gradient: Gradient::new(vec![
                ColorStop {
                    position: 0.0,
                    color: [0, 7, 100],
                },
                ColorStop {
                    position: 0.16,
                    color: [0, 2, 0],
                },
                ColorStop {
                    position: 0.33,
                    color: [0, 7, 100],
                },
                ColorStop {
                    position: 0.5,
                    color: [32, 107, 203],
                },
                ColorStop {
                    position: 0.66,
                    color: [255, 170, 0],
                },
                ColorStop {
                    position: 1.0,
                    color: [237, 255, 255],
                },
            ]),
            ..Self::default()
        }
    }

    fn fire() -> Self {
        Self {
            id: "fire".to_string(),
            name: "Fire".to_string(),
            gradient: Gradient::new(vec![
                ColorStop {
                    position: 0.0,
                    color: [0, 0, 0],
                },
                ColorStop {
                    position: 0.2,
                    color: [128, 0, 0],
                },
                ColorStop {
                    position: 0.4,
                    color: [255, 0, 0],
                },
                ColorStop {
                    position: 0.6,
                    color: [255, 128, 0],
                },
                ColorStop {
                    position: 0.8,
                    color: [255, 255, 0],
                },
                ColorStop {
                    position: 1.0,
                    color: [255, 255, 255],
                },
            ]),
            ..Self::default()
        }
    }

    fn ocean() -> Self {
        Self {
            id: "ocean".to_string(),
            name: "Ocean".to_string(),
            gradient: Gradient::new(vec![
                ColorStop {
                    position: 0.0,
                    color: [0, 0, 64],
                },
                ColorStop {
                    position: 0.25,
                    color: [0, 64, 128],
                },
                ColorStop {
                    position: 0.5,
                    color: [0, 128, 192],
                },
                ColorStop {
                    position: 0.75,
                    color: [64, 192, 255],
                },
                ColorStop {
                    position: 1.0,
                    color: [255, 255, 255],
                },
            ]),
            ..Self::default()
        }
    }

    fn electric() -> Self {
        Self {
            id: "electric".to_string(),
            name: "Electric".to_string(),
            gradient: Gradient::new(vec![
                ColorStop {
                    position: 0.0,
                    color: [32, 0, 64],
                },
                ColorStop {
                    position: 0.2,
                    color: [64, 0, 128],
                },
                ColorStop {
                    position: 0.4,
                    color: [0, 0, 255],
                },
                ColorStop {
                    position: 0.6,
                    color: [0, 255, 255],
                },
                ColorStop {
                    position: 0.8,
                    color: [0, 255, 0],
                },
                ColorStop {
                    position: 1.0,
                    color: [255, 255, 0],
                },
            ]),
            ..Self::default()
        }
    }

    fn grayscale() -> Self {
        Self {
            id: "grayscale".to_string(),
            name: "Grayscale".to_string(),
            gradient: Gradient::new(vec![
                ColorStop {
                    position: 0.0,
                    color: [0, 0, 0],
                },
                ColorStop {
                    position: 1.0,
                    color: [255, 255, 255],
                },
            ]),
            ..Self::default()
        }
    }

    fn rainbow() -> Self {
        Self {
            id: "rainbow".to_string(),
            name: "Rainbow".to_string(),
            gradient: Gradient::new(vec![
                ColorStop {
                    position: 0.0,
                    color: [255, 0, 0],
                },
                ColorStop {
                    position: 0.17,
                    color: [255, 127, 0],
                },
                ColorStop {
                    position: 0.33,
                    color: [255, 255, 0],
                },
                ColorStop {
                    position: 0.5,
                    color: [0, 255, 0],
                },
                ColorStop {
                    position: 0.67,
                    color: [0, 0, 255],
                },
                ColorStop {
                    position: 0.83,
                    color: [75, 0, 130],
                },
                ColorStop {
                    position: 1.0,
                    color: [148, 0, 211],
                },
            ]),
            ..Self::default()
        }
    }

    fn neon() -> Self {
        Self {
            id: "neon".to_string(),
            name: "Neon".to_string(),
            gradient: Gradient::new(vec![
                ColorStop {
                    position: 0.0,
                    color: [255, 0, 255],
                },
                ColorStop {
                    position: 0.33,
                    color: [0, 255, 255],
                },
                ColorStop {
                    position: 0.67,
                    color: [255, 255, 0],
                },
                ColorStop {
                    position: 1.0,
                    color: [255, 0, 255],
                },
            ]),
            ..Self::default()
        }
    }

    fn twilight() -> Self {
        Self {
            id: "twilight".to_string(),
            name: "Twilight".to_string(),
            gradient: Gradient::new(vec![
                ColorStop {
                    position: 0.0,
                    color: [255, 100, 50],
                },
                ColorStop {
                    position: 0.111,
                    color: [255, 50, 100],
                },
                ColorStop {
                    position: 0.222,
                    color: [200, 50, 150],
                },
                ColorStop {
                    position: 0.333,
                    color: [150, 50, 200],
                },
                ColorStop {
                    position: 0.444,
                    color: [80, 80, 220],
                },
                ColorStop {
                    position: 0.556,
                    color: [50, 150, 255],
                },
                ColorStop {
                    position: 0.667,
                    color: [80, 200, 200],
                },
                ColorStop {
                    position: 0.778,
                    color: [150, 200, 150],
                },
                ColorStop {
                    position: 0.889,
                    color: [200, 180, 100],
                },
                ColorStop {
                    position: 1.0,
                    color: [255, 100, 50],
                },
            ]),
            ..Self::default()
        }
    }

    fn candy() -> Self {
        Self {
            id: "candy".to_string(),
            name: "Candy".to_string(),
            gradient: Gradient::new(vec![
                ColorStop {
                    position: 0.0,
                    color: [255, 180, 200],
                },
                ColorStop {
                    position: 0.143,
                    color: [200, 180, 255],
                },
                ColorStop {
                    position: 0.286,
                    color: [180, 220, 255],
                },
                ColorStop {
                    position: 0.429,
                    color: [180, 255, 220],
                },
                ColorStop {
                    position: 0.571,
                    color: [220, 255, 180],
                },
                ColorStop {
                    position: 0.714,
                    color: [255, 240, 180],
                },
                ColorStop {
                    position: 0.857,
                    color: [255, 200, 180],
                },
                ColorStop {
                    position: 1.0,
                    color: [255, 180, 200],
                },
            ]),
            ..Self::default()
        }
    }

    fn inferno() -> Self {
        Self {
            id: "inferno".to_string(),
            name: "Inferno".to_string(),
            gradient: Gradient::new(vec![
                ColorStop {
                    position: 0.0,
                    color: [5, 0, 10],
                },
                ColorStop {
                    position: 0.04,
                    color: [200, 150, 100],
                },
                ColorStop {
                    position: 0.08,
                    color: [5, 0, 10],
                },
                ColorStop {
                    position: 0.12,
                    color: [200, 150, 100],
                },
                ColorStop {
                    position: 0.16,
                    color: [5, 0, 10],
                },
                ColorStop {
                    position: 0.20,
                    color: [200, 150, 100],
                },
                ColorStop {
                    position: 0.24,
                    color: [5, 0, 10],
                },
                ColorStop {
                    position: 0.28,
                    color: [200, 150, 100],
                },
                ColorStop {
                    position: 0.32,
                    color: [5, 0, 10],
                },
                ColorStop {
                    position: 0.36,
                    color: [200, 150, 100],
                },
                ColorStop {
                    position: 0.40,
                    color: [5, 0, 10],
                },
                ColorStop {
                    position: 0.44,
                    color: [200, 150, 100],
                },
                ColorStop {
                    position: 0.48,
                    color: [5, 0, 10],
                },
                ColorStop {
                    position: 0.52,
                    color: [200, 150, 100],
                },
                ColorStop {
                    position: 0.56,
                    color: [5, 0, 10],
                },
                ColorStop {
                    position: 0.60,
                    color: [200, 150, 100],
                },
                ColorStop {
                    position: 0.64,
                    color: [5, 0, 10],
                },
                ColorStop {
                    position: 0.68,
                    color: [200, 150, 100],
                },
                ColorStop {
                    position: 0.72,
                    color: [5, 0, 10],
                },
                ColorStop {
                    position: 0.76,
                    color: [40, 0, 20],
                },
                ColorStop {
                    position: 0.80,
                    color: [100, 10, 10],
                },
                ColorStop {
                    position: 0.84,
                    color: [180, 40, 0],
                },
                ColorStop {
                    position: 0.88,
                    color: [255, 100, 0],
                },
                ColorStop {
                    position: 0.92,
                    color: [255, 180, 50],
                },
                ColorStop {
                    position: 0.96,
                    color: [200, 150, 100],
                },
                ColorStop {
                    position: 1.0,
                    color: [255, 255, 255],
                },
            ]),
            ..Self::default()
        }
    }

    fn aurora() -> Self {
        Self {
            id: "aurora".to_string(),
            name: "Aurora".to_string(),
            gradient: Gradient::new(vec![
                ColorStop {
                    position: 0.0,
                    color: [50, 255, 100],
                },
                ColorStop {
                    position: 0.125,
                    color: [50, 255, 180],
                },
                ColorStop {
                    position: 0.25,
                    color: [50, 200, 255],
                },
                ColorStop {
                    position: 0.375,
                    color: [80, 120, 255],
                },
                ColorStop {
                    position: 0.5,
                    color: [150, 80, 255],
                },
                ColorStop {
                    position: 0.625,
                    color: [200, 100, 200],
                },
                ColorStop {
                    position: 0.75,
                    color: [150, 150, 150],
                },
                ColorStop {
                    position: 0.875,
                    color: [100, 200, 100],
                },
                ColorStop {
                    position: 1.0,
                    color: [50, 255, 100],
                },
            ]),
            ..Self::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn palette_default_has_valid_gradient() {
        let palette = Palette::default();
        assert!(palette.gradient.stops.len() >= 2);
    }

    #[test]
    fn palette_to_lut_returns_4096_entries() {
        let palette = Palette::default();
        let lut = palette.to_lut();
        assert_eq!(lut.len(), 4096);
    }

    #[test]
    fn palette_transfer_curve_identity() {
        let palette = Palette::default();
        assert!((palette.apply_transfer(0.0) - 0.0).abs() < 0.001);
        assert!((palette.apply_transfer(0.5) - 0.5).abs() < 0.001);
        assert!((palette.apply_transfer(1.0) - 1.0).abs() < 0.001);
    }

    #[test]
    fn palette_serialization() {
        let palette = Palette::default();
        let json = serde_json::to_string(&palette).unwrap();
        let parsed: Palette = serde_json::from_str(&json).unwrap();
        assert_eq!(palette.id, parsed.id);
        assert_eq!(palette.name, parsed.name);
    }

    #[test]
    fn factory_defaults_contains_classic() {
        let palettes = Palette::factory_defaults();
        assert!(palettes.iter().any(|p| p.id == "classic"));
    }

    #[test]
    fn factory_defaults_all_have_unique_ids() {
        let palettes = Palette::factory_defaults();
        let mut ids: Vec<_> = palettes.iter().map(|p| &p.id).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), palettes.len());
    }
}
