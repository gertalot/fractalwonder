use dashu_float::ops::SquareRoot;
use dashu_float::FBig;
use serde::{Deserialize, Serialize};

/// Arbitrary precision floating point with explicit precision enforcement
///
/// Uses f64 internally when precision_bits <= 64, FBig otherwise.
/// This optimization is completely transparent to external code.
#[derive(Clone, Debug)]
pub struct BigFloat {
    value: BigFloatValue,
    precision_bits: usize,
}

#[derive(Clone, Debug)]
pub enum BigFloatValue {
    F64(f64),
    Arbitrary(FBig),
}

impl BigFloat {
    /// Create BigFloat from f64 with explicit precision
    ///
    /// NO DEFAULT - precision must always be specified
    pub fn with_precision(val: f64, precision_bits: usize) -> Self {
        let value = if precision_bits <= 64 {
            BigFloatValue::F64(val)
        } else {
            let fbig = if val == 0.0 {
                FBig::ZERO.with_precision(precision_bits).unwrap()
            } else {
                FBig::try_from(val)
                    .unwrap()
                    .with_precision(precision_bits)
                    .unwrap()
            };
            BigFloatValue::Arbitrary(fbig)
        };

        Self {
            value,
            precision_bits,
        }
    }

    /// Create zero with explicit precision
    pub fn zero(precision_bits: usize) -> Self {
        Self::with_precision(0.0, precision_bits)
    }

    /// Create one with explicit precision
    pub fn one(precision_bits: usize) -> Self {
        Self::with_precision(1.0, precision_bits)
    }

    /// Get precision in bits
    pub fn precision_bits(&self) -> usize {
        self.precision_bits
    }

    /// Convert to f64 (for display/colorization only)
    /// May lose precision for values requiring > 64 bits
    pub fn to_f64(&self) -> f64 {
        match &self.value {
            BigFloatValue::F64(v) => *v,
            BigFloatValue::Arbitrary(v) => v.to_f64().value(),
        }
    }

    /// Add two BigFloats, preserving max precision
    pub fn add(&self, other: &Self) -> Self {
        let result_precision = self.precision_bits.max(other.precision_bits);

        let result_value = match (&self.value, &other.value) {
            (BigFloatValue::F64(a), BigFloatValue::F64(b)) if result_precision <= 64 => {
                BigFloatValue::F64(a + b)
            }
            _ => {
                let a_big = self.to_fbig();
                let b_big = other.to_fbig();
                BigFloatValue::Arbitrary(&a_big + &b_big)
            }
        };

        Self {
            value: result_value,
            precision_bits: result_precision,
        }
    }

    /// Subtract two BigFloats, preserving max precision
    pub fn sub(&self, other: &Self) -> Self {
        let result_precision = self.precision_bits.max(other.precision_bits);

        let result_value = match (&self.value, &other.value) {
            (BigFloatValue::F64(a), BigFloatValue::F64(b)) if result_precision <= 64 => {
                BigFloatValue::F64(a - b)
            }
            _ => {
                let a_big = self.to_fbig();
                let b_big = other.to_fbig();
                BigFloatValue::Arbitrary(&a_big - &b_big)
            }
        };

        Self {
            value: result_value,
            precision_bits: result_precision,
        }
    }

    /// Multiply two BigFloats, preserving max precision
    pub fn mul(&self, other: &Self) -> Self {
        let result_precision = self.precision_bits.max(other.precision_bits);

        let result_value = match (&self.value, &other.value) {
            (BigFloatValue::F64(a), BigFloatValue::F64(b)) if result_precision <= 64 => {
                BigFloatValue::F64(a * b)
            }
            _ => {
                let a_big = self.to_fbig();
                let b_big = other.to_fbig();
                BigFloatValue::Arbitrary(&a_big * &b_big)
            }
        };

        Self {
            value: result_value,
            precision_bits: result_precision,
        }
    }

    /// Divide two BigFloats, preserving max precision
    pub fn div(&self, other: &Self) -> Self {
        let result_precision = self.precision_bits.max(other.precision_bits);

        let result_value = match (&self.value, &other.value) {
            (BigFloatValue::F64(a), BigFloatValue::F64(b)) if result_precision <= 64 => {
                BigFloatValue::F64(a / b)
            }
            _ => {
                let a_big = self.to_fbig();
                let b_big = other.to_fbig();
                BigFloatValue::Arbitrary(&a_big / &b_big)
            }
        };

        Self {
            value: result_value,
            precision_bits: result_precision,
        }
    }

    /// Square root, preserving precision
    pub fn sqrt(&self) -> Self {
        let result_value = match &self.value {
            BigFloatValue::F64(v) if self.precision_bits <= 64 => BigFloatValue::F64(v.sqrt()),
            _ => {
                let v_big = self.to_fbig();
                BigFloatValue::Arbitrary(v_big.sqrt())
            }
        };

        Self {
            value: result_value,
            precision_bits: self.precision_bits,
        }
    }

    /// Convert to FBig for arbitrary precision operations
    fn to_fbig(&self) -> FBig {
        match &self.value {
            BigFloatValue::F64(v) => {
                if *v == 0.0 {
                    // Special handling for zero - create it with precision
                    FBig::ZERO.with_precision(self.precision_bits).unwrap()
                } else {
                    FBig::try_from(*v)
                        .unwrap()
                        .with_precision(self.precision_bits)
                        .unwrap()
                }
            }
            BigFloatValue::Arbitrary(v) => v.clone(),
        }
    }
}

impl PartialEq for BigFloat {
    fn eq(&self, other: &Self) -> bool {
        match (&self.value, &other.value) {
            (BigFloatValue::F64(a), BigFloatValue::F64(b)) => a == b,
            _ => {
                let a_big = self.to_fbig();
                let b_big = other.to_fbig();
                a_big == b_big
            }
        }
    }
}

impl PartialOrd for BigFloat {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match (&self.value, &other.value) {
            (BigFloatValue::F64(a), BigFloatValue::F64(b)) => a.partial_cmp(b),
            _ => {
                let a_big = self.to_fbig();
                let b_big = other.to_fbig();
                a_big.partial_cmp(&b_big)
            }
        }
    }
}

#[derive(Serialize, Deserialize)]
struct BigFloatSerde {
    value: String,
    precision_bits: usize,
}

impl Serialize for BigFloat {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let value_str = match &self.value {
            BigFloatValue::F64(v) => v.to_string(),
            BigFloatValue::Arbitrary(v) => v.to_string(),
        };

        let serde = BigFloatSerde {
            value: value_str,
            precision_bits: self.precision_bits,
        };

        serde.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for BigFloat {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let serde = BigFloatSerde::deserialize(deserializer)?;

        let value = if serde.precision_bits <= 64 {
            let f = serde
                .value
                .parse::<f64>()
                .map_err(|e| serde::de::Error::custom(format!("Failed to parse f64: {}", e)))?;
            BigFloatValue::F64(f)
        } else {
            let fbig = serde
                .value
                .parse::<FBig>()
                .map_err(|e| serde::de::Error::custom(format!("Failed to parse FBig: {}", e)))?;
            BigFloatValue::Arbitrary(fbig)
        };

        Ok(BigFloat {
            value,
            precision_bits: serde.precision_bits,
        })
    }
}

#[cfg(test)]
#[allow(clippy::approx_constant)]
mod tests {
    use super::*;

    // === Creation Tests ===

    #[test]
    fn test_zero_with_precision() {
        let bf = BigFloat::zero(64);
        assert_eq!(bf.precision_bits(), 64);
        assert_eq!(bf.to_f64(), 0.0);

        let bf2 = BigFloat::zero(256);
        assert_eq!(bf2.precision_bits(), 256);
        assert_eq!(bf2.to_f64(), 0.0);
    }

    #[test]
    fn test_one_with_precision() {
        let bf = BigFloat::one(64);
        assert_eq!(bf.precision_bits(), 64);
        assert_eq!(bf.to_f64(), 1.0);

        let bf2 = BigFloat::one(128);
        assert_eq!(bf2.precision_bits(), 128);
        assert_eq!(bf2.to_f64(), 1.0);
    }

    #[test]
    fn test_with_precision() {
        let bf = BigFloat::with_precision(42.5, 128);
        assert_eq!(bf.precision_bits(), 128);
        assert!((bf.to_f64() - 42.5).abs() < 1e-10);

        let bf2 = BigFloat::with_precision(-3.14159, 256);
        assert_eq!(bf2.precision_bits(), 256);
        assert!((bf2.to_f64() - (-3.14159)).abs() < 1e-5);
    }

    #[test]
    fn test_f64_path_used_for_low_precision() {
        let bf = BigFloat::with_precision(2.0, 64);
        // Should use f64 internally when precision <= 64
        if let BigFloatValue::F64(_) = bf.value {
            // Correct
        } else {
            panic!("Should use f64 fast path for precision=64");
        }
    }

    #[test]
    fn test_arbitrary_path_used_for_high_precision() {
        let bf = BigFloat::with_precision(2.0, 128);
        // Should use FBig internally when precision > 64
        if let BigFloatValue::Arbitrary(_) = bf.value {
            // Correct
        } else {
            panic!("Should use FBig for precision > 64");
        }
    }

    // === Addition Tests ===

    #[test]
    fn test_add_same_precision() {
        let a = BigFloat::with_precision(2.5, 128);
        let b = BigFloat::with_precision(1.5, 128);
        let result = a.add(&b);
        assert_eq!(result.precision_bits(), 128);
        assert!((result.to_f64() - 4.0).abs() < 1e-10);
    }

    #[test]
    fn test_add_preserves_max_precision() {
        let a = BigFloat::with_precision(2.5, 64);
        let b = BigFloat::with_precision(1.5, 256);
        let result = a.add(&b);
        assert_eq!(result.precision_bits(), 256); // Max precision
        assert!((result.to_f64() - 4.0).abs() < 1e-10);
    }

    #[test]
    fn test_add_negative_numbers() {
        let a = BigFloat::with_precision(-5.0, 128);
        let b = BigFloat::with_precision(3.0, 128);
        let result = a.add(&b);
        assert!((result.to_f64() - (-2.0)).abs() < 1e-10);
    }

    #[test]
    fn test_add_with_zero() {
        let a = BigFloat::with_precision(42.0, 128);
        let b = BigFloat::zero(128);
        let result = a.add(&b);
        assert!((result.to_f64() - 42.0).abs() < 1e-10);
    }

    #[test]
    fn test_add_large_numbers() {
        let a = BigFloat::with_precision(1e100, 256);
        let b = BigFloat::with_precision(1e100, 256);
        let result = a.add(&b);
        assert!((result.to_f64() - 2e100).abs() / 2e100 < 1e-10);
    }

    #[test]
    fn test_add_very_small_numbers() {
        let a = BigFloat::with_precision(1e-100, 256);
        let b = BigFloat::with_precision(1e-100, 256);
        let result = a.add(&b);
        assert!((result.to_f64() - 2e-100).abs() / 2e-100 < 1e-10);
    }

    // === Subtraction Tests ===

    #[test]
    fn test_sub_same_precision() {
        let a = BigFloat::with_precision(5.0, 128);
        let b = BigFloat::with_precision(3.0, 128);
        let result = a.sub(&b);
        assert!((result.to_f64() - 2.0).abs() < 1e-10);
    }

    #[test]
    fn test_sub_preserves_max_precision() {
        let a = BigFloat::with_precision(5.0, 64);
        let b = BigFloat::with_precision(3.0, 256);
        let result = a.sub(&b);
        assert_eq!(result.precision_bits(), 256);
    }

    #[test]
    fn test_sub_negative_result() {
        let a = BigFloat::with_precision(3.0, 128);
        let b = BigFloat::with_precision(5.0, 128);
        let result = a.sub(&b);
        assert!((result.to_f64() - (-2.0)).abs() < 1e-10);
    }

    #[test]
    fn test_sub_with_zero() {
        let a = BigFloat::with_precision(42.0, 128);
        let b = BigFloat::zero(128);
        let result = a.sub(&b);
        assert!((result.to_f64() - 42.0).abs() < 1e-10);
    }

    #[test]
    fn test_sub_from_zero() {
        let a = BigFloat::zero(128);
        let b = BigFloat::with_precision(42.0, 128);
        let result = a.sub(&b);
        assert!((result.to_f64() - (-42.0)).abs() < 1e-10);
    }

    // === Multiplication Tests ===

    #[test]
    fn test_mul_same_precision() {
        let a = BigFloat::with_precision(3.0, 128);
        let b = BigFloat::with_precision(4.0, 128);
        let result = a.mul(&b);
        assert!((result.to_f64() - 12.0).abs() < 1e-10);
    }

    #[test]
    fn test_mul_preserves_max_precision() {
        let a = BigFloat::with_precision(3.0, 64);
        let b = BigFloat::with_precision(4.0, 256);
        let result = a.mul(&b);
        assert_eq!(result.precision_bits(), 256);
    }

    #[test]
    fn test_mul_with_zero() {
        let a = BigFloat::with_precision(42.0, 128);
        let b = BigFloat::zero(128);
        let result = a.mul(&b);
        assert_eq!(result.to_f64(), 0.0);
    }

    #[test]
    fn test_mul_with_one() {
        let a = BigFloat::with_precision(42.0, 128);
        let b = BigFloat::one(128);
        let result = a.mul(&b);
        assert!((result.to_f64() - 42.0).abs() < 1e-10);
    }

    #[test]
    fn test_mul_negative_numbers() {
        let a = BigFloat::with_precision(-3.0, 128);
        let b = BigFloat::with_precision(4.0, 128);
        let result = a.mul(&b);
        assert!((result.to_f64() - (-12.0)).abs() < 1e-10);

        let c = BigFloat::with_precision(-3.0, 128);
        let d = BigFloat::with_precision(-4.0, 128);
        let result2 = c.mul(&d);
        assert!((result2.to_f64() - 12.0).abs() < 1e-10);
    }

    #[test]
    fn test_mul_large_numbers() {
        let a = BigFloat::with_precision(1e50, 256);
        let b = BigFloat::with_precision(1e50, 256);
        let result = a.mul(&b);
        assert!((result.to_f64() - 1e100).abs() / 1e100 < 1e-10);
    }

    // === Division Tests ===

    #[test]
    fn test_div_same_precision() {
        let a = BigFloat::with_precision(10.0, 128);
        let b = BigFloat::with_precision(2.0, 128);
        let result = a.div(&b);
        assert!((result.to_f64() - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_div_preserves_max_precision() {
        let a = BigFloat::with_precision(10.0, 64);
        let b = BigFloat::with_precision(2.0, 256);
        let result = a.div(&b);
        assert_eq!(result.precision_bits(), 256);
    }

    #[test]
    fn test_div_by_one() {
        let a = BigFloat::with_precision(42.0, 128);
        let b = BigFloat::one(128);
        let result = a.div(&b);
        assert!((result.to_f64() - 42.0).abs() < 1e-10);
    }

    #[test]
    fn test_div_fractional_result() {
        let a = BigFloat::with_precision(1.0, 128);
        let b = BigFloat::with_precision(3.0, 128);
        let result = a.div(&b);
        assert!((result.to_f64() - 0.333333).abs() < 1e-5);
    }

    #[test]
    fn test_div_negative_numbers() {
        let a = BigFloat::with_precision(-10.0, 128);
        let b = BigFloat::with_precision(2.0, 128);
        let result = a.div(&b);
        assert!((result.to_f64() - (-5.0)).abs() < 1e-10);
    }

    #[test]
    fn test_div_by_large_number() {
        let a = BigFloat::with_precision(1.0, 256);
        let b = BigFloat::with_precision(1e100, 256);
        let result = a.div(&b);
        assert!((result.to_f64() - 1e-100).abs() / 1e-100 < 1e-10);
    }

    // === Square Root Tests ===

    #[test]
    fn test_sqrt_perfect_square() {
        let a = BigFloat::with_precision(16.0, 128);
        let result = a.sqrt();
        assert!((result.to_f64() - 4.0).abs() < 1e-10);
    }

    #[test]
    fn test_sqrt_preserves_precision() {
        let a = BigFloat::with_precision(2.0, 256);
        let result = a.sqrt();
        assert_eq!(result.precision_bits(), 256);
    }

    #[test]
    fn test_sqrt_non_perfect_square() {
        let a = BigFloat::with_precision(2.0, 128);
        let result = a.sqrt();
        assert!((result.to_f64() - 1.414213).abs() < 1e-5);
    }

    #[test]
    fn test_sqrt_of_one() {
        let a = BigFloat::one(128);
        let result = a.sqrt();
        assert!((result.to_f64() - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_sqrt_of_zero() {
        let a = BigFloat::zero(128);
        let result = a.sqrt();
        assert_eq!(result.to_f64(), 0.0);
    }

    #[test]
    fn test_sqrt_large_number() {
        let a = BigFloat::with_precision(1e100, 256);
        let result = a.sqrt();
        assert!((result.to_f64() - 1e50).abs() / 1e50 < 1e-10);
    }

    // === Comparison Tests ===

    #[test]
    fn test_partial_eq_same_value() {
        let a = BigFloat::with_precision(2.5, 128);
        let b = BigFloat::with_precision(2.5, 128);
        assert_eq!(a, b);
    }

    #[test]
    fn test_partial_eq_different_precision_same_value() {
        let a = BigFloat::with_precision(2.5, 64);
        let b = BigFloat::with_precision(2.5, 256);
        assert_eq!(a, b); // Values equal, precision doesn't affect equality
    }

    #[test]
    fn test_partial_eq_different_value() {
        let a = BigFloat::with_precision(2.5, 128);
        let b = BigFloat::with_precision(3.5, 128);
        assert_ne!(a, b);
    }

    #[test]
    fn test_partial_ord_less_than() {
        let a = BigFloat::with_precision(2.5, 128);
        let b = BigFloat::with_precision(3.5, 128);
        assert!(a < b);
    }

    #[test]
    fn test_partial_ord_greater_than() {
        let a = BigFloat::with_precision(3.5, 128);
        let b = BigFloat::with_precision(2.5, 128);
        assert!(a > b);
    }

    #[test]
    fn test_partial_ord_equal() {
        let a = BigFloat::with_precision(2.5, 128);
        let b = BigFloat::with_precision(2.5, 128);
        assert_eq!(a.partial_cmp(&b), Some(std::cmp::Ordering::Equal));
    }

    #[test]
    fn test_partial_ord_negative_numbers() {
        let a = BigFloat::with_precision(-2.5, 128);
        let b = BigFloat::with_precision(-1.5, 128);
        assert!(a < b); // -2.5 < -1.5
    }

    #[test]
    fn test_partial_ord_zero() {
        let a = BigFloat::zero(128);
        let b = BigFloat::with_precision(1.0, 128);
        assert!(a < b);

        let c = BigFloat::with_precision(-1.0, 128);
        let d = BigFloat::zero(128);
        assert!(c < d);
    }

    // === Serialization Tests ===

    #[test]
    fn test_serialization_roundtrip_f64_precision() {
        let original = BigFloat::with_precision(3.14159, 64);
        let json = serde_json::to_string(&original).expect("serialize failed");
        let restored: BigFloat = serde_json::from_str(&json).expect("deserialize failed");

        assert_eq!(restored.precision_bits(), 64);
        assert!((restored.to_f64() - 3.14159).abs() < 1e-5);
    }

    #[test]
    fn test_serialization_roundtrip_high_precision() {
        let original = BigFloat::with_precision(3.14159, 256);
        let json = serde_json::to_string(&original).expect("serialize failed");
        let restored: BigFloat = serde_json::from_str(&json).expect("deserialize failed");

        assert_eq!(restored.precision_bits(), 256);
        assert!((restored.to_f64() - 3.14159).abs() < 1e-5);
    }

    #[test]
    fn test_serialization_preserves_zero() {
        let original = BigFloat::zero(128);
        let json = serde_json::to_string(&original).unwrap();
        let restored: BigFloat = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.to_f64(), 0.0);
        assert_eq!(restored.precision_bits(), 128);
    }

    #[test]
    fn test_serialization_preserves_negative() {
        let original = BigFloat::with_precision(-42.5, 256);
        let json = serde_json::to_string(&original).unwrap();
        let restored: BigFloat = serde_json::from_str(&json).unwrap();

        assert!((restored.to_f64() - (-42.5)).abs() < 1e-10);
        assert_eq!(restored.precision_bits(), 256);
    }

    // === Complex Expression Tests ===

    #[test]
    fn test_complex_expression_with_all_operations() {
        // (2 + 3) * 4 / 2 - 1 = 9
        let two = BigFloat::with_precision(2.0, 128);
        let three = BigFloat::with_precision(3.0, 128);
        let four = BigFloat::with_precision(4.0, 128);
        let one = BigFloat::one(128);

        let result = two.add(&three).mul(&four).div(&two).sub(&one);
        assert!((result.to_f64() - 9.0).abs() < 1e-10);
    }

    #[test]
    fn test_mandelbrot_iteration_formula() {
        // Test z = z^2 + c where z=(2,1), c=(0.5, 0.3)
        // z^2 = (2^2 - 1^2, 2*2*1) = (3, 4)
        // z^2 + c = (3.5, 4.3)
        let z_real = BigFloat::with_precision(2.0, 128);
        let z_imag = BigFloat::with_precision(1.0, 128);
        let c_real = BigFloat::with_precision(0.5, 128);
        let c_imag = BigFloat::with_precision(0.3, 128);

        let z_real_sq = z_real.mul(&z_real);
        let z_imag_sq = z_imag.mul(&z_imag);
        let new_real = z_real_sq.sub(&z_imag_sq).add(&c_real);

        let two = BigFloat::with_precision(2.0, 128);
        let new_imag = two.mul(&z_real).mul(&z_imag).add(&c_imag);

        assert!((new_real.to_f64() - 3.5).abs() < 1e-10);
        assert!((new_imag.to_f64() - 4.3).abs() < 1e-10);
    }

    // === Edge Case Tests ===

    #[test]
    fn test_very_large_precision() {
        let bf = BigFloat::with_precision(1.0, 1024);
        assert_eq!(bf.precision_bits(), 1024);
        assert_eq!(bf.to_f64(), 1.0);
    }

    #[test]
    fn test_operations_maintain_finite_values() {
        let a = BigFloat::with_precision(1e100, 256);
        let b = BigFloat::with_precision(1e-100, 256);

        let product = a.mul(&b);
        assert!(product.to_f64().is_finite());

        let quotient = a.div(&b);
        assert!(quotient.to_f64().is_finite());
    }

    #[test]
    fn test_chain_of_operations_maintains_precision() {
        let start = BigFloat::with_precision(1.0, 256);
        let two = BigFloat::with_precision(2.0, 256);

        let result = start.add(&two).mul(&two).div(&two).sub(&two);

        // Should get back to 1.0
        assert!((result.to_f64() - 1.0).abs() < 1e-10);
        assert_eq!(result.precision_bits(), 256);
    }
}
