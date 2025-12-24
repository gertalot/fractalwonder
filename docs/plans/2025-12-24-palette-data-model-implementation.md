# Palette Data Model Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement the unified Palette data model with gradient stops, cubic spline curves, and localStorage persistence.

**Architecture:** Replace the current Palette (LUT-only) and ColorOptions with a unified Palette struct containing gradient, transfer/falloff curves, lighting params, and flags. Factory defaults compile into the binary; user edits shadow them via localStorage.

**Tech Stack:** Rust, serde, serde_json, web_sys::Storage, OKLAB color space

**Design Document:** `docs/plans/2025-12-24-palette-data-model-design.md`

---

## Phase 1: Core Data Structures

### Task 1: Create CurvePoint and Curve structs

**Files:**
- Create: `fractalwonder-ui/src/rendering/colorizers/curve.rs`
- Modify: `fractalwonder-ui/src/rendering/colorizers/mod.rs`

**Step 1: Write the failing test**

In `fractalwonder-ui/src/rendering/colorizers/curve.rs`:

```rust
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
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p fractalwonder-ui curve::tests --no-default-features`
Expected: FAIL - module not found

**Step 3: Write minimal implementation**

Create `fractalwonder-ui/src/rendering/colorizers/curve.rs`:

```rust
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
            points: vec![
                CurvePoint { x: 0.0, y: 0.0 },
                CurvePoint { x: 1.0, y: 1.0 },
            ],
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
```

**Step 4: Add module to mod.rs**

In `fractalwonder-ui/src/rendering/colorizers/mod.rs`, add:

```rust
pub mod curve;
pub use curve::{Curve, CurvePoint};
```

**Step 5: Run test to verify it passes**

Run: `cargo test -p fractalwonder-ui curve::tests --no-default-features`
Expected: PASS (4 tests)

**Step 6: Commit**

```bash
git add fractalwonder-ui/src/rendering/colorizers/curve.rs
git add fractalwonder-ui/src/rendering/colorizers/mod.rs
git commit -m "feat(palette): add CurvePoint and Curve structs with linear interpolation"
```

---

### Task 2: Implement cubic spline interpolation for Curve

**Files:**
- Modify: `fractalwonder-ui/src/rendering/colorizers/curve.rs`

**Step 1: Write the failing test**

Add to `curve.rs` tests:

```rust
#[test]
fn curve_cubic_is_smooth() {
    // A curve with 3 points should be smooth, not piecewise linear
    let curve = Curve {
        points: vec![
            CurvePoint { x: 0.0, y: 0.0 },
            CurvePoint { x: 0.5, y: 1.0 },
            CurvePoint { x: 1.0, y: 0.0 },
        ],
    };
    // At x=0.25, cubic spline should give value > 0.5 (curves up to peak)
    // Linear would give exactly 0.5
    let y = curve.evaluate(0.25);
    assert!(y > 0.55, "Expected smooth curve, got y={} at x=0.25", y);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p fractalwonder-ui curve::tests::curve_cubic_is_smooth --no-default-features`
Expected: FAIL - linear gives 0.5, not > 0.55

**Step 3: Implement cubic spline**

Replace the `evaluate` method in `curve.rs`:

```rust
impl Curve {
    /// Create a linear (identity) curve.
    pub fn linear() -> Self {
        Self {
            points: vec![
                CurvePoint { x: 0.0, y: 0.0 },
                CurvePoint { x: 1.0, y: 1.0 },
            ],
        }
    }

    /// Evaluate the curve at position x using cubic spline interpolation.
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
                    l[i] = 2.0 * (self.points[i + 1].x - self.points[i - 1].x)
                        - h[i - 1] * mu[i - 1];
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
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p fractalwonder-ui curve::tests --no-default-features`
Expected: PASS (all 5 tests)

**Step 5: Commit**

```bash
git add fractalwonder-ui/src/rendering/colorizers/curve.rs
git commit -m "feat(palette): implement cubic spline interpolation for Curve"
```

---

### Task 3: Create ColorStop and Gradient structs

**Files:**
- Create: `fractalwonder-ui/src/rendering/colorizers/gradient.rs`
- Modify: `fractalwonder-ui/src/rendering/colorizers/mod.rs`

**Step 1: Write the failing test**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gradient_two_stops_endpoints() {
        let gradient = Gradient::new(vec![
            ColorStop { position: 0.0, color: [0, 0, 0] },
            ColorStop { position: 1.0, color: [255, 255, 255] },
        ]);
        let lut = gradient.to_lut();
        assert_eq!(lut[0], [0, 0, 0]);
        assert_eq!(lut[4095], [255, 255, 255]);
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p fractalwonder-ui gradient::tests --no-default-features`
Expected: FAIL - module not found

**Step 3: Write implementation**

Create `fractalwonder-ui/src/rendering/colorizers/gradient.rs`:

```rust
//! Color gradients with positioned stops and midpoints.

use super::color_space::{linear_rgb_to_oklab, linear_to_srgb, oklab_to_linear_rgb, srgb_to_linear};
use serde::{Deserialize, Serialize};

const LUT_SIZE: usize = 4096;

/// A color stop in the gradient.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ColorStop {
    pub position: f64,
    pub color: [u8; 3],
}

/// Color gradient with stops and midpoints.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Gradient {
    pub stops: Vec<ColorStop>,
    pub midpoints: Vec<f64>,
}

impl Gradient {
    /// Create a gradient from color stops with default midpoints (0.5).
    pub fn new(stops: Vec<ColorStop>) -> Self {
        let midpoint_count = if stops.len() > 1 { stops.len() - 1 } else { 0 };
        Self {
            stops,
            midpoints: vec![0.5; midpoint_count],
        }
    }

    /// Generate a 4096-entry LUT using OKLAB interpolation.
    pub fn to_lut(&self) -> Vec<[u8; 3]> {
        if self.stops.is_empty() {
            return vec![[0, 0, 0]; LUT_SIZE];
        }
        if self.stops.len() == 1 {
            return vec![self.stops[0].color; LUT_SIZE];
        }

        // Convert stops to OKLAB
        let oklab_stops: Vec<(f64, (f64, f64, f64))> = self
            .stops
            .iter()
            .map(|stop| {
                let r = srgb_to_linear(stop.color[0] as f64 / 255.0);
                let g = srgb_to_linear(stop.color[1] as f64 / 255.0);
                let b = srgb_to_linear(stop.color[2] as f64 / 255.0);
                (stop.position, linear_rgb_to_oklab(r, g, b))
            })
            .collect();

        (0..LUT_SIZE)
            .map(|i| {
                let t = i as f64 / (LUT_SIZE - 1) as f64;
                self.sample_oklab(&oklab_stops, t)
            })
            .collect()
    }

    fn sample_oklab(&self, oklab_stops: &[(f64, (f64, f64, f64))], t: f64) -> [u8; 3] {
        // Find segment
        let mut seg = 0;
        while seg < oklab_stops.len() - 1 && oklab_stops[seg + 1].0 < t {
            seg += 1;
        }
        if seg >= oklab_stops.len() - 1 {
            seg = oklab_stops.len() - 2;
        }

        let (pos0, (l0, a0, b0)) = oklab_stops[seg];
        let (pos1, (l1, a1, b1)) = oklab_stops[seg + 1];

        // Local t in segment
        let seg_t = if (pos1 - pos0).abs() < 1e-10 {
            0.0
        } else {
            ((t - pos0) / (pos1 - pos0)).clamp(0.0, 1.0)
        };

        // Apply midpoint bias
        let midpoint = self.midpoints.get(seg).copied().unwrap_or(0.5);
        let biased_t = apply_midpoint_bias(seg_t, midpoint);

        // Interpolate in OKLAB
        let l = l0 + biased_t * (l1 - l0);
        let a = a0 + biased_t * (a1 - a0);
        let b = b0 + biased_t * (b1 - b0);

        // Convert back to sRGB
        let (r, g, b) = oklab_to_linear_rgb(l, a, b);
        [
            (linear_to_srgb(r) * 255.0).round().clamp(0.0, 255.0) as u8,
            (linear_to_srgb(g) * 255.0).round().clamp(0.0, 255.0) as u8,
            (linear_to_srgb(b) * 255.0).round().clamp(0.0, 255.0) as u8,
        ]
    }
}

/// Apply midpoint bias to interpolation factor.
/// midpoint=0.5 is linear, <0.5 shifts toward start, >0.5 shifts toward end.
fn apply_midpoint_bias(t: f64, midpoint: f64) -> f64 {
    if (midpoint - 0.5).abs() < 1e-10 {
        return t;
    }

    // Use log-based bias for smooth adjustment
    if t <= 0.0 {
        return 0.0;
    }
    if t >= 1.0 {
        return 1.0;
    }

    // Map midpoint to exponent: midpoint=0.5 -> exp=1, midpoint<0.5 -> exp>1, midpoint>0.5 -> exp<1
    let exp = (0.5_f64).ln() / midpoint.clamp(0.01, 0.99).ln();
    t.powf(exp)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gradient_two_stops_endpoints() {
        let gradient = Gradient::new(vec![
            ColorStop { position: 0.0, color: [0, 0, 0] },
            ColorStop { position: 1.0, color: [255, 255, 255] },
        ]);
        let lut = gradient.to_lut();
        assert_eq!(lut[0], [0, 0, 0]);
        assert_eq!(lut[4095], [255, 255, 255]);
    }

    #[test]
    fn gradient_midpoint_affects_distribution() {
        let gradient_normal = Gradient::new(vec![
            ColorStop { position: 0.0, color: [0, 0, 0] },
            ColorStop { position: 1.0, color: [255, 255, 255] },
        ]);

        let mut gradient_biased = Gradient::new(vec![
            ColorStop { position: 0.0, color: [0, 0, 0] },
            ColorStop { position: 1.0, color: [255, 255, 255] },
        ]);
        gradient_biased.midpoints[0] = 0.25; // Shift toward dark

        let lut_normal = gradient_normal.to_lut();
        let lut_biased = gradient_biased.to_lut();

        // At midpoint of LUT, biased should be brighter (midpoint shifted left)
        let mid = LUT_SIZE / 2;
        assert!(
            lut_biased[mid][0] > lut_normal[mid][0],
            "Biased gradient should be brighter at midpoint"
        );
    }

    #[test]
    fn gradient_three_stops() {
        let gradient = Gradient::new(vec![
            ColorStop { position: 0.0, color: [255, 0, 0] },
            ColorStop { position: 0.5, color: [0, 255, 0] },
            ColorStop { position: 1.0, color: [0, 0, 255] },
        ]);
        let lut = gradient.to_lut();

        // Check endpoints
        assert_eq!(lut[0], [255, 0, 0]);
        assert_eq!(lut[4095], [0, 0, 255]);

        // At midpoint, should be close to green
        let mid = LUT_SIZE / 2;
        assert!(lut[mid][1] > lut[mid][0], "Green should dominate at midpoint");
        assert!(lut[mid][1] > lut[mid][2], "Green should dominate at midpoint");
    }

    #[test]
    fn midpoint_bias_identity() {
        assert!((apply_midpoint_bias(0.0, 0.5) - 0.0).abs() < 0.001);
        assert!((apply_midpoint_bias(0.5, 0.5) - 0.5).abs() < 0.001);
        assert!((apply_midpoint_bias(1.0, 0.5) - 1.0).abs() < 0.001);
    }

    #[test]
    fn midpoint_bias_shifts() {
        // Midpoint < 0.5: output > input (brighter earlier)
        assert!(apply_midpoint_bias(0.5, 0.25) > 0.5);
        // Midpoint > 0.5: output < input (darker earlier)
        assert!(apply_midpoint_bias(0.5, 0.75) < 0.5);
    }
}
```

**Step 4: Add module to mod.rs**

In `fractalwonder-ui/src/rendering/colorizers/mod.rs`, add:

```rust
pub mod gradient;
pub use gradient::{ColorStop, Gradient};
```

**Step 5: Run test to verify it passes**

Run: `cargo test -p fractalwonder-ui gradient::tests --no-default-features`
Expected: PASS (5 tests)

**Step 6: Commit**

```bash
git add fractalwonder-ui/src/rendering/colorizers/gradient.rs
git add fractalwonder-ui/src/rendering/colorizers/mod.rs
git commit -m "feat(palette): add ColorStop and Gradient with OKLAB interpolation and midpoints"
```

---

### Task 4: Create LightingParams struct

**Files:**
- Create: `fractalwonder-ui/src/rendering/colorizers/lighting_params.rs`
- Modify: `fractalwonder-ui/src/rendering/colorizers/mod.rs`

**Step 1: Write the failing test**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lighting_params_default_values() {
        let params = LightingParams::default();
        assert!((params.ambient - 0.75).abs() < 0.001);
        assert!((params.elevation - std::f64::consts::FRAC_PI_4).abs() < 0.001);
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p fractalwonder-ui lighting_params::tests --no-default-features`
Expected: FAIL - module not found

**Step 3: Write implementation**

Create `fractalwonder-ui/src/rendering/colorizers/lighting_params.rs`:

```rust
//! Blinn-Phong lighting parameters.

use serde::{Deserialize, Serialize};

/// Blinn-Phong lighting parameters for 3D shading.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LightingParams {
    pub ambient: f64,
    pub diffuse: f64,
    pub specular: f64,
    pub shininess: f64,
    pub strength: f64,
    pub azimuth: f64,
    pub elevation: f64,
}

impl Default for LightingParams {
    fn default() -> Self {
        Self {
            ambient: 0.75,
            diffuse: 0.5,
            specular: 0.9,
            shininess: 64.0,
            strength: 1.5,
            azimuth: -std::f64::consts::FRAC_PI_2,
            elevation: std::f64::consts::FRAC_PI_4,
        }
    }
}

impl LightingParams {
    /// Convert to the existing ShadingSettings format.
    pub fn to_shading_settings(&self, enabled: bool) -> super::ShadingSettings {
        super::ShadingSettings {
            enabled,
            light_azimuth: self.azimuth,
            light_elevation: self.elevation,
            ambient: self.ambient,
            diffuse: self.diffuse,
            specular: self.specular,
            shininess: self.shininess,
            strength: self.strength,
            distance_falloff: 10.0, // Default, will be replaced by falloff curve
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lighting_params_default_values() {
        let params = LightingParams::default();
        assert!((params.ambient - 0.75).abs() < 0.001);
        assert!((params.elevation - std::f64::consts::FRAC_PI_4).abs() < 0.001);
    }

    #[test]
    fn lighting_params_serialization() {
        let params = LightingParams::default();
        let json = serde_json::to_string(&params).unwrap();
        let parsed: LightingParams = serde_json::from_str(&json).unwrap();
        assert_eq!(params, parsed);
    }
}
```

**Step 4: Add module to mod.rs**

In `fractalwonder-ui/src/rendering/colorizers/mod.rs`, add:

```rust
pub mod lighting_params;
pub use lighting_params::LightingParams;
```

**Step 5: Run test to verify it passes**

Run: `cargo test -p fractalwonder-ui lighting_params::tests --no-default-features`
Expected: PASS (2 tests)

**Step 6: Commit**

```bash
git add fractalwonder-ui/src/rendering/colorizers/lighting_params.rs
git add fractalwonder-ui/src/rendering/colorizers/mod.rs
git commit -m "feat(palette): add LightingParams struct"
```

---

### Task 5: Create unified Palette struct

**Files:**
- Modify: `fractalwonder-ui/src/rendering/colorizers/palette.rs` (rename existing to `palette_lut.rs`)
- Create: `fractalwonder-ui/src/rendering/colorizers/palette.rs` (new unified struct)
- Modify: `fractalwonder-ui/src/rendering/colorizers/mod.rs`

**Step 1: Rename existing palette.rs to palette_lut.rs**

```bash
mv fractalwonder-ui/src/rendering/colorizers/palette.rs \
   fractalwonder-ui/src/rendering/colorizers/palette_lut.rs
```

**Step 2: Update mod.rs for the rename**

Change `pub mod palette;` to `pub mod palette_lut;` and update the use statement.

**Step 3: Write the failing test for new Palette**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn palette_default_has_valid_gradient() {
        let palette = Palette::default();
        assert!(palette.gradient.stops.len() >= 2);
    }

    #[test]
    fn palette_to_lut_returns_4096_entries() {
        let palette = Palette::default();
        let lut = palette.to_lut();
        assert_eq!(lut.len(), 4096);
    }
}
```

**Step 4: Write implementation**

Create `fractalwonder-ui/src/rendering/colorizers/palette.rs`:

```rust
//! Unified Palette struct containing gradient, curves, lighting, and flags.

use super::{ColorStop, Curve, Gradient, LightingParams};
use serde::{Deserialize, Serialize};

/// A complete palette configuration.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Palette {
    pub id: String,
    pub name: String,
    pub gradient: Gradient,
    pub transfer_curve: Curve,
    pub histogram_enabled: bool,
    pub smooth_enabled: bool,
    pub shading_enabled: bool,
    pub falloff_curve: Curve,
    pub lighting: LightingParams,
}

impl Default for Palette {
    fn default() -> Self {
        Self {
            id: "default".to_string(),
            name: "Default".to_string(),
            gradient: Gradient::new(vec![
                ColorStop { position: 0.0, color: [0, 0, 0] },
                ColorStop { position: 1.0, color: [255, 255, 255] },
            ]),
            transfer_curve: Curve::linear(),
            histogram_enabled: false,
            smooth_enabled: true,
            shading_enabled: false,
            falloff_curve: Curve::linear(),
            lighting: LightingParams::default(),
        }
    }
}

impl Palette {
    /// Generate a color lookup table from the gradient.
    pub fn to_lut(&self) -> Vec<[u8; 3]> {
        self.gradient.to_lut()
    }

    /// Apply the transfer curve to a normalized value.
    pub fn apply_transfer(&self, t: f64) -> f64 {
        self.transfer_curve.evaluate(t)
    }

    /// Apply the falloff curve to a distance value.
    pub fn apply_falloff(&self, t: f64) -> f64 {
        self.falloff_curve.evaluate(t)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn palette_default_has_valid_gradient() {
        let palette = Palette::default();
        assert!(palette.gradient.stops.len() >= 2);
    }

    #[test]
    fn palette_to_lut_returns_4096_entries() {
        let palette = Palette::default();
        let lut = palette.to_lut();
        assert_eq!(lut.len(), 4096);
    }

    #[test]
    fn palette_transfer_curve_identity() {
        let palette = Palette::default();
        assert!((palette.apply_transfer(0.0) - 0.0).abs() < 0.001);
        assert!((palette.apply_transfer(0.5) - 0.5).abs() < 0.001);
        assert!((palette.apply_transfer(1.0) - 1.0).abs() < 0.001);
    }

    #[test]
    fn palette_serialization() {
        let palette = Palette::default();
        let json = serde_json::to_string(&palette).unwrap();
        let parsed: Palette = serde_json::from_str(&json).unwrap();
        assert_eq!(palette.id, parsed.id);
        assert_eq!(palette.name, parsed.name);
    }
}
```

**Step 5: Update mod.rs**

```rust
pub mod color_space;
pub mod colorizer;
pub mod curve;
pub mod gradient;
pub mod lighting_params;
pub mod palette;
pub mod palette_lut;
pub mod pipeline;
pub mod settings;
pub mod shading;
pub mod smooth_iteration;

// Re-exports
pub use colorizer::{Colorizer, ColorizerKind};
pub use curve::{Curve, CurvePoint};
pub use gradient::{ColorStop, Gradient};
pub use lighting_params::LightingParams;
pub use palette::Palette;
pub use palette_lut::PaletteLut;
pub use pipeline::ColorPipeline;
pub use settings::{apply_transfer_bias, ColorOptions, ShadingSettings};
pub use shading::apply_slope_shading;
pub use smooth_iteration::{SmoothIterationColorizer, SmoothIterationContext};
```

**Step 6: Rename Palette to PaletteLut in palette_lut.rs**

In `palette_lut.rs`, rename `pub struct Palette` to `pub struct PaletteLut`.

**Step 7: Run tests to verify**

Run: `cargo test -p fractalwonder-ui palette::tests --no-default-features`
Expected: PASS (4 tests)

**Step 8: Commit**

```bash
git add fractalwonder-ui/src/rendering/colorizers/
git commit -m "feat(palette): add unified Palette struct, rename old Palette to PaletteLut"
```

---

## Phase 2: Factory Defaults

### Task 6: Implement factory default palettes

**Files:**
- Modify: `fractalwonder-ui/src/rendering/colorizers/palette.rs`

**Step 1: Write the failing test**

```rust
#[test]
fn factory_defaults_contains_classic() {
    let palettes = Palette::factory_defaults();
    assert!(palettes.iter().any(|p| p.id == "classic"));
}

#[test]
fn factory_defaults_all_have_unique_ids() {
    let palettes = Palette::factory_defaults();
    let mut ids: Vec<_> = palettes.iter().map(|p| &p.id).collect();
    ids.sort();
    ids.dedup();
    assert_eq!(ids.len(), palettes.len());
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p fractalwonder-ui palette::tests::factory_defaults --no-default-features`
Expected: FAIL - method not found

**Step 3: Implement factory_defaults**

Add to `palette.rs`:

```rust
impl Palette {
    // ... existing methods ...

    /// Factory default palettes built into the binary.
    pub fn factory_defaults() -> Vec<Palette> {
        vec![
            Self::classic(),
            Self::fire(),
            Self::ocean(),
            Self::electric(),
            Self::grayscale(),
            Self::rainbow(),
            Self::neon(),
            Self::twilight(),
            Self::candy(),
            Self::inferno(),
            Self::aurora(),
        ]
    }

    fn classic() -> Self {
        Self {
            id: "classic".to_string(),
            name: "Classic".to_string(),
            gradient: Gradient::new(vec![
                ColorStop { position: 0.0, color: [0, 7, 100] },
                ColorStop { position: 0.16, color: [0, 2, 0] },
                ColorStop { position: 0.33, color: [0, 7, 100] },
                ColorStop { position: 0.5, color: [32, 107, 203] },
                ColorStop { position: 0.66, color: [255, 170, 0] },
                ColorStop { position: 1.0, color: [237, 255, 255] },
            ]),
            ..Self::default()
        }
    }

    fn fire() -> Self {
        Self {
            id: "fire".to_string(),
            name: "Fire".to_string(),
            gradient: Gradient::new(vec![
                ColorStop { position: 0.0, color: [0, 0, 0] },
                ColorStop { position: 0.2, color: [128, 0, 0] },
                ColorStop { position: 0.4, color: [255, 0, 0] },
                ColorStop { position: 0.6, color: [255, 128, 0] },
                ColorStop { position: 0.8, color: [255, 255, 0] },
                ColorStop { position: 1.0, color: [255, 255, 255] },
            ]),
            ..Self::default()
        }
    }

    fn ocean() -> Self {
        Self {
            id: "ocean".to_string(),
            name: "Ocean".to_string(),
            gradient: Gradient::new(vec![
                ColorStop { position: 0.0, color: [0, 0, 64] },
                ColorStop { position: 0.25, color: [0, 64, 128] },
                ColorStop { position: 0.5, color: [0, 128, 192] },
                ColorStop { position: 0.75, color: [64, 192, 255] },
                ColorStop { position: 1.0, color: [255, 255, 255] },
            ]),
            ..Self::default()
        }
    }

    fn electric() -> Self {
        Self {
            id: "electric".to_string(),
            name: "Electric".to_string(),
            gradient: Gradient::new(vec![
                ColorStop { position: 0.0, color: [32, 0, 64] },
                ColorStop { position: 0.2, color: [64, 0, 128] },
                ColorStop { position: 0.4, color: [0, 0, 255] },
                ColorStop { position: 0.6, color: [0, 255, 255] },
                ColorStop { position: 0.8, color: [0, 255, 0] },
                ColorStop { position: 1.0, color: [255, 255, 0] },
            ]),
            ..Self::default()
        }
    }

    fn grayscale() -> Self {
        Self {
            id: "grayscale".to_string(),
            name: "Grayscale".to_string(),
            gradient: Gradient::new(vec![
                ColorStop { position: 0.0, color: [0, 0, 0] },
                ColorStop { position: 1.0, color: [255, 255, 255] },
            ]),
            ..Self::default()
        }
    }

    fn rainbow() -> Self {
        Self {
            id: "rainbow".to_string(),
            name: "Rainbow".to_string(),
            gradient: Gradient::new(vec![
                ColorStop { position: 0.0, color: [255, 0, 0] },
                ColorStop { position: 0.17, color: [255, 127, 0] },
                ColorStop { position: 0.33, color: [255, 255, 0] },
                ColorStop { position: 0.5, color: [0, 255, 0] },
                ColorStop { position: 0.67, color: [0, 0, 255] },
                ColorStop { position: 0.83, color: [75, 0, 130] },
                ColorStop { position: 1.0, color: [148, 0, 211] },
            ]),
            ..Self::default()
        }
    }

    fn neon() -> Self {
        Self {
            id: "neon".to_string(),
            name: "Neon".to_string(),
            gradient: Gradient::new(vec![
                ColorStop { position: 0.0, color: [255, 0, 255] },
                ColorStop { position: 0.33, color: [0, 255, 255] },
                ColorStop { position: 0.67, color: [255, 255, 0] },
                ColorStop { position: 1.0, color: [255, 0, 255] },
            ]),
            ..Self::default()
        }
    }

    fn twilight() -> Self {
        Self {
            id: "twilight".to_string(),
            name: "Twilight".to_string(),
            gradient: Gradient::new(vec![
                ColorStop { position: 0.0, color: [255, 100, 50] },
                ColorStop { position: 0.2, color: [200, 50, 150] },
                ColorStop { position: 0.4, color: [80, 80, 220] },
                ColorStop { position: 0.6, color: [50, 150, 255] },
                ColorStop { position: 0.8, color: [150, 200, 150] },
                ColorStop { position: 1.0, color: [255, 100, 50] },
            ]),
            ..Self::default()
        }
    }

    fn candy() -> Self {
        Self {
            id: "candy".to_string(),
            name: "Candy".to_string(),
            gradient: Gradient::new(vec![
                ColorStop { position: 0.0, color: [255, 180, 200] },
                ColorStop { position: 0.25, color: [200, 180, 255] },
                ColorStop { position: 0.5, color: [180, 255, 220] },
                ColorStop { position: 0.75, color: [255, 240, 180] },
                ColorStop { position: 1.0, color: [255, 180, 200] },
            ]),
            ..Self::default()
        }
    }

    fn inferno() -> Self {
        Self {
            id: "inferno".to_string(),
            name: "Inferno".to_string(),
            gradient: Gradient::new(vec![
                ColorStop { position: 0.0, color: [5, 0, 10] },
                ColorStop { position: 0.25, color: [100, 10, 10] },
                ColorStop { position: 0.5, color: [255, 100, 0] },
                ColorStop { position: 0.75, color: [255, 180, 50] },
                ColorStop { position: 1.0, color: [200, 150, 100] },
            ]),
            ..Self::default()
        }
    }

    fn aurora() -> Self {
        Self {
            id: "aurora".to_string(),
            name: "Aurora".to_string(),
            gradient: Gradient::new(vec![
                ColorStop { position: 0.0, color: [50, 255, 100] },
                ColorStop { position: 0.25, color: [50, 200, 255] },
                ColorStop { position: 0.5, color: [150, 80, 255] },
                ColorStop { position: 0.75, color: [100, 200, 100] },
                ColorStop { position: 1.0, color: [50, 255, 100] },
            ]),
            ..Self::default()
        }
    }
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p fractalwonder-ui palette::tests --no-default-features`
Expected: PASS (all tests)

**Step 5: Commit**

```bash
git add fractalwonder-ui/src/rendering/colorizers/palette.rs
git commit -m "feat(palette): implement factory default palettes"
```

---

## Phase 3: Persistence

### Task 7: Implement localStorage persistence

**Files:**
- Modify: `fractalwonder-ui/src/rendering/colorizers/palette.rs`

**Step 1: Add persistence methods**

```rust
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsValue;
#[cfg(target_arch = "wasm32")]
use web_sys::window;

impl Palette {
    /// Save palette to localStorage.
    #[cfg(target_arch = "wasm32")]
    pub fn save(&self) -> Result<(), JsValue> {
        let storage = window()
            .ok_or("no window")?
            .local_storage()
            .map_err(|_| "localStorage error")?
            .ok_or("no localStorage")?;

        let json = serde_json::to_string(self).map_err(|e| e.to_string())?;
        storage.set_item(&format!("palette:{}", self.id), &json)
    }

    /// Load palette from localStorage.
    #[cfg(target_arch = "wasm32")]
    pub fn load(id: &str) -> Option<Self> {
        let storage = window()?.local_storage().ok()??;
        let json = storage.get_item(&format!("palette:{id}")).ok()??;
        serde_json::from_str(&json).ok()
    }

    /// Delete palette from localStorage.
    #[cfg(target_arch = "wasm32")]
    pub fn delete(id: &str) {
        if let Some(storage) = window().and_then(|w| w.local_storage().ok().flatten()) {
            let _ = storage.remove_item(&format!("palette:{id}"));
        }
    }

    /// Get palette by ID: localStorage first, then factory default.
    #[cfg(target_arch = "wasm32")]
    pub fn get(id: &str) -> Option<Self> {
        Self::load(id).or_else(|| Self::factory_defaults().into_iter().find(|p| p.id == id))
    }

    /// Non-WASM stubs for testing.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn save(&self) -> Result<(), String> {
        Ok(()) // No-op in tests
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn load(_id: &str) -> Option<Self> {
        None // No localStorage in tests
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn delete(_id: &str) {
        // No-op in tests
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn get(id: &str) -> Option<Self> {
        Self::factory_defaults().into_iter().find(|p| p.id == id)
    }
}
```

**Step 2: Add test for get() fallback**

```rust
#[test]
fn palette_get_returns_factory_default() {
    let palette = Palette::get("classic");
    assert!(palette.is_some());
    assert_eq!(palette.unwrap().id, "classic");
}

#[test]
fn palette_get_returns_none_for_unknown() {
    let palette = Palette::get("nonexistent");
    assert!(palette.is_none());
}
```

**Step 3: Run tests**

Run: `cargo test -p fractalwonder-ui palette::tests --no-default-features`
Expected: PASS

**Step 4: Commit**

```bash
git add fractalwonder-ui/src/rendering/colorizers/palette.rs
git commit -m "feat(palette): implement localStorage persistence with factory shadowing"
```

---

## Phase 4: Create RenderSettings

### Task 8: Create RenderSettings struct

**Files:**
- Create: `fractalwonder-ui/src/rendering/colorizers/render_settings.rs`
- Modify: `fractalwonder-ui/src/rendering/colorizers/mod.rs`

**Step 1: Write the failing test**

```rust
#[test]
fn render_settings_default_cycle_count_is_one() {
    let settings = RenderSettings::default();
    assert_eq!(settings.cycle_count, 1);
}
```

**Step 2: Implement RenderSettings**

Create `fractalwonder-ui/src/rendering/colorizers/render_settings.rs`:

```rust
//! Runtime render settings separate from palette.

use serde::{Deserialize, Serialize};

/// Runtime settings that are not persisted with the palette.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RenderSettings {
    pub cycle_count: u32,
    pub use_gpu: bool,
    pub xray_enabled: bool,
}

impl Default for RenderSettings {
    fn default() -> Self {
        Self {
            cycle_count: 1,
            use_gpu: true,
            xray_enabled: false,
        }
    }
}

impl RenderSettings {
    pub fn cycle_up(&mut self) {
        self.cycle_count = (self.cycle_count + 1).min(1024);
    }

    pub fn cycle_down(&mut self) {
        self.cycle_count = self.cycle_count.saturating_sub(1).max(1);
    }

    pub fn cycle_up_by(&mut self, amount: u32) {
        self.cycle_count = (self.cycle_count + amount).min(1024);
    }

    pub fn cycle_down_by(&mut self, amount: u32) {
        self.cycle_count = self.cycle_count.saturating_sub(amount).max(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_settings_default_cycle_count_is_one() {
        let settings = RenderSettings::default();
        assert_eq!(settings.cycle_count, 1);
    }

    #[test]
    fn render_settings_cycle_bounds() {
        let mut settings = RenderSettings::default();
        settings.cycle_count = 1024;
        settings.cycle_up();
        assert_eq!(settings.cycle_count, 1024);

        settings.cycle_count = 1;
        settings.cycle_down();
        assert_eq!(settings.cycle_count, 1);
    }
}
```

**Step 3: Add to mod.rs**

```rust
pub mod render_settings;
pub use render_settings::RenderSettings;
```

**Step 4: Run tests**

Run: `cargo test -p fractalwonder-ui render_settings::tests --no-default-features`
Expected: PASS

**Step 5: Commit**

```bash
git add fractalwonder-ui/src/rendering/colorizers/render_settings.rs
git add fractalwonder-ui/src/rendering/colorizers/mod.rs
git commit -m "feat(palette): add RenderSettings struct"
```

---

## Phase 5: Integration

### Task 9: Run full test suite and fix any issues

**Step 1: Run cargo check**

```bash
cargo check --workspace --all-targets --all-features
```

Fix any compilation errors.

**Step 2: Run clippy**

```bash
cargo clippy --all-targets --all-features -- -D warnings -W clippy::all
```

Fix any warnings.

**Step 3: Run all tests**

```bash
cargo test --workspace --all-targets --all-features -- --nocapture
```

**Step 4: Format code**

```bash
cargo fmt --all
```

**Step 5: Commit fixes**

```bash
git add -A
git commit -m "fix: address clippy warnings and test failures"
```

---

### Task 10: Update ARCHITECTURE.md with final module structure

**Files:**
- Modify: `docs/ux-palette-editor/ARCHITECTURE.md`

**Step 1: Add module listing**

Update the Rust Implementation Architecture section to include:

```markdown
### Module Structure

| Module | Purpose |
|--------|---------|
| `curve.rs` | CurvePoint, Curve with cubic spline interpolation |
| `gradient.rs` | ColorStop, Gradient with OKLAB and midpoints |
| `lighting_params.rs` | LightingParams for Blinn-Phong |
| `palette.rs` | Unified Palette struct with factory defaults and persistence |
| `palette_lut.rs` | PaletteLut (formerly Palette) for LUT generation |
| `render_settings.rs` | RenderSettings for cycle_count, use_gpu, xray |
```

**Step 2: Commit**

```bash
git add docs/ux-palette-editor/ARCHITECTURE.md
git commit -m "docs: update ARCHITECTURE.md with final module structure"
```

---

## Summary

This plan creates 8 new/modified files:
1. `curve.rs` - CurvePoint and Curve with cubic spline
2. `gradient.rs` - ColorStop and Gradient with OKLAB + midpoints
3. `lighting_params.rs` - LightingParams struct
4. `palette.rs` - Unified Palette with factory defaults and persistence
5. `palette_lut.rs` - Renamed from palette.rs (PaletteLut)
6. `render_settings.rs` - RenderSettings struct
7. `mod.rs` - Updated exports
8. `ARCHITECTURE.md` - Documentation updates

Total: 10 tasks, approximately 60-90 minutes of implementation.
