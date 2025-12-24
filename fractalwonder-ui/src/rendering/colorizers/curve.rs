//! Cubic interpolating spline curves for transfer and falloff functions.

use serde::{Deserialize, Serialize};

/// A control point on a curve.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CurvePoint {
    pub x: f64,
    pub y: f64,
}

/// A cubic interpolating spline through control points.
/// The curve passes exactly through each point.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Curve {
    pub points: Vec<CurvePoint>,
}

impl Curve {
    /// Create a linear (identity) curve.
    pub fn linear() -> Self {
        Self {
            points: vec![CurvePoint { x: 0.0, y: 0.0 }, CurvePoint { x: 1.0, y: 1.0 }],
        }
    }

    /// Evaluate the curve at position x using linear interpolation.
    /// TODO: Replace with cubic spline interpolation.
    pub fn evaluate(&self, x: f64) -> f64 {
        let x = x.clamp(0.0, 1.0);

        if self.points.is_empty() {
            return x;
        }
        if self.points.len() == 1 {
            return self.points[0].y;
        }

        // Find the segment containing x
        let mut i = 0;
        while i < self.points.len() - 1 && self.points[i + 1].x < x {
            i += 1;
        }

        if i >= self.points.len() - 1 {
            return self.points.last().unwrap().y;
        }

        let p0 = &self.points[i];
        let p1 = &self.points[i + 1];

        // Linear interpolation for now
        let t = if (p1.x - p0.x).abs() < 1e-10 {
            0.0
        } else {
            (x - p0.x) / (p1.x - p0.x)
        };

        p0.y + t * (p1.y - p0.y)
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
        };
        assert!((curve.evaluate(0.5) - 0.8).abs() < 0.001);
    }
}
