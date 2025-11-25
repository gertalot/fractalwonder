use fractalwonder_core::MandelbrotData;

/// Grayscale colorizer for Mandelbrot data.
///
/// Points in the set (escaped=false) are black.
/// Escaped points get grayscale based on normalized iteration count.
pub fn colorize(data: &MandelbrotData) -> [u8; 4] {
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
        };
        assert_eq!(colorize(&data), [0, 0, 0, 255]);
    }

    #[test]
    fn escaped_at_zero_is_black() {
        let data = MandelbrotData {
            iterations: 0,
            max_iterations: 1000,
            escaped: true,
        };
        assert_eq!(colorize(&data), [0, 0, 0, 255]);
    }

    #[test]
    fn escaped_at_max_is_white() {
        let data = MandelbrotData {
            iterations: 1000,
            max_iterations: 1000,
            escaped: true,
        };
        assert_eq!(colorize(&data), [255, 255, 255, 255]);
    }

    #[test]
    fn escaped_halfway_is_gray() {
        let data = MandelbrotData {
            iterations: 500,
            max_iterations: 1000,
            escaped: true,
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
        };
        assert_eq!(colorize(&data), [0, 0, 0, 255]);
    }
}
