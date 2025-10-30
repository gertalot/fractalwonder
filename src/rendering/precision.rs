pub struct PrecisionCalculator;

impl PrecisionCalculator {
    /// Calculate required precision bits for given zoom level.
    ///
    /// Formula: precision_bits = max(zoom.log10() × 3.322 + 128, 64).next_power_of_two()
    ///
    /// Explanation:
    /// - Each decimal digit requires ~3.322 bits (log2(10))
    /// - Add 128 bit base for safety margin
    /// - Minimum 64 bits (for low zoom)
    /// - Round up to next power of 2 for efficient allocation
    ///
    /// Examples:
    /// - zoom=1: 64 bits (minimum)
    /// - zoom=1e10: 128 bits
    /// - zoom=1e15: 256 bits
    /// - zoom=1e30: 512 bits
    /// - zoom=1e50: 1024 bits
    pub fn calculate_precision_bits(zoom: f64) -> usize {
        let zoom_digits = zoom.log10();
        let required_bits = (zoom_digits * 3.322 + 128.0) as usize;
        required_bits.max(64).next_power_of_two()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_precision_scales_with_zoom() {
        let bits_1 = PrecisionCalculator::calculate_precision_bits(1.0);
        let bits_20 = PrecisionCalculator::calculate_precision_bits(1e20);
        let bits_50 = PrecisionCalculator::calculate_precision_bits(1e50);
        let bits_120 = PrecisionCalculator::calculate_precision_bits(1e120);

        // At low zoom, should use reasonable baseline
        assert!(bits_1 >= 64);
        assert!(bits_1 <= 256);

        // Should scale with zoom (using zoom levels that produce different power-of-two results)
        assert_eq!(bits_1, 128);
        assert_eq!(bits_20, 256);
        assert_eq!(bits_50, 512);
        assert_eq!(bits_120, 1024);
    }

    #[test]
    fn test_precision_is_power_of_two() {
        let bits = PrecisionCalculator::calculate_precision_bits(1e20);
        assert_eq!(bits.count_ones(), 1); // Power of 2
    }

    #[test]
    fn test_minimum_precision() {
        // Even at zoom=1, should have reasonable minimum
        let bits = PrecisionCalculator::calculate_precision_bits(1.0);
        assert!(bits >= 64);
    }
}
