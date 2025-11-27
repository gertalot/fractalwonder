use fractalwonder_core::MandelbrotData;

/// Grayscale colorizer for Mandelbrot data.
///
/// - Glitched pixels (glitched=true) are rendered in cyan with brightness based on iteration count.
/// - Points in the set (escaped=false) are black.
/// - Escaped points get grayscale based on normalized iteration count.
pub fn colorize(data: &MandelbrotData) -> [u8; 4] {
    // Glitched pixels get cyan overlay with brightness based on iteration count
    if data.glitched {
        if data.max_iterations == 0 {
            return [0, 255, 255, 255]; // Bright cyan if no max_iterations
        }
        // Normalize iterations to 0.0..1.0, then scale to a visible cyan range
        let normalized = data.iterations as f64 / data.max_iterations as f64;
        // Use range [64, 255] so even low iterations are visible
        let brightness = (64.0 + normalized * 191.0) as u8;
        return [0, brightness, brightness, 255]; // Cyan (no red, equal green+blue)
    }

    if !data.escaped {
        // In the set = black
        return [0, 0, 0, 255];
    }

    if data.max_iterations == 0 {
        return [0, 0, 0, 255];
    }

    // Normalize iterations to 0.0..1.0
    let normalized = data.iterations as f64 / data.max_iterations as f64;
    let gray = (normalized * 255.0) as u8;

    [gray, gray, gray, 255]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn in_set_is_black() {
        let data = MandelbrotData {
            iterations: 1000,
            max_iterations: 1000,
            escaped: false,
            glitched: false,
        };
        assert_eq!(colorize(&data), [0, 0, 0, 255]);
    }

    #[test]
    fn escaped_at_zero_is_black() {
        let data = MandelbrotData {
            iterations: 0,
            max_iterations: 1000,
            escaped: true,
            glitched: false,
        };
        assert_eq!(colorize(&data), [0, 0, 0, 255]);
    }

    #[test]
    fn escaped_at_max_is_white() {
        let data = MandelbrotData {
            iterations: 1000,
            max_iterations: 1000,
            escaped: true,
            glitched: false,
        };
        assert_eq!(colorize(&data), [255, 255, 255, 255]);
    }

    #[test]
    fn escaped_halfway_is_gray() {
        let data = MandelbrotData {
            iterations: 500,
            max_iterations: 1000,
            escaped: true,
            glitched: false,
        };
        let result = colorize(&data);
        // 500/1000 * 255 = 127.5 -> 127
        assert_eq!(result, [127, 127, 127, 255]);
    }

    #[test]
    fn handles_zero_max_iterations() {
        let data = MandelbrotData {
            iterations: 0,
            max_iterations: 0,
            escaped: true,
            glitched: false,
        };
        assert_eq!(colorize(&data), [0, 0, 0, 255]);
    }

    #[test]
    fn glitched_pixel_is_cyan() {
        let data = MandelbrotData {
            iterations: 500,
            max_iterations: 1000,
            escaped: false,
            glitched: true,
        };
        let color = colorize(&data);
        // Cyan: red=0, green=blue
        assert_eq!(color[0], 0, "Cyan should have no red component");
        assert_eq!(color[1], color[2], "Cyan should have equal green and blue");
        assert!(color[1] > 0, "Cyan should have positive green/blue");
        assert_eq!(color[3], 255, "Alpha should be opaque");
    }

    #[test]
    fn glitched_brightness_varies_with_iterations() {
        let low_iter = MandelbrotData {
            iterations: 100,
            max_iterations: 1000,
            escaped: false,
            glitched: true,
        };
        let high_iter = MandelbrotData {
            iterations: 900,
            max_iterations: 1000,
            escaped: false,
            glitched: true,
        };

        let low_color = colorize(&low_iter);
        let high_color = colorize(&high_iter);

        // Higher iterations should produce brighter cyan
        assert!(
            high_color[1] > low_color[1],
            "Higher iterations should be brighter: {} vs {}",
            high_color[1],
            low_color[1]
        );
    }

    #[test]
    fn glitched_with_zero_iterations_still_visible() {
        let data = MandelbrotData {
            iterations: 0,
            max_iterations: 1000,
            escaped: false,
            glitched: true,
        };
        let color = colorize(&data);
        // Should still be visible (minimum brightness of 64)
        assert!(
            color[1] >= 64,
            "Glitched at zero iterations should still be visible: {}",
            color[1]
        );
    }

    #[test]
    fn glitched_with_zero_max_iterations() {
        let data = MandelbrotData {
            iterations: 0,
            max_iterations: 0,
            escaped: true,
            glitched: true,
        };
        let color = colorize(&data);
        // Should be bright cyan
        assert_eq!(color, [0, 255, 255, 255]);
    }

    #[test]
    fn glitched_overrides_in_set() {
        // Glitched takes precedence over in-set (escaped=false)
        let data = MandelbrotData {
            iterations: 500,
            max_iterations: 1000,
            escaped: false,
            glitched: true,
        };
        let color = colorize(&data);
        // Should be cyan, not black
        assert_eq!(color[0], 0);
        assert!(color[1] > 0, "Should be cyan, not black");
    }

    #[test]
    fn glitched_overrides_escaped() {
        // Glitched takes precedence over escaped
        let data = MandelbrotData {
            iterations: 500,
            max_iterations: 1000,
            escaped: true,
            glitched: true,
        };
        let color = colorize(&data);
        // Should be cyan, not gray
        assert_eq!(color[0], 0, "Should be cyan (no red), not gray");
        assert_eq!(color[1], color[2], "Should be cyan (equal green/blue)");
    }
}
