//! OKLAB color space conversions for perceptually uniform palette interpolation.

/// Convert sRGB component [0,1] to linear RGB (remove gamma).
pub fn srgb_to_linear(c: f64) -> f64 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

/// Convert linear RGB component to sRGB [0,1] (apply gamma).
pub fn linear_to_srgb(c: f64) -> f64 {
    if c <= 0.0031308 {
        c * 12.92
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    }
}

/// Convert linear RGB to OKLAB (L, a, b).
/// L is perceptual lightness [0,1], a is green-red, b is blue-yellow.
pub fn linear_rgb_to_oklab(r: f64, g: f64, b: f64) -> (f64, f64, f64) {
    // RGB to LMS cone responses
    let l = 0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b;
    let m = 0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b;
    let s = 0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b;

    // Cube root (perceptual non-linearity)
    let l_ = l.cbrt();
    let m_ = m.cbrt();
    let s_ = s.cbrt();

    // LMS to OKLAB
    let lab_l = 0.2104542553 * l_ + 0.7936177850 * m_ - 0.0040720468 * s_;
    let lab_a = 1.9779984951 * l_ - 2.4285922050 * m_ + 0.4505937099 * s_;
    let lab_b = 0.0259040371 * l_ + 0.7827717662 * m_ - 0.8086757660 * s_;

    (lab_l, lab_a, lab_b)
}

/// Convert OKLAB to linear RGB.
pub fn oklab_to_linear_rgb(l: f64, a: f64, b: f64) -> (f64, f64, f64) {
    // OKLAB to LMS (cube-root space)
    let l_ = l + 0.3963377774 * a + 0.2158037573 * b;
    let m_ = l - 0.1055613458 * a - 0.0638541728 * b;
    let s_ = l - 0.0894841775 * a - 1.2914855480 * b;

    // Cube to undo perceptual non-linearity
    let lms_l = l_ * l_ * l_;
    let lms_m = m_ * m_ * m_;
    let lms_s = s_ * s_ * s_;

    // LMS to linear RGB
    let r = 4.0767416621 * lms_l - 3.3077115913 * lms_m + 0.2309699292 * lms_s;
    let g = -1.2684380046 * lms_l + 2.6097574011 * lms_m - 0.3413193965 * lms_s;
    let b = -0.0041960863 * lms_l - 0.7034186147 * lms_m + 1.7076147010 * lms_s;

    (r.clamp(0.0, 1.0), g.clamp(0.0, 1.0), b.clamp(0.0, 1.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn srgb_to_linear_black() {
        assert!((srgb_to_linear(0.0) - 0.0).abs() < 1e-6);
    }

    #[test]
    fn srgb_to_linear_white() {
        assert!((srgb_to_linear(1.0) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn srgb_to_linear_mid_gray() {
        // sRGB 0.5 ≈ linear 0.214
        let result = srgb_to_linear(0.5);
        assert!((result - 0.214).abs() < 0.01);
    }

    #[test]
    fn linear_to_srgb_roundtrip() {
        for i in 0..=10 {
            let original = i as f64 / 10.0;
            let roundtrip = linear_to_srgb(srgb_to_linear(original));
            assert!((original - roundtrip).abs() < 1e-6, "Failed at {original}");
        }
    }

    #[test]
    fn oklab_white() {
        let (l, a, b) = linear_rgb_to_oklab(1.0, 1.0, 1.0);
        assert!((l - 1.0).abs() < 0.01, "L should be ~1.0, got {l}");
        assert!(a.abs() < 0.01, "a should be ~0, got {a}");
        assert!(b.abs() < 0.01, "b should be ~0, got {b}");
    }

    #[test]
    fn oklab_black() {
        let (l, a, b) = linear_rgb_to_oklab(0.0, 0.0, 0.0);
        assert!(l.abs() < 0.01, "L should be ~0, got {l}");
        assert!(a.abs() < 0.01, "a should be ~0, got {a}");
        assert!(b.abs() < 0.01, "b should be ~0, got {b}");
    }

    #[test]
    fn oklab_roundtrip() {
        let test_colors = [
            (1.0, 0.0, 0.0), // Red
            (0.0, 1.0, 0.0), // Green
            (0.0, 0.0, 1.0), // Blue
            (0.5, 0.5, 0.5), // Gray
        ];
        for (r, g, b) in test_colors {
            let (l, a, ob) = linear_rgb_to_oklab(r, g, b);
            let (r2, g2, b2) = oklab_to_linear_rgb(l, a, ob);
            assert!((r - r2).abs() < 1e-4, "R mismatch for ({r},{g},{b})");
            assert!((g - g2).abs() < 1e-4, "G mismatch for ({r},{g},{b})");
            assert!((b - b2).abs() < 1e-4, "B mismatch for ({r},{g},{b})");
        }
    }
}
