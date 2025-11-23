use dashu_base::{Abs, Approximation};
use dashu_float::ops::SquareRoot;
use dashu_float::{DBig, FBig};
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

    /// Create BigFloat from string with explicit precision
    ///
    /// Allows creating values beyond f64 range (e.g., "1e1000").
    /// Uses atomic base conversion with target precision to avoid precision loss.
    pub fn from_string(val: &str, precision_bits: usize) -> Result<Self, String> {
        if precision_bits <= 64 {
            val.parse::<f64>()
                .map(|f| Self::with_precision(f, precision_bits))
                .map_err(|e| format!("Failed to parse f64: {}", e))
        } else {
            // Parse as decimal, then convert to binary with atomic precision specification
            val.parse::<DBig>()
                .map_err(|e| format!("Failed to parse DBig: {}", e))
                .map(|dbig| {
                    // Use with_base_and_precision for atomic conversion with target precision
                    // Returns Approximation enum, extract value using match
                    let fbig_halfaway = match dbig.with_base_and_precision::<2>(precision_bits) {
                        Approximation::Exact(v) => v,
                        Approximation::Inexact(v, _) => v,
                    };
                    // Convert from HalfAway rounding to Zero rounding (used by FBig default)
                    let fbig_with_prec =
                        fbig_halfaway.with_rounding::<dashu_float::round::mode::Zero>();
                    Self {
                        value: BigFloatValue::Arbitrary(fbig_with_prec),
                        precision_bits,
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
}
