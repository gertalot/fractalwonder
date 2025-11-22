use crate::BigFloat;
use serde::{Deserialize, Serialize};

/// Viewport in fractal space with BigFloat precision
///
/// Defines a rectangular region in fractal coordinates:
/// - `center`: Center point (x, y) in fractal space
/// - `width`: Visible width in fractal space
/// - `height`: Visible height in fractal space
///
/// At extreme zoom depths (10^2000), width/height are ~10^-2000.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Viewport {
    pub center: (BigFloat, BigFloat),
    pub width: BigFloat,
    pub height: BigFloat,
}

impl Viewport {
    /// Create new viewport with BigFloat coordinates
    ///
    /// This is the primary constructor that preserves full precision.
    pub fn with_bigfloat(
        center_x: BigFloat,
        center_y: BigFloat,
        width: BigFloat,
        height: BigFloat,
    ) -> Self {
        Self {
            center: (center_x, center_y),
            width,
            height,
        }
    }

    /// Create new viewport from f64 values with explicit precision
    ///
    /// Use this for initial viewport creation or when f64 precision is sufficient.
    /// For extreme depths, use `with_bigfloat` instead.
    pub fn from_f64(
        center_x: f64,
        center_y: f64,
        width: f64,
        height: f64,
        precision_bits: usize,
    ) -> Self {
        Self {
            center: (
                BigFloat::with_precision(center_x, precision_bits),
                BigFloat::with_precision(center_y, precision_bits),
            ),
            width: BigFloat::with_precision(width, precision_bits),
            height: BigFloat::with_precision(height, precision_bits),
        }
    }

    /// Create viewport from string representations (for extreme precision coordinates)
    ///
    /// Use this when loading saved positions with coordinates that exceed f64 precision.
    /// Returns an error if any string cannot be parsed.
    pub fn from_strings(
        center_x: &str,
        center_y: &str,
        width: &str,
        height: &str,
        precision_bits: usize,
    ) -> Result<Self, String> {
        Ok(Self {
            center: (
                BigFloat::from_string(center_x, precision_bits)?,
                BigFloat::from_string(center_y, precision_bits)?,
            ),
            width: BigFloat::from_string(width, precision_bits)?,
            height: BigFloat::from_string(height, precision_bits)?,
        })
    }

    /// Get the precision bits of this viewport
    pub fn precision_bits(&self) -> usize {
        self.width.precision_bits()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ============================================================================
    // with_bigfloat() constructor tests
    // ============================================================================

    #[test]
    fn with_bigfloat_stores_center_coordinates() {
        let center_x = BigFloat::with_precision(-0.5, 256);
        let center_y = BigFloat::with_precision(0.3, 256);
        let width = BigFloat::with_precision(4.0, 256);
        let height = BigFloat::with_precision(3.0, 256);

        let viewport = Viewport::with_bigfloat(center_x.clone(), center_y.clone(), width, height);

        assert_eq!(viewport.center.0, center_x);
        assert_eq!(viewport.center.1, center_y);
    }

    #[test]
    fn with_bigfloat_stores_width_and_height() {
        let center_x = BigFloat::zero(256);
        let center_y = BigFloat::zero(256);
        let width = BigFloat::with_precision(4.0, 256);
        let height = BigFloat::with_precision(3.0, 256);

        let viewport = Viewport::with_bigfloat(center_x, center_y, width.clone(), height.clone());

        assert_eq!(viewport.width, width);
        assert_eq!(viewport.height, height);
    }

    #[test]
    fn with_bigfloat_preserves_precision_metadata() {
        let center_x = BigFloat::with_precision(0.0, 512);
        let center_y = BigFloat::with_precision(0.0, 512);
        let width = BigFloat::with_precision(4.0, 512);
        let height = BigFloat::with_precision(3.0, 512);

        let viewport = Viewport::with_bigfloat(center_x, center_y, width, height);

        assert_eq!(viewport.center.0.precision_bits(), 512);
        assert_eq!(viewport.center.1.precision_bits(), 512);
        assert_eq!(viewport.width.precision_bits(), 512);
        assert_eq!(viewport.height.precision_bits(), 512);
    }

    // ============================================================================
    // from_f64() constructor tests
    // ============================================================================

    #[test]
    fn from_f64_creates_equivalent_bigfloat_values() {
        let viewport = Viewport::from_f64(-0.5, 0.3, 4.0, 3.0, 128);

        let expected_x = BigFloat::with_precision(-0.5, 128);
        let expected_y = BigFloat::with_precision(0.3, 128);
        let expected_width = BigFloat::with_precision(4.0, 128);
        let expected_height = BigFloat::with_precision(3.0, 128);

        assert_eq!(viewport.center.0, expected_x);
        assert_eq!(viewport.center.1, expected_y);
        assert_eq!(viewport.width, expected_width);
        assert_eq!(viewport.height, expected_height);
    }

    #[test]
    fn from_f64_sets_requested_precision() {
        let viewport = Viewport::from_f64(0.0, 0.0, 4.0, 3.0, 7000);

        assert_eq!(viewport.center.0.precision_bits(), 7000);
        assert_eq!(viewport.center.1.precision_bits(), 7000);
        assert_eq!(viewport.width.precision_bits(), 7000);
        assert_eq!(viewport.height.precision_bits(), 7000);
    }

    // ============================================================================
    // from_strings() constructor tests
    // ============================================================================

    #[test]
    fn from_strings_parses_coordinates_correctly() {
        let viewport = Viewport::from_strings("-0.5", "0.25", "4.0", "3.0", 256).unwrap();

        let expected_x = BigFloat::from_string("-0.5", 256).unwrap();
        let expected_y = BigFloat::from_string("0.25", 256).unwrap();
        let expected_width = BigFloat::from_string("4.0", 256).unwrap();
        let expected_height = BigFloat::from_string("3.0", 256).unwrap();

        assert_eq!(viewport.center.0, expected_x);
        assert_eq!(viewport.center.1, expected_y);
        assert_eq!(viewport.width, expected_width);
        assert_eq!(viewport.height, expected_height);
    }

    #[test]
    fn from_strings_handles_extreme_coordinates() {
        // Coordinates at extreme precision that cannot be represented in f64
        let viewport = Viewport::from_strings(
            "-0.743643887037158704752191506114774",
            "0.131825904205311970493132056385139",
            "1e-2000",
            "7.5e-2001",
            7000,
        )
        .unwrap();

        // Verify precision is preserved
        assert_eq!(viewport.precision_bits(), 7000);

        // Verify the coordinates are in expected ranges using BigFloat comparison
        let neg_one = BigFloat::with_precision(-1.0, 7000);
        let zero = BigFloat::zero(7000);
        let one = BigFloat::with_precision(1.0, 7000);

        assert!(viewport.center.0 > neg_one); // > -1
        assert!(viewport.center.0 < zero); // < 0 (it's negative)
        assert!(viewport.center.1 > zero); // > 0
        assert!(viewport.center.1 < one); // < 1

        // Verify width is extremely small (deep zoom)
        let small_threshold = BigFloat::from_string("1e-100", 7000).unwrap();
        assert!(viewport.width < small_threshold);
    }

    #[test]
    fn from_strings_returns_error_on_invalid_input() {
        let result = Viewport::from_strings("not_a_number", "0.0", "4.0", "3.0", 128);
        assert!(result.is_err());
    }

    // ============================================================================
    // Extreme depth tests
    // ============================================================================

    #[test]
    fn viewport_supports_width_beyond_f64_range() {
        // Width at 10^-500 (well beyond f64 min of ~10^-308)
        let width = BigFloat::from_string("1e-500", 7000).unwrap();
        let height = BigFloat::from_string("7.5e-501", 7000).unwrap();
        let viewport = Viewport::with_bigfloat(
            BigFloat::zero(7000),
            BigFloat::zero(7000),
            width.clone(),
            height.clone(),
        );

        assert_eq!(viewport.width, width);
        assert_eq!(viewport.height, height);

        // Verify we can do arithmetic with it (zoom in 2x = halve width)
        let two = BigFloat::with_precision(2.0, 7000);
        let zoomed_width = viewport.width.div(&two);
        let expected = BigFloat::from_string("5e-501", 7000).unwrap();
        assert_eq!(zoomed_width, expected);
    }

    #[test]
    fn viewport_supports_tiny_visible_region() {
        // At extreme depth, width/height are ~10^-2000
        let tiny_width = BigFloat::from_string("1e-2000", 7000).unwrap();
        let tiny_height = BigFloat::from_string("7.5e-2001", 7000).unwrap();
        let tiny_offset = BigFloat::from_string("1e-2000", 7000).unwrap();

        let viewport = Viewport::with_bigfloat(
            tiny_offset.clone(),
            BigFloat::zero(7000),
            tiny_width.clone(),
            tiny_height,
        );

        assert_eq!(viewport.center.0, tiny_offset);
        assert_eq!(viewport.width, tiny_width);
        assert!(viewport.center.0 > BigFloat::zero(7000));
    }

    // ============================================================================
    // Serialization round-trip tests
    // ============================================================================

    #[test]
    fn serialization_roundtrip_preserves_normal_values() {
        let original = Viewport::from_f64(-0.5, 0.3, 4.0, 3.0, 256);

        let json = serde_json::to_string(&original).unwrap();
        let restored: Viewport = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.center.0, original.center.0);
        assert_eq!(restored.center.1, original.center.1);
        assert_eq!(restored.width, original.width);
        assert_eq!(restored.height, original.height);
        assert_eq!(restored.precision_bits(), 256);
    }

    #[test]
    fn serialization_roundtrip_preserves_extreme_values() {
        let original = Viewport::from_strings(
            "-0.743643887037158704752191506114774",
            "0.131825904205311970493132056385139",
            "1e-2000",
            "7.5e-2001",
            7000,
        )
        .unwrap();

        let json = serde_json::to_string(&original).unwrap();
        let restored: Viewport = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.center.0, original.center.0);
        assert_eq!(restored.center.1, original.center.1);
        assert_eq!(restored.width, original.width);
        assert_eq!(restored.height, original.height);
        assert_eq!(restored.precision_bits(), 7000);
    }

    // ============================================================================
    // precision_bits() accessor tests
    // ============================================================================

    #[test]
    fn precision_bits_returns_width_precision() {
        // precision_bits() delegates to width.precision_bits()
        let viewport = Viewport::with_bigfloat(
            BigFloat::with_precision(0.0, 128),
            BigFloat::with_precision(0.0, 256),
            BigFloat::with_precision(4.0, 512), // this is what precision_bits() returns
            BigFloat::with_precision(3.0, 1024),
        );

        assert_eq!(viewport.precision_bits(), 512);
    }

    // ============================================================================
    // Mixed precision tests
    // ============================================================================

    #[test]
    fn viewport_allows_mixed_precision_components() {
        let viewport = Viewport::with_bigfloat(
            BigFloat::with_precision(0.0, 64),
            BigFloat::with_precision(0.0, 128),
            BigFloat::with_precision(4.0, 256),
            BigFloat::with_precision(3.0, 512),
        );

        assert_eq!(viewport.center.0.precision_bits(), 64);
        assert_eq!(viewport.center.1.precision_bits(), 128);
        assert_eq!(viewport.width.precision_bits(), 256);
        assert_eq!(viewport.height.precision_bits(), 512);
    }
}
