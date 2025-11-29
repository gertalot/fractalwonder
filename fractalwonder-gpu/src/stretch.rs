// fractalwonder-gpu/src/stretch.rs

use fractalwonder_core::ComputeData;

/// Stretches a small ComputeData buffer to full canvas size by duplicating pixels.
///
/// Each source pixel becomes a `scale × scale` block in the output.
/// Output is in row-major order, suitable for colorization.
pub fn stretch_compute_data(
    small: &[ComputeData],
    small_w: u32,
    small_h: u32,
    scale: u32,
) -> Vec<ComputeData> {
    debug_assert_eq!(
        small.len(),
        (small_w * small_h) as usize,
        "Input size mismatch"
    );

    if scale == 1 {
        return small.to_vec();
    }

    let full_w = small_w * scale;
    let full_h = small_h * scale;
    let mut full = Vec::with_capacity((full_w * full_h) as usize);

    for sy in 0..small_h {
        // For each row in the small image, we output `scale` rows
        for _dy in 0..scale {
            for sx in 0..small_w {
                let src = &small[(sy * small_w + sx) as usize];
                // Duplicate this pixel `scale` times horizontally
                for _dx in 0..scale {
                    full.push(src.clone());
                }
            }
        }
    }

    full
}

#[cfg(test)]
mod tests {
    use super::*;
    use fractalwonder_core::MandelbrotData;

    fn make_data(iterations: u32) -> ComputeData {
        ComputeData::Mandelbrot(MandelbrotData {
            iterations,
            max_iterations: 1000,
            escaped: iterations < 1000,
            glitched: false,
        })
    }

    #[test]
    fn test_stretch_scale_1() {
        let small = vec![make_data(10), make_data(20), make_data(30), make_data(40)];
        let result = stretch_compute_data(&small, 2, 2, 1);
        assert_eq!(result.len(), 4);

        // Verify each element matches
        let expected_iters: Vec<u32> = vec![10, 20, 30, 40];
        let actual_iters: Vec<u32> = result
            .iter()
            .map(|d| match d {
                ComputeData::Mandelbrot(m) => m.iterations,
                _ => panic!("Expected Mandelbrot"),
            })
            .collect();
        assert_eq!(actual_iters, expected_iters);
    }

    #[test]
    fn test_stretch_scale_2() {
        // 2x2 input, scale 2 -> 4x4 output
        let small = vec![make_data(1), make_data(2), make_data(3), make_data(4)];
        let result = stretch_compute_data(&small, 2, 2, 2);
        assert_eq!(result.len(), 16);

        // Expected layout:
        // 1 1 2 2
        // 1 1 2 2
        // 3 3 4 4
        // 3 3 4 4
        let expected_iters: Vec<u32> = vec![
            1, 1, 2, 2, // row 0
            1, 1, 2, 2, // row 1
            3, 3, 4, 4, // row 2
            3, 3, 4, 4, // row 3
        ];

        let actual_iters: Vec<u32> = result
            .iter()
            .map(|d| match d {
                ComputeData::Mandelbrot(m) => m.iterations,
                _ => panic!("Expected Mandelbrot"),
            })
            .collect();

        assert_eq!(actual_iters, expected_iters);
    }

    #[test]
    fn test_stretch_scale_16() {
        // 1x1 input, scale 16 -> 16x16 output
        let small = vec![make_data(42)];
        let result = stretch_compute_data(&small, 1, 1, 16);
        assert_eq!(result.len(), 256);

        // All pixels should have iterations = 42
        for d in &result {
            match d {
                ComputeData::Mandelbrot(m) => assert_eq!(m.iterations, 42),
                _ => panic!("Expected Mandelbrot"),
            }
        }
    }

    #[test]
    fn test_stretch_preserves_glitch_flag() {
        let small = vec![ComputeData::Mandelbrot(MandelbrotData {
            iterations: 100,
            max_iterations: 1000,
            escaped: true,
            glitched: true,
        })];
        let result = stretch_compute_data(&small, 1, 1, 4);
        assert_eq!(result.len(), 16);

        for d in &result {
            match d {
                ComputeData::Mandelbrot(m) => {
                    assert!(m.glitched);
                    assert!(m.escaped);
                }
                _ => panic!("Expected Mandelbrot"),
            }
        }
    }
}
