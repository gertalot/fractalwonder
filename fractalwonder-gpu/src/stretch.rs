// fractalwonder-gpu/src/stretch.rs

use fractalwonder_core::{ComputeData, MandelbrotData};

/// Sentinel value indicating a pixel was not computed in the current Adam7 pass.
pub const SENTINEL_NOT_COMPUTED: u32 = 0xFFFFFFFF;

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
        for (i, computed) in gpu_result.iter().enumerate() {
            if let ComputeData::Mandelbrot(m) = computed {
                if m.iterations != SENTINEL_NOT_COMPUTED {
                    self.data[i] = Some(computed.clone());
                }
            }
        }
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
        self.data
            .iter()
            .map(|opt| {
                opt.clone()
                    .expect("All pixels should be computed after pass 7")
            })
            .collect()
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
        })
    }

    fn make_sentinel() -> ComputeData {
        ComputeData::Mandelbrot(MandelbrotData {
            iterations: SENTINEL_NOT_COMPUTED,
            max_iterations: 1000,
            escaped: false,
            glitched: false,
        })
    }

    fn get_iterations(data: &ComputeData) -> u32 {
        match data {
            ComputeData::Mandelbrot(m) => m.iterations,
            _ => panic!("Expected Mandelbrot"),
        }
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
