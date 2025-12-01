// fractalwonder-ui/src/rendering/adam7.rs

use fractalwonder_core::{ComputeData, MandelbrotData};

/// Sentinel value indicating a pixel was not computed in the current Adam7 pass.
pub const SENTINEL_NOT_COMPUTED: u32 = 0xFFFFFFFF;

/// Adam7 progressive rendering pass (1-7).
///
/// Replaces the old resolution-based Pass system. Each pass computes a subset
/// of pixels at full resolution, with each pass doubling the pixel count.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Adam7Pass(u8);

impl Adam7Pass {
    /// Create a new Adam7Pass. Panics if step is not 0-7.
    /// Step 0 means "compute all pixels" (no Adam7 interlacing).
    /// Steps 1-7 are the Adam7 interlacing passes.
    pub fn new(step: u8) -> Self {
        assert!(step <= 7, "Adam7 step must be 0-7, got {step}");
        Self(step)
    }

    /// Create an Adam7Pass that computes all pixels (no interlacing).
    pub fn all_pixels() -> Self {
        Self(0)
    }

    /// Returns all 7 passes in order.
    pub fn all() -> [Adam7Pass; 7] {
        [1, 2, 3, 4, 5, 6, 7].map(Adam7Pass)
    }

    /// Returns the step number (1-7).
    pub fn step(&self) -> u8 {
        self.0
    }

    /// Returns true if this is the final pass (step 7).
    pub fn is_final(&self) -> bool {
        self.0 == 7
    }

    /// Cumulative pixel percentage after this pass completes.
    pub fn cumulative_percent(&self) -> f32 {
        match self.0 {
            1 => 1.5625,
            2 => 3.125,
            3 => 6.25,
            4 => 12.5,
            5 => 25.0,
            6 => 50.0,
            7 => 100.0,
            _ => 0.0,
        }
    }

    /// Pixels computed in this pass as a fraction (for progress display).
    pub fn pass_fraction(&self) -> f32 {
        match self.0 {
            1 => 1.0 / 64.0,
            2 => 1.0 / 64.0,
            3 => 2.0 / 64.0,
            4 => 4.0 / 64.0,
            5 => 8.0 / 64.0,
            6 => 16.0 / 64.0,
            7 => 32.0 / 64.0,
            _ => 0.0,
        }
    }
}

/// Accumulator for Adam7 progressive rendering.
///
/// Collects results across multiple Adam7 passes, merging each pass's computed
/// pixels into a full-resolution buffer.
pub struct Adam7Accumulator {
    data: Vec<Option<ComputeData>>,
    width: u32,
    #[allow(dead_code)]
    height: u32,
}

impl Adam7Accumulator {
    /// Create a new accumulator for the given dimensions.
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            data: vec![None; (width * height) as usize],
            width,
            height,
        }
    }

    /// Merge GPU results into accumulator.
    ///
    /// Only updates pixels where GPU returned valid data (not sentinel).
    pub fn merge(&mut self, gpu_result: &[ComputeData]) {
        let mut merged_count = 0u32;
        let mut sentinel_count = 0u32;
        let mut non_mandelbrot_count = 0u32;

        for (i, computed) in gpu_result.iter().enumerate() {
            if let ComputeData::Mandelbrot(m) = computed {
                if m.iterations != SENTINEL_NOT_COMPUTED {
                    self.data[i] = Some(computed.clone());
                    merged_count += 1;
                } else {
                    sentinel_count += 1;
                }
            } else {
                non_mandelbrot_count += 1;
            }
        }

        let filled = self.data.iter().filter(|opt| opt.is_some()).count();
        let total = self.data.len();
        let percent = (filled as f64 / total as f64) * 100.0;

        log::debug!(
            "Merge: {} pixels merged, {} sentinel, {} non-mandelbrot. Total filled: {}/{} ({:.1}%)",
            merged_count,
            sentinel_count,
            non_mandelbrot_count,
            filled,
            total,
            percent
        );
    }

    /// Export to Vec<ComputeData> for colorization.
    ///
    /// Uncomputed pixels (None) are filled from left neighbor, or top neighbor
    /// if at left edge. First pixel defaults to black if uncomputed.
    pub fn to_display_buffer(&self) -> Vec<ComputeData> {
        let mut result = Vec::with_capacity(self.data.len());
        let width = self.width as usize;

        for (i, pixel) in self.data.iter().enumerate() {
            match pixel {
                Some(d) => result.push(d.clone()),
                None => {
                    // Try left neighbor first, then top neighbor
                    let fallback = if i % width > 0 {
                        result.get(i - 1).cloned()
                    } else if i >= width {
                        result.get(i - width).cloned()
                    } else {
                        None
                    };

                    result.push(fallback.unwrap_or_else(Self::black_pixel));
                }
            }
        }

        result
    }

    /// Export final complete buffer for caching.
    ///
    /// After pass 7, all pixels should be computed. Panics if any are missing.
    pub fn to_final_buffer(&self) -> Vec<ComputeData> {
        // Collect missing pixel indices for diagnostic
        let missing: Vec<usize> = self
            .data
            .iter()
            .enumerate()
            .filter_map(|(i, opt)| if opt.is_none() { Some(i) } else { None })
            .collect();

        if !missing.is_empty() {
            let total = self.data.len();
            let width = self.width as usize;

            // Log first few missing pixels with their (x, y) coordinates
            let sample: Vec<String> = missing
                .iter()
                .take(10)
                .map(|&i| {
                    let x = i % width;
                    let y = i / width;
                    format!("({x}, {y})")
                })
                .collect();

            // Check Adam7 pass coverage for missing pixels
            let pass_info: Vec<String> = missing
                .iter()
                .take(5)
                .map(|&i| {
                    let x = (i % width) as u32;
                    let y = (i / width) as u32;
                    let expected_pass = Self::expected_adam7_pass(x, y);
                    format!("({x}, {y}) -> pass {expected_pass}")
                })
                .collect();

            log::error!(
                "Missing {} of {} pixels after pass 7. First 10: [{}]. Expected passes: [{}]",
                missing.len(),
                total,
                sample.join(", "),
                pass_info.join(", ")
            );
            panic!(
                "All pixels should be computed after pass 7. Missing {} pixels, first: {:?}",
                missing.len(),
                sample
            );
        }

        self.data.iter().map(|opt| opt.clone().unwrap()).collect()
    }

    /// Determine which Adam7 pass should compute a given pixel.
    fn expected_adam7_pass(x: u32, y: u32) -> u8 {
        // Adam7 interlacing pattern
        if x.is_multiple_of(8) && y.is_multiple_of(8) {
            1
        } else if x % 8 == 4 && y.is_multiple_of(8) {
            2
        } else if x.is_multiple_of(4) && y % 8 == 4 {
            3
        } else if x % 4 == 2 && y.is_multiple_of(4) {
            4
        } else if x.is_multiple_of(2) && y % 4 == 2 {
            5
        } else if !x.is_multiple_of(2) && y.is_multiple_of(2) {
            6
        } else if !y.is_multiple_of(2) {
            7
        } else {
            0 // Should never happen - indicates pattern bug
        }
    }

    /// Check if all pixels have been computed.
    pub fn is_complete(&self) -> bool {
        self.data.iter().all(|opt| opt.is_some())
    }

    /// Default black pixel for uncomputed areas.
    fn black_pixel() -> ComputeData {
        ComputeData::Mandelbrot(MandelbrotData {
            iterations: 0,
            max_iterations: 1,
            escaped: false,
            glitched: false,
            final_z_norm_sq: 0.0,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_data(iterations: u32) -> ComputeData {
        ComputeData::Mandelbrot(MandelbrotData {
            iterations,
            max_iterations: 1000,
            escaped: iterations < 1000,
            glitched: false,
            final_z_norm_sq: 0.0,
        })
    }

    fn make_sentinel() -> ComputeData {
        ComputeData::Mandelbrot(MandelbrotData {
            iterations: SENTINEL_NOT_COMPUTED,
            max_iterations: 1000,
            escaped: false,
            glitched: false,
            final_z_norm_sq: 0.0,
        })
    }

    fn get_iterations(data: &ComputeData) -> u32 {
        match data {
            ComputeData::Mandelbrot(m) => m.iterations,
            _ => panic!("Expected Mandelbrot"),
        }
    }

    #[test]
    fn test_all_passes() {
        let passes = Adam7Pass::all();
        assert_eq!(passes.len(), 7);
        assert_eq!(passes[0].step(), 1);
        assert_eq!(passes[6].step(), 7);
    }

    #[test]
    fn test_is_final() {
        assert!(!Adam7Pass::new(1).is_final());
        assert!(!Adam7Pass::new(6).is_final());
        assert!(Adam7Pass::new(7).is_final());
    }

    #[test]
    fn test_cumulative_percent() {
        assert!((Adam7Pass::new(1).cumulative_percent() - 1.5625).abs() < 0.001);
        assert!((Adam7Pass::new(7).cumulative_percent() - 100.0).abs() < 0.001);
    }

    #[test]
    fn test_fractions_sum_to_one() {
        let total: f32 = Adam7Pass::all().iter().map(|p| p.pass_fraction()).sum();
        assert!((total - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_step_zero_computes_all() {
        let pass = Adam7Pass::new(0);
        assert_eq!(pass.step(), 0);
        assert!(!pass.is_final()); // step 0 is not a "final" pass
    }

    #[test]
    fn test_all_pixels_helper() {
        let pass = Adam7Pass::all_pixels();
        assert_eq!(pass.step(), 0);
    }

    #[test]
    #[should_panic(expected = "Adam7 step must be 0-7")]
    fn test_invalid_step_eight() {
        Adam7Pass::new(8);
    }

    #[test]
    fn test_new_accumulator() {
        let acc = Adam7Accumulator::new(10, 10);
        assert_eq!(acc.data.len(), 100);
        assert!(acc.data.iter().all(|x| x.is_none()));
    }

    #[test]
    fn test_merge_skips_sentinel() {
        let mut acc = Adam7Accumulator::new(2, 2);

        // GPU returns: computed, sentinel, sentinel, computed
        let gpu_result = vec![
            make_data(100),
            make_sentinel(),
            make_sentinel(),
            make_data(200),
        ];

        acc.merge(&gpu_result);

        assert!(acc.data[0].is_some());
        assert!(acc.data[1].is_none());
        assert!(acc.data[2].is_none());
        assert!(acc.data[3].is_some());
    }

    #[test]
    fn test_to_display_buffer_fills_gaps() {
        let mut acc = Adam7Accumulator::new(4, 1);

        // Only first and last computed
        acc.data[0] = Some(make_data(100));
        acc.data[3] = Some(make_data(200));

        let display = acc.to_display_buffer();

        // Gaps filled from left neighbor
        assert_eq!(get_iterations(&display[0]), 100);
        assert_eq!(get_iterations(&display[1]), 100); // from left
        assert_eq!(get_iterations(&display[2]), 100); // from left
        assert_eq!(get_iterations(&display[3]), 200);
    }

    #[test]
    fn test_to_display_buffer_uses_top_at_edge() {
        let mut acc = Adam7Accumulator::new(2, 2);

        // Row 0: [100, 200]
        // Row 1: [None, 300]
        acc.data[0] = Some(make_data(100));
        acc.data[1] = Some(make_data(200));
        acc.data[3] = Some(make_data(300));

        let display = acc.to_display_buffer();

        // acc.data[2] (row 1, col 0) should copy from top (acc.data[0])
        assert_eq!(get_iterations(&display[2]), 100);
    }

    #[test]
    fn test_is_complete() {
        let mut acc = Adam7Accumulator::new(2, 2);
        assert!(!acc.is_complete());

        acc.data[0] = Some(make_data(1));
        acc.data[1] = Some(make_data(2));
        acc.data[2] = Some(make_data(3));
        assert!(!acc.is_complete());

        acc.data[3] = Some(make_data(4));
        assert!(acc.is_complete());
    }

    #[test]
    fn test_to_final_buffer() {
        let mut acc = Adam7Accumulator::new(2, 2);
        acc.data[0] = Some(make_data(1));
        acc.data[1] = Some(make_data(2));
        acc.data[2] = Some(make_data(3));
        acc.data[3] = Some(make_data(4));

        let final_buf = acc.to_final_buffer();
        assert_eq!(final_buf.len(), 4);
        assert_eq!(get_iterations(&final_buf[0]), 1);
        assert_eq!(get_iterations(&final_buf[3]), 4);
    }

    #[test]
    #[should_panic(expected = "All pixels should be computed")]
    fn test_to_final_buffer_panics_if_incomplete() {
        let acc = Adam7Accumulator::new(2, 2);
        acc.to_final_buffer();
    }
}
