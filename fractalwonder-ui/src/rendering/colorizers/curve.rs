//! Cubic interpolating spline curves for transfer and falloff functions.

use serde::{Deserialize, Serialize};

/// Scale mode for curve evaluation.
///
/// - `Linear`: Output equals curve value directly (y = curve(x))
/// - `Log`: Applies power function to output for aggressive falloff (y = curve(x)^10)
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum CurveScale {
    /// Linear scale: output equals curve value directly
    Linear,
    /// Log scale: applies power of 10 to output for aggressive falloff
    #[default]
    Log,
}

/// A control point on a curve.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CurvePoint {
    pub x: f64,
    pub y: f64,
}

/// A cubic interpolating spline through control points.
/// The curve passes exactly through each point.
///
/// # Performance Notes
///
/// Spline coefficients are computed on each `evaluate()` call in O(n) time.
/// For typical use cases (2-5 control points), this is negligible and acceptable.
/// If evaluating the same curve thousands of times in a tight loop, consider
/// caching coefficients by moving them to a separate cached structure.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Curve {
    pub points: Vec<CurvePoint>,
    /// Scale mode for curve output (default: Log for aggressive falloff)
    #[serde(default)]
    pub scale: CurveScale,
}

impl Curve {
    /// Create a linear (identity) curve with default (Log) scale.
    pub fn linear() -> Self {
        Self {
            points: vec![CurvePoint { x: 0.0, y: 0.0 }, CurvePoint { x: 1.0, y: 1.0 }],
            scale: CurveScale::default(),
        }
    }

    /// Evaluate the curve at position x using cubic spline interpolation.
    ///
    /// # Performance
    ///
    /// This method recomputes spline coefficients on each call, resulting in O(n)
    /// complexity where n is the number of control points. The coefficient
    /// computation involves solving a tridiagonal linear system using the Thomas
    /// algorithm.
    ///
    /// For typical curves (2-5 control points), this overhead is negligible
    /// (microseconds) and the simplicity of not caching coefficients is preferable.
    /// Curves remain valid even if control points are modified, since coefficients
    /// are always recomputed from the current points.
    pub fn evaluate(&self, x: f64) -> f64 {
        let x = x.clamp(0.0, 1.0);

        if self.points.is_empty() {
            return x;
        }
        if self.points.len() == 1 {
            return self.points[0].y;
        }
        if self.points.len() == 2 {
            return self.linear_interpolate(x);
        }

        self.cubic_interpolate(x)
    }

    fn linear_interpolate(&self, x: f64) -> f64 {
        let p0 = &self.points[0];
        let p1 = &self.points[1];
        let t = if (p1.x - p0.x).abs() < 1e-10 {
            0.0
        } else {
            (x - p0.x) / (p1.x - p0.x)
        };
        p0.y + t * (p1.y - p0.y)
    }

    fn cubic_interpolate(&self, x: f64) -> f64 {
        let n = self.points.len();

        // Find segment
        let mut i = 0;
        while i < n - 1 && self.points[i + 1].x <= x {
            i += 1;
        }
        if i >= n - 1 {
            i = n - 2;
        }

        // Get spline coefficients for this segment
        let coeffs = self.compute_spline_coefficients();
        let (a, b, c, d) = coeffs[i];

        // Local parameter t in [0, 1] for this segment
        let x0 = self.points[i].x;
        let x1 = self.points[i + 1].x;
        let t = if (x1 - x0).abs() < 1e-10 {
            0.0
        } else {
            (x - x0) / (x1 - x0)
        };

        // Evaluate cubic: a + b*t + c*t^2 + d*t^3
        a + t * (b + t * (c + t * d))
    }

    /// Compute natural cubic spline coefficients for each segment.
    /// Returns Vec of (a, b, c, d) for each segment.
    fn compute_spline_coefficients(&self) -> Vec<(f64, f64, f64, f64)> {
        let n = self.points.len();
        if n < 2 {
            return vec![];
        }

        // Extract y values and compute segment widths
        let y: Vec<f64> = self.points.iter().map(|p| p.y).collect();
        let h: Vec<f64> = self.points.windows(2).map(|w| w[1].x - w[0].x).collect();

        // Solve for second derivatives (natural spline: s''(0) = s''(n) = 0)
        let mut m = vec![0.0; n]; // Second derivatives at each point

        if n > 2 {
            // Build tridiagonal system
            let mut alpha = vec![0.0; n - 1];
            for i in 1..n - 1 {
                if h[i - 1].abs() > 1e-10 && h[i].abs() > 1e-10 {
                    alpha[i] = 3.0 / h[i] * (y[i + 1] - y[i]) - 3.0 / h[i - 1] * (y[i] - y[i - 1]);
                }
            }

            // Solve tridiagonal system using Thomas algorithm
            let mut l = vec![1.0; n];
            let mut mu = vec![0.0; n];
            let mut z = vec![0.0; n];

            for i in 1..n - 1 {
                if h[i - 1].abs() > 1e-10 && h[i].abs() > 1e-10 {
                    l[i] =
                        2.0 * (self.points[i + 1].x - self.points[i - 1].x) - h[i - 1] * mu[i - 1];
                    if l[i].abs() > 1e-10 {
                        mu[i] = h[i] / l[i];
                        z[i] = (alpha[i] - h[i - 1] * z[i - 1]) / l[i];
                    }
                }
            }

            // Back substitution
            for i in (1..n - 1).rev() {
                m[i] = z[i] - mu[i] * m[i + 1];
            }
        }

        // Build coefficients for each segment
        let mut coeffs = Vec::with_capacity(n - 1);
        for i in 0..n - 1 {
            let hi = if h[i].abs() > 1e-10 { h[i] } else { 1.0 };

            let a = y[i];
            let b = (y[i + 1] - y[i]) / hi - hi * (2.0 * m[i] + m[i + 1]) / 3.0;
            let c = m[i];
            let d = (m[i + 1] - m[i]) / (3.0 * hi);

            // Scale coefficients for t in [0, 1] instead of [x_i, x_{i+1}]
            let a_scaled = a;
            let b_scaled = b * hi;
            let c_scaled = c * hi * hi;
            let d_scaled = d * hi * hi * hi;

            coeffs.push((a_scaled, b_scaled, c_scaled, d_scaled));
        }

        coeffs
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn curve_linear_endpoints() {
        let curve = Curve::linear();
        assert!((curve.evaluate(0.0) - 0.0).abs() < 0.001);
        assert!((curve.evaluate(1.0) - 1.0).abs() < 0.001);
    }

    #[test]
    fn curve_linear_midpoint() {
        let curve = Curve::linear();
        assert!((curve.evaluate(0.5) - 0.5).abs() < 0.001);
    }

    #[test]
    fn curve_clamps_input() {
        let curve = Curve::linear();
        assert!((curve.evaluate(-0.5) - 0.0).abs() < 0.001);
        assert!((curve.evaluate(1.5) - 1.0).abs() < 0.001);
    }

    #[test]
    fn curve_three_points() {
        let curve = Curve {
            points: vec![
                CurvePoint { x: 0.0, y: 0.0 },
                CurvePoint { x: 0.5, y: 0.8 },
                CurvePoint { x: 1.0, y: 1.0 },
            ],
            scale: CurveScale::Linear,
        };
        assert!((curve.evaluate(0.5) - 0.8).abs() < 0.001);
    }

    #[test]
    fn curve_cubic_is_smooth() {
        // A curve with 3 points should be smooth, not piecewise linear
        let curve = Curve {
            points: vec![
                CurvePoint { x: 0.0, y: 0.0 },
                CurvePoint { x: 0.5, y: 1.0 },
                CurvePoint { x: 1.0, y: 0.0 },
            ],
            scale: CurveScale::Linear,
        };
        // At x=0.25, cubic spline should give value > 0.5 (curves up to peak)
        // Linear would give exactly 0.5
        let y = curve.evaluate(0.25);
        assert!(y > 0.55, "Expected smooth curve, got y={} at x=0.25", y);
    }
}
