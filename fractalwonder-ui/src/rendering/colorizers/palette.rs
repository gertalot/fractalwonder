//! Unified Palette struct containing gradient, curves, lighting, and flags.

use super::{ColorStop, Curve, Gradient, LightingParams};
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsValue;
#[cfg(target_arch = "wasm32")]
use web_sys::window;

/// Cached factory palettes (loaded from /assets/factory_palettes.json).
static FACTORY_PALETTES: OnceLock<Vec<Palette>> = OnceLock::new();

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

    /// Factory default palettes.
    ///
    /// Returns cached palettes. Must call `load_factory_defaults()` first.
    pub fn factory_defaults() -> Vec<Palette> {
        FACTORY_PALETTES
            .get()
            .cloned()
            .expect("factory_defaults() called before load_factory_defaults()")
    }

    /// Load factory palettes from static asset (WASM only).
    ///
    /// Fetches `/fractalwonder/assets/factory_palettes.json` and caches the result.
    #[cfg(target_arch = "wasm32")]
    pub async fn load_factory_defaults() -> Result<(), String> {
        use wasm_bindgen::JsCast;
        use wasm_bindgen_futures::JsFuture;

        // Already loaded?
        if FACTORY_PALETTES.get().is_some() {
            return Ok(());
        }

        let window = web_sys::window().ok_or("no window")?;
        let url = "/fractalwonder/assets/factory_palettes.json";

        let resp_value = JsFuture::from(window.fetch_with_str(url))
            .await
            .map_err(|e| format!("fetch error: {:?}", e))?;

        let resp: web_sys::Response = resp_value.dyn_into().map_err(|_| "response cast error")?;

        if !resp.ok() {
            return Err(format!("HTTP {} fetching {}", resp.status(), url));
        }

        let json_value = JsFuture::from(resp.text().map_err(|_| "text() error")?)
            .await
            .map_err(|e| format!("text error: {:?}", e))?;

        let json_str = json_value.as_string().ok_or("response not a string")?;

        let palettes: Vec<Palette> =
            serde_json::from_str(&json_str).map_err(|e| format!("JSON parse error: {}", e))?;

        let _ = FACTORY_PALETTES.set(palettes);
        Ok(())
    }

    /// Non-WASM stub for load_factory_defaults (no-op, filesystem loading happens lazily).
    #[cfg(not(target_arch = "wasm32"))]
    pub async fn load_factory_defaults() -> Result<(), String> {
        Ok(())
    }

    /// Save palette to localStorage.
    #[cfg(target_arch = "wasm32")]
    pub fn save(&self) -> Result<(), JsValue> {
        let storage = window()
            .ok_or("no window")?
            .local_storage()
            .map_err(|_| "localStorage error")?
            .ok_or("no localStorage")?;

        let json = serde_json::to_string(self).map_err(|e| e.to_string())?;
        storage.set_item(&format!("palette:{}", self.id), &json)
    }

    /// Load palette from localStorage.
    #[cfg(target_arch = "wasm32")]
    pub fn load(id: &str) -> Option<Self> {
        let storage = window()?.local_storage().ok()??;
        let json = storage.get_item(&format!("palette:{id}")).ok()??;
        serde_json::from_str(&json).ok()
    }

    /// Delete palette from localStorage.
    #[cfg(target_arch = "wasm32")]
    pub fn delete(id: &str) {
        if let Some(storage) = window().and_then(|w| w.local_storage().ok().flatten()) {
            let _ = storage.remove_item(&format!("palette:{id}"));
        }
    }

    /// Get palette by ID: localStorage first, then factory default.
    #[cfg(target_arch = "wasm32")]
    pub fn get(id: &str) -> Option<Self> {
        Self::load(id).or_else(|| Self::factory_defaults().into_iter().find(|p| p.id == id))
    }

    /// Non-WASM stubs for testing.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn save(&self) -> Result<(), String> {
        Ok(()) // No-op in tests
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn load(_id: &str) -> Option<Self> {
        None // No localStorage in tests
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn delete(_id: &str) {
        // No-op in tests
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn get(id: &str) -> Option<Self> {
        Self::factory_defaults().into_iter().find(|p| p.id == id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Load factory palettes from filesystem for tests.
    fn init_factory_palettes() {
        let _ = FACTORY_PALETTES.get_or_init(|| {
            let path = concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../assets/factory_palettes.json"
            );
            let json =
                std::fs::read_to_string(path).expect("failed to read assets/factory_palettes.json");
            serde_json::from_str(&json).expect("invalid factory_palettes.json")
        });
    }

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
    #[ignore] // Run with: cargo test print_factory_json -- --ignored --nocapture
    fn print_factory_json() {
        init_factory_palettes();
        let palettes = Palette::factory_defaults();
        let json = serde_json::to_string_pretty(&palettes).unwrap();
        println!("\n{}", json);
    }

    #[test]
    fn factory_defaults_contains_classic() {
        init_factory_palettes();
        let palettes = Palette::factory_defaults();
        assert!(palettes.iter().any(|p| p.id == "classic"));
    }

    #[test]
    fn factory_defaults_all_have_unique_ids() {
        init_factory_palettes();
        let palettes = Palette::factory_defaults();
        let mut ids: Vec<_> = palettes.iter().map(|p| &p.id).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), palettes.len());
    }

    #[test]
    fn palette_get_returns_factory_default() {
        init_factory_palettes();
        let palette = Palette::get("classic");
        assert!(palette.is_some());
        assert_eq!(palette.unwrap().id, "classic");
    }

    #[test]
    fn palette_get_returns_none_for_unknown() {
        init_factory_palettes();
        let palette = Palette::get("nonexistent");
        assert!(palette.is_none());
    }
}
