use dashu_base::{Abs, Approximation};
use dashu_float::ops::SquareRoot;
use dashu_float::{DBig, FBig};
use serde::{Deserialize, Serialize};

/// Extract value from Approximation, accepting both Exact and Inexact results.
///
/// The dashu library's `with_precision()` returns `Approximation<FBig, _>` which
/// can be `Exact` or `Inexact`. Using `.unwrap()` panics on `Inexact`, which
/// commonly occurs during precision changes. This helper safely extracts the
/// value regardless of exactness.
fn approx_value<E>(approx: Approximation<FBig, E>) -> FBig {
    match approx {
        Approximation::Exact(v) => v,
        Approximation::Inexact(v, _) => v,
    }
}

/// Check if a string contains an exponent beyond f64's representable range.
/// f64 can represent values from ~5e-324 to ~1.8e308.
/// Returns true if the value would overflow or underflow f64.
fn exceeds_f64_exponent_range(s: &str) -> bool {
    // Maximum safe exponent magnitude for f64 (conservative)
    const F64_MAX_EXPONENT: i32 = 307;

    // Look for 'e' or 'E' in scientific notation
    let lowercase = s.to_lowercase();
    if let Some(e_pos) = lowercase.find('e') {
        // Skip the 'e' and any sign
        let exp_str = &s[e_pos + 1..];
        if let Ok(exp) = exp_str.parse::<i32>() {
            return exp.abs() > F64_MAX_EXPONENT;
        }
    }

    false
}

/// Estimate log2 from BINARY (base-2) string representation from FBig::to_string().
/// FBig outputs in base-2 format like "0.00000...001..." where zeros are binary zeros.
fn estimate_log2_from_binary_string(s: &str) -> f64 {
    // Strip leading sign if present (log2 of absolute value)
    let unsigned_str = s.strip_prefix('-').unwrap_or(s);

    // Handle small values: "0.000...001..."
    // Count leading zeros after decimal point - each zero is one power of 2
    if let Some(after_decimal) = unsigned_str.strip_prefix("0.") {
        let leading_zeros = after_decimal.chars().take_while(|&c| c == '0').count();
        // In binary: 0.000...001 with n zeros = 2^-(n+1)
        // log2(2^-(n+1)) = -(n+1)
        return -(leading_zeros as f64 + 1.0);
    }

    // Handle large values: count digits before decimal point
    // In binary, n digits before decimal = 2^(n-1) magnitude
    if let Some(dot_pos) = unsigned_str.find('.') {
        let integer_part = &unsigned_str[..dot_pos];
        // Remove any sign
        let digits: String = integer_part
            .chars()
            .filter(|c| *c == '0' || *c == '1')
            .collect();
        if !digits.is_empty() {
            return (digits.len() - 1) as f64;
        }
    } else {
        // No decimal point - count all binary digits
        let digits: String = unsigned_str
            .chars()
            .filter(|c| *c == '0' || *c == '1')
            .collect();
        if !digits.is_empty() {
            return (digits.len() - 1) as f64;
        }
    }

    // Last resort
    0.0
}

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
                approx_value(FBig::ZERO.with_precision(precision_bits))
            } else {
                // FBig::try_from(f64) can fail for NaN/Infinity, handle gracefully
                match FBig::try_from(val) {
                    Ok(f) => approx_value(f.with_precision(precision_bits)),
                    Err(_) => approx_value(FBig::ZERO.with_precision(precision_bits)),
                }
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

    /// Convert to a different precision.
    ///
    /// If reducing precision, this may lose information.
    /// If increasing precision, the value is preserved but stored with more bits.
    pub fn to_precision(&self, precision_bits: usize) -> Self {
        if precision_bits == self.precision_bits {
            return self.clone();
        }

        if precision_bits <= 64 {
            // Downsample to f64
            Self {
                value: BigFloatValue::F64(self.to_f64()),
                precision_bits,
            }
        } else {
            // Convert to arbitrary precision
            let fbig = approx_value(self.to_fbig().with_precision(precision_bits));
            Self {
                value: BigFloatValue::Arbitrary(fbig),
                precision_bits,
            }
        }
    }

    /// Convert to f64 (for display/colorization only)
    /// May lose precision for values requiring > 64 bits
    pub fn to_f64(&self) -> f64 {
        match &self.value {
            BigFloatValue::F64(v) => *v,
            BigFloatValue::Arbitrary(v) => v.to_f64().value(),
        }
    }

    /// Create BigFloat from string with explicit precision
    ///
    /// Allows creating values beyond f64 range (e.g., "1e1000").
    /// Uses atomic base conversion with target precision to avoid precision loss.
    ///
    /// # Automatic precision upgrade
    /// If the string contains an exponent beyond f64's range (~10^308), the value
    /// is automatically parsed with arbitrary precision regardless of precision_bits.
    /// This prevents underflow/overflow when parsing extreme values like "4.0e-1000".
    pub fn from_string(val: &str, precision_bits: usize) -> Result<Self, String> {
        // Check if exponent exceeds f64 range (e.g., "4.0e-1000")
        let extreme_exponent = exceeds_f64_exponent_range(val);

        // Use f64 only if:
        // 1. Requested precision is 64 bits or less, AND
        // 2. The value's exponent is within f64's representable range
        let use_f64 = precision_bits <= 64 && !extreme_exponent;

        if use_f64 {
            val.parse::<f64>()
                .map(|f| Self::with_precision(f, precision_bits))
                .map_err(|e| format!("Failed to parse f64: {}", e))
        } else {
            // Use arbitrary precision parsing
            // For extreme exponents, use minimum 256 bits to avoid precision loss
            // For normal values, use the requested precision
            let effective_precision = if extreme_exponent {
                precision_bits.max(256)
            } else {
                precision_bits
            };

            // Parse as decimal, then convert to binary with atomic precision specification
            val.parse::<DBig>()
                .map_err(|e| format!("Failed to parse DBig: {}", e))
                .map(|dbig| {
                    // Use with_base_and_precision for atomic conversion with target precision
                    // Returns Approximation enum, extract value using match
                    let fbig_halfaway = match dbig.with_base_and_precision::<2>(effective_precision)
                    {
                        Approximation::Exact(v) => v,
                        Approximation::Inexact(v, _) => v,
                    };
                    // Convert from HalfAway rounding to Zero rounding (used by FBig default)
                    let fbig_with_prec =
                        fbig_halfaway.with_rounding::<dashu_float::round::mode::Zero>();
                    Self {
                        value: BigFloatValue::Arbitrary(fbig_with_prec),
                        precision_bits: effective_precision,
                    }
                })
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

    /// Absolute value
    pub fn abs(&self) -> Self {
        match &self.value {
            BigFloatValue::F64(v) => BigFloat {
                value: BigFloatValue::F64(v.abs()),
                precision_bits: self.precision_bits,
            },
            BigFloatValue::Arbitrary(v) => BigFloat {
                value: BigFloatValue::Arbitrary(v.clone().abs()),
                precision_bits: self.precision_bits,
            },
        }
    }

    /// Approximate log2 using exponent extraction.
    /// Accurate to ~1 bit, sufficient for precision calculation.
    /// Returns f64::NEG_INFINITY for zero values.
    pub fn log2_approx(&self) -> f64 {
        match &self.value {
            BigFloatValue::F64(v) => {
                if *v == 0.0 {
                    f64::NEG_INFINITY
                } else {
                    v.abs().log2()
                }
            }
            BigFloatValue::Arbitrary(v) => {
                // FBig uses base-2 representation internally
                // log2(value) ≈ exponent (crude but sufficient for precision calc)
                // For more accuracy, we convert to f64 if possible, else estimate from exponent
                let f64_val = v.to_f64().value();
                if f64_val == 0.0 {
                    // Either actually zero, or underflowed to zero
                    // FBig::to_string() is base-2, so we need to count BINARY zeros
                    // and NOT multiply by log2(10)
                    let s = v.to_string();
                    if s == "0" || s == "0.0" {
                        return f64::NEG_INFINITY;
                    }
                    // Very small but not zero - estimate from binary string
                    estimate_log2_from_binary_string(&s)
                } else if f64_val.is_finite() {
                    f64_val.abs().log2()
                } else {
                    // Value too extreme for f64, estimate from binary string representation
                    let s = v.to_string();
                    estimate_log2_from_binary_string(&s)
                }
            }
        }
    }

    /// Convert to FBig for arbitrary precision operations
    fn to_fbig(&self) -> FBig {
        match &self.value {
            BigFloatValue::F64(v) => {
                if *v == 0.0 {
                    // Special handling for zero - create it with precision
                    approx_value(FBig::ZERO.with_precision(self.precision_bits))
                } else {
                    // FBig::try_from(f64) can fail for NaN/Infinity
                    match FBig::try_from(*v) {
                        Ok(f) => approx_value(f.with_precision(self.precision_bits)),
                        Err(_) => approx_value(FBig::ZERO.with_precision(self.precision_bits)),
                    }
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

impl std::fmt::Display for BigFloat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.value {
            BigFloatValue::F64(v) => write!(f, "{}", v),
            BigFloatValue::Arbitrary(v) => write!(f, "{}", v),
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
mod tests {
    use super::*;

    #[test]
    fn abs_returns_positive_for_negative_value() {
        let neg = BigFloat::with_precision(-5.0, 64);
        let result = neg.abs();
        assert_eq!(result.to_f64(), 5.0);
    }

    #[test]
    fn abs_returns_same_for_positive_value() {
        let pos = BigFloat::with_precision(3.0, 64);
        let result = pos.abs();
        assert_eq!(result.to_f64(), 3.0);
    }

    #[test]
    fn abs_preserves_precision() {
        let neg = BigFloat::with_precision(-5.0, 256);
        let result = neg.abs();
        assert_eq!(result.precision_bits(), 256);
    }

    #[test]
    fn abs_works_with_arbitrary_precision() {
        let neg = BigFloat::from_string("-1e-500", 7000).unwrap();
        let pos = BigFloat::from_string("1e-500", 7000).unwrap();
        assert_eq!(neg.abs(), pos);
    }

    #[test]
    fn log2_approx_returns_correct_value_for_powers_of_two() {
        let val = BigFloat::with_precision(8.0, 64); // 2^3
        let log2 = val.log2_approx();
        assert!((log2 - 3.0).abs() < 0.1);
    }

    #[test]
    fn log2_approx_returns_negative_for_small_values() {
        let val = BigFloat::with_precision(0.125, 64); // 2^-3
        let log2 = val.log2_approx();
        assert!((log2 - (-3.0)).abs() < 0.1);
    }

    #[test]
    fn log2_approx_works_with_extreme_values() {
        // 1e-500 ≈ 2^-1661 (since log2(10) ≈ 3.322)
        let val = BigFloat::from_string("1e-500", 7000).unwrap();
        let log2 = val.log2_approx();
        // Expected: -500 * 3.322 ≈ -1661
        assert!(log2 < -1600.0);
        assert!(log2 > -1700.0);
    }

    #[test]
    fn log2_approx_handles_values_near_one() {
        let val = BigFloat::with_precision(1.0, 64);
        let log2 = val.log2_approx();
        assert!(log2.abs() < 0.1);
    }

    #[test]
    fn log2_approx_returns_neg_infinity_for_zero() {
        let val = BigFloat::with_precision(0.0, 64);
        let log2 = val.log2_approx();
        assert!(log2 == f64::NEG_INFINITY);
    }

    #[test]
    fn log2_approx_handles_negative_input() {
        let val = BigFloat::with_precision(-8.0, 64);
        let log2 = val.log2_approx();
        assert!((log2 - 3.0).abs() < 0.1);
    }

    #[test]
    fn log2_approx_works_with_negative_extreme_values() {
        let val = BigFloat::from_string("-1e-500", 7000).unwrap();
        let log2 = val.log2_approx();
        assert!(log2 < -1600.0);
        assert!(log2 > -1700.0);
    }

    #[test]
    fn from_string_with_extreme_exponent_auto_upgrades_precision() {
        // When parsing "4.0e-1000" with 64 bits, it should auto-upgrade
        // to 256 bits (minimum) to avoid f64 underflow
        let val = BigFloat::from_string("4.0e-1000", 64).unwrap();
        let log2 = val.log2_approx();

        // Should NOT be -inf (which would indicate f64 underflow)
        assert!(log2.is_finite(), "log2 should be finite, not -inf");

        // Should be approximately -1000 * log2(10) ≈ -3322
        assert!(log2 < -3300.0, "log2 should be around -3322");
        assert!(log2 > -3350.0, "log2 should be around -3322");

        // Precision should be upgraded to at least 256 bits
        assert!(
            val.precision_bits() >= 256,
            "precision should be at least 256, got {}",
            val.precision_bits()
        );
    }

    #[test]
    fn from_string_with_normal_exponent_uses_f64_when_low_precision() {
        // Normal values should still use f64 path when precision <= 64
        let val = BigFloat::from_string("2.5", 64).unwrap();
        assert_eq!(val.precision_bits(), 64);
        assert!((val.to_f64() - 2.5).abs() < 0.0001);
    }
}
