# Advanced Fractal Coloring Algorithms Research

This document provides comprehensive research on advanced colorizing algorithms used by state-of-the-art
Mandelbrot set renderers. These techniques are essential for creating visually appealing and mathematically
meaningful visualizations, especially at extreme zoom levels.

## Table of Contents

1. [Smooth Iteration Count](#1-smooth-iteration-count)
2. [Distance Estimation Method (DEM)](#2-distance-estimation-method-dem)
3. [Histogram Coloring](#3-histogram-coloring)
4. [Exponential Smoothing](#4-exponential-smoothing)
5. [Slope/Normal-Based Shading](#5-slopenormal-based-shading)
6. [Implementation Considerations](#6-implementation-considerations)
7. [Algorithm Comparison](#7-algorithm-comparison)
8. [Implementation Roadmap](#8-implementation-roadmap)
9. [References](#9-references)

---

## 1. Smooth Iteration Count

### Overview

The smooth iteration count (also called "normalized iteration count" or "continuous potential") eliminates the
discrete banding that occurs with standard escape-time coloring. Instead of integer iteration counts, it produces
continuous fractional values that create smooth color gradients.

### Mathematical Foundation

The technique is rooted in complex potential theory. For the Mandelbrot iteration `z_{n+1} = z_n² + c`, when a point
escapes (|z_n| > R), we can compute a fractional iteration count.

**Core Insight** (Earl L. Hinrichs): The orbit grows as z, z², z⁴, z⁸... Taking log gives 1, 2, 4, 8, 16. Taking
log again gives 0, 1, 2, 3, 4 - matching integer iterations. The double logarithm generalizes this to continuous
values.

### Formula

For the standard Mandelbrot set (degree d = 2):

```
μ = n + 1 - log₂(log₂|z_n|)
```

Or equivalently:

```
μ = n - log₂(log_R(|z_n|))
```

Where:
- `n` = discrete iteration count (when |z_n| > R)
- `z_n` = the escape value
- `R` = escape radius (typically 2, but larger values like 256 improve smoothness)

**Generalized formula** for degree-d polynomials `z_{n+1} = z_n^d + c`:

```
μ = n + 1 - log_d(log|z_n|)
```

### GLSL Implementation

```glsl
float smoothIterationCount(vec2 c) {
    const float B = 256.0;  // Large escape radius for better smoothing
    float n = 0.0;
    vec2 z = vec2(0.0);

    for (int i = 0; i < MAX_ITER; i++) {
        z = vec2(z.x*z.x - z.y*z.y, 2.0*z.x*z.y) + c;
        if (dot(z, z) > B*B) break;
        n += 1.0;
    }

    // Smooth iteration count formula
    return n - log2(log2(dot(z, z))) + 4.0;
}
```

### Rust Implementation

```rust
fn smooth_iteration_count(c: Complex64, max_iter: u32) -> f64 {
    const ESCAPE_RADIUS: f64 = 256.0;
    let escape_radius_sq = ESCAPE_RADIUS * ESCAPE_RADIUS;

    let mut z = Complex64::new(0.0, 0.0);
    let mut n = 0u32;

    while n < max_iter {
        z = z * z + c;
        if z.norm_sqr() > escape_radius_sq {
            break;
        }
        n += 1;
    }

    if n == max_iter {
        return f64::from(max_iter);  // Interior point
    }

    // Smooth iteration count
    let log_zn = z.norm_sqr().ln() / 2.0;  // ln(|z|)
    let nu = log_zn.ln() / 2.0_f64.ln();   // log_2(ln(|z|))
    f64::from(n) + 1.0 - nu
}
```

### Properties

- **Continuity**: The smooth iteration count is continuous as a function of c
- **Independence**: Nearly independent of escape radius R (in the limit R → ∞)
- **Differentiability**: Piecewise-continuous and differentiable
- **Connection to potential**: Related to the Douady-Hubbard potential and external rays

### Pros

- Eliminates color banding completely
- Computationally cheap (just a few extra operations)
- Works with any color palette
- Enables smooth gradients and interpolation

### Cons

- Only applies to exterior (escaping) points
- Requires floating-point precision
- Very deep zooms may need higher precision for the logarithms

### Requirements for Implementation

- Floating-point arithmetic for the iteration count
- Reasonably large escape radius (256 recommended, minimum 2)
- Access to logarithm functions

---

## 2. Distance Estimation Method (DEM)

### Overview

The Distance Estimation Method computes the approximate Euclidean distance from a point c to the Mandelbrot set
boundary. This allows rendering of features smaller than pixel size and creates distinctive visual effects
emphasizing the fractal boundary.

### Historical Context

Pioneered by Thurston and made known to the general community by Peitgen and Richter in "The Beauty of Fractals"
(1986). A precise algorithm is given in "The Science of Fractal Images" (1988), page 198, called DEM/M.

### Mathematical Foundation

The method uses the derivative of the iteration function. The distance estimate is derived from the
Koebe 1/4-theorem, which guarantees that if b is the derivative magnitude, no point of M has distance from c
smaller than b/4.

**Derivative recurrence relation**:

For `z_{n+1} = z_n² + c`:

```
ρ_{n+1} = 2·z_n·ρ_n + 1
```

Where ρ_n = dz_n/dc, with ρ_0 = 0.

**Distance estimate formula** (Milnor):

```
d_n = (2·|z_n|·ln|z_n|) / |ρ_n|
```

Or equivalently:

```
d_n = |z_n|·ln(|z_n|²) / |ρ_n|
```

### Algorithm

```
Input: c (complex point), max_iter, escape_radius R
Output: distance estimate (or 0 if inside M)

z = 0
dz = 0  // derivative

for n = 1 to max_iter:
    // Update derivative FIRST (uses old z)
    dz = 2 * z * dz + 1

    // Then update z
    z = z² + c

    if |z| > R:
        // Point escaped - compute distance
        distance = |z| * ln(|z|²) / |dz|
        return distance

return 0  // Didn't escape - inside M (or increase max_iter)
```

### Expanded Real/Imaginary Implementation

```rust
fn distance_estimate(cr: f64, ci: f64, max_iter: u32) -> f64 {
    const ESCAPE_RADIUS: f64 = 1000.0;  // Large radius improves accuracy
    let escape_sq = ESCAPE_RADIUS * ESCAPE_RADIUS;

    let mut zr = 0.0_f64;
    let mut zi = 0.0_f64;
    let mut dzr = 0.0_f64;  // d(z)/d(c) real part
    let mut dzi = 0.0_f64;  // d(z)/d(c) imaginary part

    for _ in 0..max_iter {
        // Derivative: dz' = 2*z*dz + 1
        // (zr + i*zi) * (dzr + i*dzi) = zr*dzr - zi*dzi + i*(zr*dzi + zi*dzr)
        let dzr_new = 2.0 * (zr * dzr - zi * dzi) + 1.0;
        let dzi_new = 2.0 * (zr * dzi + zi * dzr);
        dzr = dzr_new;
        dzi = dzi_new;

        // z' = z² + c
        let zr_new = zr * zr - zi * zi + cr;
        let zi_new = 2.0 * zr * zi + ci;
        zr = zr_new;
        zi = zi_new;

        let z_mag_sq = zr * zr + zi * zi;
        if z_mag_sq > escape_sq {
            let z_mag = z_mag_sq.sqrt();
            let dz_mag = (dzr * dzr + dzi * dzi).sqrt();

            // Distance estimate: |z| * ln(|z|²) / |dz|
            return z_mag * z_mag_sq.ln() / dz_mag;
        }
    }

    0.0  // Interior point
}
```

### Color Mapping

The distance can be mapped to colors in several ways:

**1. Logarithmic mapping** (most common):
```
color_index = -k * ln(distance)
where k = palette_size / ln(max_zoom_magnification)
```

**2. Boundary emphasis** (HSV adjustment):
```rust
fn distance_to_color(distance: f64, pixel_size: f64) -> Color {
    let t = (distance / (thickness_factor * pixel_size)).clamp(0.0, 1.0);
    // Darken pixels near boundary
    hsv_to_rgb(hue, saturation, value * t)
}
```

**3. Anti-aliasing without oversampling**:
```rust
// Interpolate between boundary color and exterior color based on distance
let t = (distance / pixel_size).clamp(0.0, 1.0);
lerp(boundary_color, exterior_color, t)
```

### Interior Distance Estimation

For points inside the Mandelbrot set, a different approach is needed:

1. Detect the attracting cycle (period detection)
2. Compute the derivative with respect to the cycle
3. Estimate distance to the boundary from inside

This is significantly more complex and computationally expensive.

### Pros

- Renders filaments and thin structures accurately
- Detects features smaller than pixels
- Enables boundary-aware anti-aliasing
- Creates striking visual effects
- Independent of palette choice

### Cons

- Requires computing and storing the derivative (doubles memory for iteration)
- Less meaningful for deep interior regions
- Approximate (accurate to within factor of 1/5 to 1)
- Noise/artifacts can appear at very deep zooms

### Requirements for Implementation

- Complex number derivative tracking alongside main iteration
- Higher escape radius (1000+ recommended)
- Sufficient precision for derivative calculations at deep zoom

---

## 3. Histogram Coloring

### Overview

Histogram coloring (histogram equalization) is a post-processing technique that distributes colors evenly across
the rendered image based on the frequency distribution of iteration counts. This prevents dominant iteration
values from overwhelming the image and ensures details at all scales are visible.

### Historical Context

Popularized by early fractal software, notably Fractint (late 1980s-1990s). The technique addresses a fundamental
problem: at any given zoom level, certain iteration counts dominate, wasting most of the color palette on a few
bands while leaving intricate details nearly invisible.

### Algorithm

**Pass 1: Compute iteration counts**
```
for each pixel (x, y):
    iterations[x][y] = compute_escape_iterations(pixel_to_complex(x, y))
```

**Pass 2: Build histogram**
```
histogram = array of size max_iterations, initialized to 0
for each pixel (x, y):
    n = iterations[x][y]
    if n < max_iterations:  // Exclude interior points
        histogram[n] += 1
```

**Pass 3: Compute CDF (Cumulative Distribution Function)**
```
total = sum(histogram)
cdf[0] = histogram[0]
for i = 1 to max_iterations:
    cdf[i] = cdf[i-1] + histogram[i]

// Normalize to [0, 1]
for i = 0 to max_iterations:
    cdf[i] = cdf[i] / total
```

**Pass 4: Map colors**
```
for each pixel (x, y):
    n = iterations[x][y]
    if n == max_iterations:
        color = interior_color
    else:
        normalized = cdf[n]  // Value in [0, 1]
        color = palette[normalized * (palette_size - 1)]
```

### Rust Implementation

```rust
struct HistogramColorizer {
    histogram: Vec<u64>,
    cdf: Vec<f64>,
    total_exterior: u64,
}

impl HistogramColorizer {
    fn new(max_iter: usize) -> Self {
        Self {
            histogram: vec![0; max_iter],
            cdf: vec![0.0; max_iter],
            total_exterior: 0,
        }
    }

    fn build_histogram(&mut self, iterations: &[u32], max_iter: u32) {
        self.histogram.fill(0);
        self.total_exterior = 0;

        for &n in iterations {
            if n < max_iter {
                self.histogram[n as usize] += 1;
                self.total_exterior += 1;
            }
        }
    }

    fn compute_cdf(&mut self) {
        if self.total_exterior == 0 {
            return;
        }

        let mut cumulative = 0u64;
        for i in 0..self.histogram.len() {
            cumulative += self.histogram[i];
            self.cdf[i] = cumulative as f64 / self.total_exterior as f64;
        }
    }

    fn get_normalized_value(&self, iteration: u32, max_iter: u32) -> f64 {
        if iteration >= max_iter {
            return 1.0;  // Interior
        }
        self.cdf[iteration as usize]
    }
}
```

### Combining with Smooth Iteration Count

For best results, combine histogram equalization with smooth iteration counts:

```rust
fn histogram_smooth_color(
    smooth_iter: f64,
    histogram: &[u64],
    cdf: &[f64],
    max_iter: u32,
) -> f64 {
    let n = smooth_iter.floor() as usize;
    let frac = smooth_iter.fract();

    if n >= max_iter as usize - 1 {
        return 1.0;
    }

    // Interpolate between CDF values
    let cdf_low = cdf[n];
    let cdf_high = cdf[n + 1];
    cdf_low + frac * (cdf_high - cdf_low)
}
```

### Pros

- Automatically adapts to any zoom level
- No wasted palette entries
- Reveals details at all scales equally
- Independent of max iteration count choice
- Enhances visual contrast significantly

### Cons

- Requires two passes (cannot color during iteration)
- Memory overhead for storing all iteration counts
- Global operation (entire image must be computed first)
- Can produce unusual colors in sparsely-populated regions

### Requirements for Implementation

- Storage for iteration counts (one value per pixel)
- Histogram array (size = max iterations)
- CDF array (size = max iterations)
- Two-pass rendering pipeline

---

## 4. Exponential Smoothing

### Overview

Exponential smoothing accumulates a weighted sum over the entire orbit, producing smooth coloring that works for
both divergent (Mandelbrot-type) and convergent (Newton-type) fractals. It was developed by Ron Barnett and
implemented by Damien M. Jones.

### Mathematical Foundation

Instead of using only the final iteration count, we accumulate a sum over all iterations:

**For divergent fractals** (orbit escapes to infinity):
```
sum = Σ exp(-|z_n|)
```

As |z| increases, exp(-|z|) approaches zero, so the sum converges.

**For convergent fractals** (orbit converges to a fixed point):
```
sum = Σ exp(-1 / |z_n - z_{n-1}|)
```

As the orbit converges, |z_n - z_{n-1}| approaches zero, so exp(-1/|...|) approaches zero.

### Algorithm

```
Input: c (complex point), max_iter
Output: smoothed value for coloring

z = 0
z_old = 0
sum_divergent = 0
sum_convergent = 0

for n = 1 to max_iter:
    z_old = z
    z = z² + c

    // Divergent contribution
    sum_divergent += exp(-|z|)

    // Convergent contribution (avoid division by zero)
    step = |z - z_old|
    if step > epsilon:
        sum_convergent += exp(-1 / step)

    if |z| > escape_radius:
        return sum_divergent  // Use divergent sum

// Point didn't escape - use convergent sum or mark as interior
return sum_convergent
```

### Rust Implementation

```rust
fn exponential_smoothing(c: Complex64, max_iter: u32) -> f64 {
    const ESCAPE_RADIUS: f64 = 4.0;
    const EPSILON: f64 = 1e-10;

    let mut z = Complex64::new(0.0, 0.0);
    let mut z_old = z;
    let mut sum = 0.0_f64;

    for _ in 0..max_iter {
        z_old = z;
        z = z * z + c;

        // Accumulate exponential decay based on magnitude
        let z_abs = z.norm();
        sum += (-z_abs).exp();

        if z.norm_sqr() > ESCAPE_RADIUS * ESCAPE_RADIUS {
            return sum;
        }
    }

    // For interior/non-escaping points, could return a different value
    // or use the convergent sum approach
    sum
}
```

### Extended Implementation (Divergent + Convergent)

```rust
struct ExpSmoothResult {
    divergent_sum: f64,
    convergent_sum: f64,
    escaped: bool,
}

fn exponential_smoothing_full(c: Complex64, max_iter: u32) -> ExpSmoothResult {
    const ESCAPE_RADIUS_SQ: f64 = 16.0;
    const EPSILON: f64 = 1e-12;

    let mut z = Complex64::new(0.0, 0.0);
    let mut z_old = z;
    let mut div_sum = 0.0_f64;
    let mut conv_sum = 0.0_f64;

    for _ in 0..max_iter {
        z_old = z;
        z = z * z + c;

        // Divergent: exp(-|z|)
        div_sum += (-z.norm()).exp();

        // Convergent: exp(-1/|z - z_old|)
        let step = (z - z_old).norm();
        if step > EPSILON {
            conv_sum += (-1.0 / step).exp();
        }

        if z.norm_sqr() > ESCAPE_RADIUS_SQ {
            return ExpSmoothResult {
                divergent_sum: div_sum,
                convergent_sum: conv_sum,
                escaped: true,
            };
        }
    }

    ExpSmoothResult {
        divergent_sum: div_sum,
        convergent_sum: conv_sum,
        escaped: false,
    }
}
```

### Pros

- Works for all fractal types (divergent, convergent, mixed)
- Produces smooth results without explicit smoothing formula
- Natural gradient near basin boundaries
- Doesn't require special handling for different fractal formulas

### Cons

- More expensive (one exp() call per iteration)
- Doesn't map directly to iteration count
- Result range varies with max_iter and location
- May need normalization for consistent color mapping

### Requirements for Implementation

- Exponential function (exp)
- Tracking of previous z value for convergent variant
- Floating-point accumulator

---

## 5. Slope/Normal-Based Shading

### Overview

Slope shading (also called "normal mapping" or "3D lighting") treats the iteration count field as a height map
and computes surface normals to apply lighting effects. This creates a striking 3D appearance that emphasizes
the fractal structure.

### Approaches

There are two main approaches:

1. **Finite Difference Method**: Post-processing using neighboring pixel values
2. **Analytic Derivative Method**: Computing normals from the mathematical derivative during iteration

### Method 1: Finite Difference (8-Neighbor)

This method compares each pixel's iteration count to its 8 neighbors to estimate the local slope.

**Algorithm** (from fractalforums.org, implemented by philthompson.me):

```
Input: iterations[width][height] (smooth iteration counts)
Output: shaded colors

for each pixel (x, y) not on border:
    if is_interior(x, y):
        continue  // Skip Mandelbrot interior

    subject = iterations[x][y]
    running_sum = 0
    high = subject
    low = subject

    // Process 8 neighbors
    for each neighbor (nx, ny) of (x, y):
        neighbor_val = iterations[nx][ny]
        high = max(high, neighbor_val)
        low = min(low, neighbor_val)

        diff = neighbor_val - subject

        // Apply based on neighbor position and light direction
        // Light from top-right: negate for left/bottom neighbors
        h_diff = diff
        v_diff = diff

        if nx < x: h_diff = -h_diff  // Left neighbor
        if ny > y: v_diff = -v_diff  // Bottom neighbor

        // Accumulate based on position
        if nx != x: running_sum += h_diff
        if ny != y: running_sum += v_diff

    // Compute slope factor
    range = high - low
    if range > 0:
        slope = (running_sum * height_factor) / range
    else:
        slope = 0

    // Apply to color
    color = base_color + slope  // Clamp to valid range
```

**Rust Implementation**:

```rust
fn apply_slope_shading(
    iterations: &[f64],
    width: usize,
    height: usize,
    height_factor: f64,
) -> Vec<f64> {
    let mut shading = vec![0.0; width * height];

    for y in 1..height-1 {
        for x in 1..width-1 {
            let idx = y * width + x;
            let subject = iterations[idx];

            // Skip interior points
            if subject >= f64::MAX * 0.9 {
                continue;
            }

            let mut running_sum = 0.0;
            let mut high = subject;
            let mut low = subject;

            // 8 neighbors with their relative positions
            let neighbors = [
                (-1, -1), (0, -1), (1, -1),
                (-1,  0),          (1,  0),
                (-1,  1), (0,  1), (1,  1),
            ];

            for (dx, dy) in neighbors {
                let nx = (x as i32 + dx) as usize;
                let ny = (y as i32 + dy) as usize;
                let neighbor_val = iterations[ny * width + nx];

                high = high.max(neighbor_val);
                low = low.min(neighbor_val);

                let diff = neighbor_val - subject;

                // Apply direction based on light position (top-right)
                let h_component = if dx < 0 { -diff } else { diff };
                let v_component = if dy > 0 { -diff } else { diff };

                if dx != 0 { running_sum += h_component; }
                if dy != 0 { running_sum += v_component; }
            }

            let range = high - low;
            if range > 0.0 {
                shading[idx] = (running_sum * height_factor) / range;
            }
        }
    }

    shading
}
```

### Method 2: Analytic Derivative (Normal from Potential)

This method computes the normal vector from the gradient of the potential function using the derivative computed
during iteration.

**Mathematical basis** (from Cheritat):

The potential approximates as `2^(-n) * ln|z_n|` for large |z_n|. The normal direction is perpendicular to
equipotential lines and can be computed from `z/dz` where dz = dz_n/dc.

**Formula**:

```
u = z_n / dz_n  (complex normal direction)
normal = (Re(u), Im(u), 1) / sqrt(|u|² + 1)
```

**Lambert shading**:

```
light_dir = normalize(light_x, light_y, light_z)
shading = dot(normal, light_dir)
shading = max(0, shading)  // Clamp negative values
```

**Rust Implementation**:

```rust
fn compute_normal_and_shade(
    z: Complex64,
    dz: Complex64,
    light_angle: f64,
    light_elevation: f64,
) -> f64 {
    // Compute u = z / dz
    let u = z / dz;

    // Normal vector (x, y, 1) normalized
    let h2 = 1.5;  // Height factor
    let norm_factor = (u.norm_sqr() + h2 * h2).sqrt();
    let normal = (u.re / norm_factor, u.im / norm_factor, h2 / norm_factor);

    // Light direction from angle and elevation
    let light_dir = (
        light_angle.cos() * light_elevation.cos(),
        light_angle.sin() * light_elevation.cos(),
        light_elevation.sin(),
    );

    // Lambert shading: dot product
    let shade = normal.0 * light_dir.0
              + normal.1 * light_dir.1
              + normal.2 * light_dir.2;

    shade.max(0.0)  // Clamp to [0, 1]
}
```

### Kalles Fraktaler Parameters

Kalles Fraktaler provides these slope parameters:

- **Slope Magnification**: 100 for unzoomed view, 10000+ for deep zooms
- **Blend Percentage**: How much slope affects final color (0-100%)

At deep zooms, the slope needs significant magnification because the iteration gradients become very shallow
relative to pixel spacing.

### Pros

- Creates dramatic 3D visual effects
- Emphasizes fractal structure and boundaries
- Works well with any base coloring algorithm
- Finite difference method is easy to implement

### Cons

- Finite difference method requires post-processing
- Analytic method requires computing second derivative for best results
- Parameters (height factor, light direction) need tuning per zoom level
- Can introduce noise at very deep zooms

### Requirements for Implementation

- For finite difference: neighbor access, post-processing pass
- For analytic: derivative tracking during iteration
- Light direction parameters (angle, elevation)
- Height/magnification factor (scales with zoom)

---

## 6. Implementation Considerations

### Precision at Deep Zoom

At extreme zoom levels (10^100 and beyond), standard f64 precision is insufficient:

- **Smooth iteration count**: The log functions may need extended precision
- **Distance estimation**: The derivative can grow/shrink exponentially
- **Histogram coloring**: Works at any precision (integer iteration counts)
- **Slope shading**: Finite difference works if smooth iteration is available

### GPU vs CPU

| Algorithm | GPU Suitability | Notes |
|-----------|-----------------|-------|
| Smooth iteration | Excellent | Few extra operations per pixel |
| Distance estimation | Good | Extra complex multiply per iteration |
| Histogram coloring | Poor (pass 1), Good (pass 2) | Atomic operations needed for histogram |
| Exponential smoothing | Good | exp() is fast on GPU |
| Slope shading (FD) | Excellent | Pure post-processing |
| Slope shading (analytic) | Good | Extra complex operations |

### Memory Requirements

| Algorithm | Per-Pixel Storage | Additional Storage |
|-----------|-------------------|-------------------|
| Smooth iteration | f64 (8 bytes) | None |
| Distance estimation | f64 (8 bytes) | Complex derivative during iteration |
| Histogram coloring | f64 (8 bytes) | Histogram array (max_iter × 8 bytes) |
| Exponential smoothing | f64 (8 bytes) | Previous z during iteration |
| Slope shading (FD) | f64 (8 bytes) | Output shading array |

### Combining Algorithms

Recommended combinations:

1. **Best quality**: Smooth iteration + Histogram + Slope shading
2. **Deep zoom**: Smooth iteration + Distance estimation + Analytic slope
3. **Universal**: Exponential smoothing + Histogram
4. **Fast preview**: Smooth iteration only

---

## 7. Algorithm Comparison

| Algorithm | Anti-banding | Boundary Detail | 3D Effect | Universal | Cost |
|-----------|--------------|-----------------|-----------|-----------|------|
| Smooth iteration | Excellent | None | No | Divergent only | Low |
| Distance estimation | Good | Excellent | No | Divergent only | Medium |
| Histogram coloring | Excellent | Good | No | Yes | Medium |
| Exponential smoothing | Good | Good | No | Yes | Medium |
| Slope shading | None | Excellent | Yes | Yes | Low-Medium |

### Recommendation for Fractal Wonder

For a renderer targeting deep zooms (10^2000), implement in this order:

1. **Essential**: Smooth iteration count - eliminates banding, low overhead
2. **High impact**: Histogram coloring - adapts to any zoom level
3. **Visual polish**: Slope shading (finite difference) - adds depth
4. **Advanced**: Distance estimation - for rendering thin filaments
5. **Alternative**: Exponential smoothing - for experimentation

---

## 8. Implementation Roadmap

This section defines self-contained, shippable increments that build progressively toward visually stunning fractal
renders. Each increment is complete—no "this will be fixed later" dependencies. The goal is **more interesting things
to look at** as quickly as possible.

### 8.1 Codebase Architecture

This section documents how the current colorizer system works and where new code should live.

#### 8.1.1 Directory Structure

```
fractalwonder-core/src/
├── compute_data.rs          # ComputeData, MandelbrotData structs
└── lib.rs                   # Re-exports MandelbrotData

fractalwonder-ui/src/rendering/
├── colorizers/
│   ├── mod.rs               # Colorizer type alias, dispatch function
│   ├── mandelbrot.rs        # Mandelbrot colorization logic
│   └── test_image.rs        # Test pattern colorization
├── parallel_renderer.rs     # Uses colorize() for tile rendering
└── ...
```

#### 8.1.2 The Colorizer Abstraction

The colorizer is a simple function that converts compute data to RGBA pixels:

```rust
// fractalwonder-ui/src/rendering/colorizers/mod.rs

/// Colorizer function type - converts compute data to RGBA pixels.
/// The bool parameter is xray_enabled.
pub type Colorizer = fn(&ComputeData, bool) -> [u8; 4];

/// Dispatch colorization based on ComputeData variant.
pub fn colorize(data: &ComputeData, xray_enabled: bool) -> [u8; 4] {
    match data {
        ComputeData::TestImage(d) => colorize_test_image(d),
        ComputeData::Mandelbrot(d) => colorize_mandelbrot(d, xray_enabled),
    }
}
```

The dispatch pattern separates concerns:
- `ComputeData` is an enum with variants for each fractal type
- Each variant has its own colorizer function
- The `colorize()` function dispatches to the appropriate colorizer

#### 8.1.3 ComputeData and MandelbrotData

The compute layer produces `ComputeData` for each pixel:

```rust
// fractalwonder-core/src/compute_data.rs

pub struct MandelbrotData {
    pub iterations: u32,       // Iteration count at escape
    pub max_iterations: u32,   // Maximum iterations used
    pub escaped: bool,         // Whether point escaped
    pub glitched: bool,        // Whether reference orbit was glitched
}

pub enum ComputeData {
    TestImage(TestImageData),
    Mandelbrot(MandelbrotData),
}
```

**Key Insight**: To add smooth iteration or distance estimation, we extend `MandelbrotData` with new fields.
The colorizer can then use these fields without changing the dispatch pattern.

#### 8.1.4 How the Renderer Uses Colorizers

The `ParallelRenderer` calls `colorize()` for each pixel in a tile callback:

```rust
// fractalwonder-ui/src/rendering/parallel_renderer.rs

use crate::rendering::colorizers::colorize;

// In tile completion callback:
let xray = xray_enabled.get();
let pixels: Vec<u8> = result.data
    .iter()
    .flat_map(|d| colorize(d, xray))
    .collect();

// Draw pixels to canvas...
```

The renderer also stores `tile_results` for re-colorizing without recompute (used by xray toggle).
This pattern extends naturally to palette changes and other colorization settings.

#### 8.1.5 Current Colorizer Implementation

The current Mandelbrot colorizer is minimal (`mandelbrot.rs`):

```rust
pub fn colorize(data: &MandelbrotData, xray_enabled: bool) -> [u8; 4] {
    // Glitched pixels → cyan (when xray enabled)
    if xray_enabled && data.glitched {
        let brightness = (64.0 + normalized * 191.0) as u8;
        return [0, brightness, brightness, 255];
    }

    // Interior → black
    if !data.escaped {
        return [0, 0, 0, 255];
    }

    // Exterior → linear grayscale
    let normalized = data.iterations as f64 / data.max_iterations as f64;
    let gray = (normalized * 255.0) as u8;
    [gray, gray, gray, 255]
}
```

**Limitations**:
- Linear grayscale mapping: `gray = (iterations / max_iterations) * 255`
- No smooth iteration (visible banding)
- No color palettes
- No post-processing effects

#### 8.1.6 Where New Code Will Live

| Feature | Location | Changes |
|---------|----------|---------|
| **Palettes** | `colorizers/palette.rs` (new) | New module, imported by `mandelbrot.rs` |
| **Smooth iteration** | `fractalwonder-core/src/compute_data.rs` | Add `final_z_norm_sq: f64` to `MandelbrotData` |
| **Smooth colorizer** | `colorizers/mandelbrot.rs` | Update `colorize()` to use smooth formula |
| **Slope shading** | `colorizers/shading.rs` (new) | Post-processing module for slope shading |
| **Histogram eq** | `colorizers/histogram.rs` (new) | Two-pass equalization, integrates with renderer |
| **Distance estimation** | `fractalwonder-core/...` | Compute-side changes, `distance_estimate: Option<f64>` |

**Integration Pattern**:

New colorizer features follow this pattern:
1. Extend `MandelbrotData` if compute data is needed (smooth iteration, distance)
2. Add new module in `colorizers/` for the feature logic
3. Update `colorize_mandelbrot()` to use the new data/module
4. Re-export from `colorizers/mod.rs` if needed elsewhere

### 8.2 Visual Impact vs Implementation Effort

| Algorithm | Visual Impact | Implementation Effort | Dependencies |
|-----------|---------------|----------------------|--------------|
| Color palettes | High | Low | None |
| Smooth iteration count | High | Medium | Compute changes (store final z) |
| Slope shading (FD) | Very High | Low | Smooth iteration data |
| Histogram equalization | Medium | Medium | Two-pass rendering |
| Distance estimation | High | High | Derivative tracking in compute |

**Strategy**: Maximize visual impact early. Color palettes and slope shading provide dramatic improvement with
minimal compute changes.

---

### 8.3 Increment 1: Color Palettes

**Deliverable**: Rich, customizable color palettes replacing grayscale.

**Why First**: Biggest visual improvement with zero compute changes. Works with existing `MandelbrotData`.

**Files**:
- `colorizers/palette.rs` (new) - Palette struct and sampling logic
- `colorizers/mandelbrot.rs` - Import and use palette in `colorize()`
- `colorizers/mod.rs` - Re-export `Palette` for UI access

**Implementation**:

```rust
/// A color palette maps normalized iteration values [0.0, 1.0] to RGB colors.
pub struct Palette {
    colors: Vec<[u8; 3]>,  // Control points
}

impl Palette {
    /// Classic Ultra Fractal-style palette
    pub fn ultra_fractal() -> Self {
        Self {
            colors: vec![
                [0, 7, 100],      // Deep blue
                [32, 107, 203],   // Blue
                [237, 255, 255],  // White
                [255, 170, 0],    // Orange
                [0, 2, 0],        // Near black
            ],
        }
    }

    /// Sample the palette at position t ∈ [0, 1]
    pub fn sample(&self, t: f64) -> [u8; 3] {
        let t = t.clamp(0.0, 1.0);
        let scaled = t * (self.colors.len() - 1) as f64;
        let i = scaled.floor() as usize;
        let frac = scaled.fract();

        if i >= self.colors.len() - 1 {
            return self.colors[self.colors.len() - 1];
        }

        // Linear interpolation between adjacent colors
        let c1 = self.colors[i];
        let c2 = self.colors[i + 1];
        [
            (c1[0] as f64 + frac * (c2[0] as f64 - c1[0] as f64)) as u8,
            (c1[1] as f64 + frac * (c2[1] as f64 - c1[1] as f64)) as u8,
            (c1[2] as f64 + frac * (c2[2] as f64 - c1[2] as f64)) as u8,
        ]
    }
}
```

**Predefined Palettes**:
1. **Ultra Fractal classic**: Blue → white → orange → black (the "Wikipedia" look)
2. **Fire**: Black → red → orange → yellow → white
3. **Ocean**: Deep blue → cyan → white
4. **Monochrome**: Black → white (current, for comparison)
5. **Psychedelic**: Full HSV hue cycle

**Palette Cycling**: Apply `(t * cycle_count) % 1.0` before sampling for more detail at high iteration counts.

**Test Strategy**:
1. Visual regression: render known coordinates, compare against reference images
2. Interpolation: verify smooth gradients between control points
3. Edge cases: t=0, t=1, t slightly outside [0,1]

**Acceptance Criteria**:
- At least 5 palettes selectable via UI
- Palette cycling parameter (1-100 cycles)
- No visible banding within a single palette segment
- Render time unchanged (palette lookup is O(1))

---

### 8.4 Increment 2: Smooth Iteration Count

**Deliverable**: Eliminate iteration banding with continuous coloring.

**Why Second**: Foundational for all advanced techniques. Banding is the most obvious visual flaw.

**Files**:
- `fractalwonder-core/src/compute_data.rs` - Add `final_z_norm_sq` field to `MandelbrotData`
- `colorizers/mandelbrot.rs` - Update `colorize()` to compute smooth iteration
- Compute modules (CPU/GPU) - Store `|z_n|²` at escape

**Compute Changes Required** (`fractalwonder-core/src/compute_data.rs`):

```rust
// Current MandelbrotData
pub struct MandelbrotData {
    pub iterations: u32,
    pub max_iterations: u32,
    pub escaped: bool,
    pub glitched: bool,
}

// Extended MandelbrotData
pub struct MandelbrotData {
    pub iterations: u32,
    pub max_iterations: u32,
    pub escaped: bool,
    pub glitched: bool,
    pub final_z_norm_sq: f64,  // NEW: |z_n|² at escape (for smooth coloring)
}
```

**Colorizer Changes** (`colorizers/mandelbrot.rs`):

The current colorizer signature is `fn colorize(data: &MandelbrotData, xray_enabled: bool) -> [u8; 4]`.
Palette becomes an additional parameter or a thread-local/static reference:

```rust
// colorizers/mandelbrot.rs
use super::palette::Palette;

pub fn colorize(data: &MandelbrotData, xray_enabled: bool, palette: &Palette) -> [u8; 4] {
    // Glitched handling (unchanged)
    if xray_enabled && data.glitched {
        let normalized = data.iterations as f64 / data.max_iterations as f64;
        let brightness = (64.0 + normalized * 191.0) as u8;
        return [0, brightness, brightness, 255];
    }

    if !data.escaped {
        return [0, 0, 0, 255];  // Interior is black
    }

    // Smooth iteration count: μ = n + 1 - log₂(log₂|z_n|²)/2
    let smooth = if data.final_z_norm_sq > 1.0 {
        let log_zn = data.final_z_norm_sq.ln() / 2.0;  // ln(|z|)
        let nu = log_zn.ln() / 2.0_f64.ln();           // log₂(ln(|z|))
        data.iterations as f64 + 1.0 - nu
    } else {
        data.iterations as f64
    };

    let normalized = (smooth / data.max_iterations as f64).clamp(0.0, 1.0);
    let [r, g, b] = palette.sample(normalized);
    [r, g, b, 255]
}
```

**Signature Change Propagation**:

Adding `palette` to `colorize()` requires updates to:
- `colorizers/mod.rs` - Update dispatch function signature
- `parallel_renderer.rs` - Pass palette to `colorize()` calls

**Mathematical Foundation**:

The smooth iteration count `μ` satisfies:
- `μ` is continuous as a function of `c`
- `μ` approaches `n` as `|z_n|` approaches the escape radius
- `μ` approaches `n+1` as `|z_n|` approaches infinity

This requires escape radius > 2 (recommend 256 for best smoothing).

**Test Strategy**:
1. **Continuity**: Adjacent pixels should have `|μ₁ - μ₂| < 1` (no jumps > 1 iteration)
2. **Consistency**: Same pixel computed twice yields identical `μ`
3. **Boundary**: At exact escape radius, `μ ≈ n`
4. **Deep zoom**: Verify formula works with extended precision types

**Acceptance Criteria**:
- No visible banding in exterior regions
- `final_z_norm_sq` correctly stored in compute data
- GPU compute shader updated to output `final_z_norm_sq`
- Performance: <5% overhead vs integer iteration count

---

### 8.5 Increment 3: Slope Shading (Finite Difference)

**Deliverable**: 3D lighting effect using iteration count as height field.

**Why Third**: Very high visual impact. Pure post-processing—no compute changes beyond Increment 2.

**Files**:
- `colorizers/shading.rs` (new) - Slope shading computation
- `colorizers/mod.rs` - Re-export shading functions
- `parallel_renderer.rs` - Integrate shading pass after colorization

**Implementation** (`colorizers/shading.rs`):

Slope shading treats smooth iteration count as a height map and computes apparent illumination.

```rust
/// Apply slope shading to an iteration buffer.
/// Returns a shading multiplier per pixel in range [0.0, 1.0].
pub fn compute_slope_shading(
    iterations: &[f64],      // Smooth iteration counts
    width: usize,
    height: usize,
    light_angle: f64,        // Radians, 0 = right, π/2 = top
    height_factor: f64,      // Typically 1.0-20.0
) -> Vec<f64> {
    let mut shading = vec![0.5; width * height];  // Neutral default

    let light_x = light_angle.cos();
    let light_y = light_angle.sin();

    for y in 1..height-1 {
        for x in 1..width-1 {
            let idx = y * width + x;
            let center = iterations[idx];

            // Skip interior points
            if center.is_nan() || center.is_infinite() {
                continue;
            }

            // Sobel-like gradient estimation
            let left = iterations[idx - 1];
            let right = iterations[idx + 1];
            let up = iterations[(y - 1) * width + x];
            let down = iterations[(y + 1) * width + x];

            // Gradient (pointing "uphill" in iteration space)
            let dx = (right - left) * height_factor;
            let dy = (down - up) * height_factor;

            // Dot product with light direction
            let dot = dx * light_x + dy * light_y;

            // Normalize to [0, 1] range
            let shade = (dot / (1.0 + dx.abs() + dy.abs()) + 1.0) / 2.0;
            shading[idx] = shade.clamp(0.0, 1.0);
        }
    }

    shading
}

/// Apply shading to final color
pub fn apply_shading(base_color: [u8; 3], shade: f64) -> [u8; 3] {
    // Blend between darker and lighter versions
    let factor = 0.3 + shade * 1.4;  // Range [0.3, 1.7]
    [
        (base_color[0] as f64 * factor).clamp(0.0, 255.0) as u8,
        (base_color[1] as f64 * factor).clamp(0.0, 255.0) as u8,
        (base_color[2] as f64 * factor).clamp(0.0, 255.0) as u8,
    ]
}
```

**Parameters**:
- `light_angle`: Direction of light source (default: π/4, top-right)
- `height_factor`: Exaggeration of slopes (auto-scale with zoom: deeper zoom → higher factor)
- `shading_strength`: Blend with unshaded color (0.0 = off, 1.0 = full effect)

**Auto-Scaling for Deep Zoom**:

At deep zoom, iteration gradients become very small. The height factor should scale with zoom:

```rust
let height_factor = base_height_factor * (1.0 + zoom_level.log10() / 10.0);
```

**Test Strategy**:
1. **Flat region**: Uniform iterations → shade = 0.5 (neutral)
2. **Slope facing light**: Higher shade value
3. **Slope facing away**: Lower shade value
4. **Edge pixels**: No crash, reasonable values
5. **Performance**: Shading pass < 10ms for 4K image

**Acceptance Criteria**:
- Visible 3D relief effect on fractal boundary
- Light direction adjustable via UI
- Height factor auto-scales or is user-adjustable
- Shading can be toggled on/off
- No artifacts at image borders

---

### 8.6 Increment 4: Histogram Equalization

**Deliverable**: Automatic color distribution adapting to any zoom level.

**Why Fourth**: Ensures details visible at all scales. Requires buffering all iteration counts before coloring.

**Files**:
- `colorizers/histogram.rs` (new) - `HistogramEqualizer` struct and CDF logic
- `colorizers/mod.rs` - Re-export for renderer access
- `parallel_renderer.rs` - Major changes: buffer tiles, build histogram, then colorize

**Architecture Impact**:

This increment requires the most significant change to the rendering pipeline. Currently,
`ParallelRenderer` colorizes each tile immediately on completion. Histogram equalization
requires:
1. All tiles complete first (iteration data buffered)
2. Build histogram from all iteration values
3. Then colorize using the equalized CDF

The existing `tile_results` buffer in `ParallelRenderer` (used for xray re-colorizing)
provides the foundation for this pattern.

**Implementation** (`colorizers/histogram.rs`):

```rust
pub struct HistogramEqualizer {
    histogram: Vec<u64>,
    cdf: Vec<f64>,
    bucket_count: usize,
}

impl HistogramEqualizer {
    pub fn new(bucket_count: usize) -> Self {
        Self {
            histogram: vec![0; bucket_count],
            cdf: vec![0.0; bucket_count],
            bucket_count,
        }
    }

    /// Build histogram from smooth iteration values.
    /// Returns the min and max iteration values encountered.
    pub fn build(&mut self, iterations: &[f64], max_iter: f64) -> (f64, f64) {
        self.histogram.fill(0);

        let mut min_iter = f64::MAX;
        let mut max_iter_seen = 0.0_f64;
        let mut exterior_count = 0u64;

        for &iter in iterations {
            if iter.is_nan() || iter >= max_iter {
                continue;  // Skip interior points
            }
            min_iter = min_iter.min(iter);
            max_iter_seen = max_iter_seen.max(iter);
            exterior_count += 1;

            let bucket = ((iter / max_iter) * self.bucket_count as f64)
                .floor() as usize;
            let bucket = bucket.min(self.bucket_count - 1);
            self.histogram[bucket] += 1;
        }

        // Build CDF
        let mut cumulative = 0u64;
        for i in 0..self.bucket_count {
            cumulative += self.histogram[i];
            self.cdf[i] = if exterior_count > 0 {
                cumulative as f64 / exterior_count as f64
            } else {
                i as f64 / self.bucket_count as f64
            };
        }

        (min_iter, max_iter_seen)
    }

    /// Map an iteration value through the equalized CDF.
    pub fn equalize(&self, iter: f64, max_iter: f64) -> f64 {
        if iter.is_nan() || iter >= max_iter {
            return 1.0;  // Interior
        }

        let bucket = ((iter / max_iter) * self.bucket_count as f64)
            .floor() as usize;
        let bucket = bucket.min(self.bucket_count - 1);

        self.cdf[bucket]
    }
}
```

**Integration with Smooth Iteration**:

```rust
// In colorization pipeline:
let smooth_iter = compute_smooth_iteration(data);
let equalized = histogram.equalize(smooth_iter, max_iter);
let color = palette.sample(equalized);
```

**Rendering Pipeline Change**:

```
Before: pixel → compute → colorize → display
After:  pixel → compute → buffer → histogram → colorize → display
                              ↑
                    (all pixels must complete before coloring)
```

This requires a two-pass approach or buffering all iteration data before colorization.

**Test Strategy**:
1. **Uniform distribution**: After equalization, all palette colors used roughly equally
2. **Sparse iterations**: Few unique iteration counts → still reasonable distribution
3. **Single iteration**: All pixels same iteration → no division by zero
4. **Performance**: Histogram build < 5ms for 4K image

**Acceptance Criteria**:
- Color distribution adapts to local iteration range
- Details visible at any zoom level
- Interior points unaffected by equalization
- Can be toggled on/off
- Histogram bucket count configurable (default: 1024)

---

### 8.7 Increment 5: Distance Estimation

**Deliverable**: Render thin filaments and boundary details smaller than pixels.

**Why Fifth**: High implementation cost (requires derivative tracking), but essential for deep zoom quality.

**Files**:
- `fractalwonder-core/src/compute_data.rs` - Add `distance_estimate: Option<f64>` to `MandelbrotData`
- `fractalwonder-core/src/mandelbrot/...` - Track derivative `dz/dc` during iteration
- `colorizers/mandelbrot.rs` - Use distance for boundary darkening

**Architecture Impact**:

Distance estimation requires tracking the derivative `dz/dc` during iteration, which means
changes to the core compute loop. For perturbation theory rendering, this extends to
computing the derivative in delta space. This is the most invasive change to the compute
layer.

**Compute Changes Required** (`fractalwonder-core/src/compute_data.rs`):

```rust
pub struct MandelbrotData {
    pub iterations: u32,
    pub max_iterations: u32,
    pub escaped: bool,
    pub glitched: bool,
    pub final_z_norm_sq: f64,
    pub distance_estimate: Option<f64>,  // NEW: estimated distance to M
}
```

**Distance Estimation During Iteration**:

```rust
fn iterate_with_distance(c: Complex64, max_iter: u32) -> MandelbrotData {
    let mut z = Complex64::zero();
    let mut dz = Complex64::zero();  // Derivative dz/dc

    for n in 0..max_iter {
        // Update derivative FIRST (uses current z)
        dz = 2.0 * z * dz + Complex64::new(1.0, 0.0);

        // Then update z
        z = z * z + c;

        if z.norm_sqr() > ESCAPE_RADIUS_SQ {
            // Distance estimate formula
            let z_norm = z.norm();
            let dz_norm = dz.norm();
            let distance = if dz_norm > 0.0 {
                Some(z_norm * z_norm.ln() * 2.0 / dz_norm)
            } else {
                None
            };

            return MandelbrotData {
                iterations: n,
                max_iterations: max_iter,
                escaped: true,
                glitched: false,
                final_z_norm_sq: z.norm_sqr(),
                distance_estimate: distance,
            };
        }
    }

    MandelbrotData {
        iterations: max_iter,
        max_iterations: max_iter,
        escaped: false,
        glitched: false,
        final_z_norm_sq: z.norm_sqr(),
        distance_estimate: None,
    }
}
```

**Using Distance for Coloring**:

```rust
pub fn colorize_with_distance(
    data: &MandelbrotData,
    palette: &Palette,
    pixel_size: f64,  // Size of one pixel in complex plane
) -> [u8; 4] {
    if !data.escaped {
        return [0, 0, 0, 255];
    }

    // Base color from smooth iteration
    let base_color = colorize_smooth(data, palette);

    // Darken based on distance to boundary
    if let Some(distance) = data.distance_estimate {
        let relative_dist = distance / pixel_size;
        if relative_dist < 1.0 {
            // Very close to boundary - darken significantly
            let factor = relative_dist.sqrt();
            return [
                (base_color[0] as f64 * factor) as u8,
                (base_color[1] as f64 * factor) as u8,
                (base_color[2] as f64 * factor) as u8,
                255,
            ];
        }
    }

    base_color
}
```

**Test Strategy**:
1. **Derivative correctness**: Compare `dz` against numerical differentiation
2. **Distance bounds**: Verify `distance ≤ actual_distance` (DEM is a lower bound)
3. **Filament visibility**: Thin structures visible at sub-pixel scale
4. **Performance**: <10% overhead from derivative tracking

**Acceptance Criteria**:
- Thin filaments rendered with visible detail
- Distance-based darkening at boundary
- Works with perturbation (derivative in delta space)
- Optional—can be disabled for performance

---

### 8.8 Summary

| Increment | Visual Impact | Primary Files | Compute Changes |
|-----------|---------------|---------------|-----------------|
| 1. Color Palettes | High | `colorizers/palette.rs` (new) | None |
| 2. Smooth Iteration | High | `compute_data.rs`, `colorizers/mandelbrot.rs` | Add `final_z_norm_sq` |
| 3. Slope Shading | Very High | `colorizers/shading.rs` (new) | None |
| 4. Histogram Eq. | Medium | `colorizers/histogram.rs` (new), `parallel_renderer.rs` | Buffer tiles |
| 5. Distance Est. | High | `compute_data.rs`, compute modules | Track derivative |

**File Change Summary**:

| File | Increments | Type of Change |
|------|------------|----------------|
| `colorizers/palette.rs` | 1 | New module |
| `colorizers/mandelbrot.rs` | 1, 2, 5 | Add palette, smooth iteration, distance |
| `colorizers/shading.rs` | 3 | New module |
| `colorizers/histogram.rs` | 4 | New module |
| `colorizers/mod.rs` | 1, 3, 4 | Re-exports |
| `fractalwonder-core/src/compute_data.rs` | 2, 5 | Extend `MandelbrotData` |
| `parallel_renderer.rs` | 1, 4 | Pass palette, buffer for histogram |

**Dependency Graph**:

```
                    ┌─────────────────────────┐
                    │ Increment 1:            │
                    │ Color Palettes          │
                    └─────────────────────────┘
                              │
                              ▼
                    ┌─────────────────────────┐
                    │ Increment 2:            │
                    │ Smooth Iteration        │
                    └─────────────────────────┘
                              │
              ┌───────────────┴───────────────┐
              ▼                               ▼
    ┌─────────────────────────┐     ┌─────────────────────────┐
    │ Increment 3:            │     │ Increment 4:            │
    │ Slope Shading           │     │ Histogram Equalization  │
    └─────────────────────────┘     └─────────────────────────┘

                    ┌─────────────────────────┐
                    │ Increment 5:            │
                    │ Distance Estimation     │
                    │ (independent)           │
                    └─────────────────────────┘
```

**Recommended Implementation Order for "More Interesting Things to Look At"**:

1. **Increment 1** (Color Palettes): Immediate visual improvement, no risk
2. **Increment 2** (Smooth Iteration): Foundation for everything else
3. **Increment 3** (Slope Shading): Dramatic 3D effect, pure post-processing
4. **Increment 4** (Histogram): Adaptive coloring for exploration
5. **Increment 5** (Distance Estimation): Advanced, for serious deep zoom work

After Increments 1-3, the renderer will produce visually stunning images comparable to Kalles Fraktaler's output.

---

## 9. References

### Primary Sources

- [Smooth Iteration Count - Inigo Quilez](https://iquilezles.org/articles/msetsmooth/)
- [Smooth Shading for Mandelbrot Exterior - Linas Vepstas](https://linas.org/art-gallery/escape/smooth.html)
- [Smooth Iteration Count Derivation - Ruben van Nieuwpoort](https://rubenvannieuwpoort.nl/posts/smooth-iteration-count-for-the-mandelbrot-set)
- [Distance Estimator - MROB Encyclopaedia](http://www.mrob.com/pub/muency/distanceestimator.html)
- [Mandelbrot Techniques - Arnaud Cheritat](https://www.math.univ-toulouse.fr/~cheritat/wiki-draw/index.php/Mandelbrot_set)
- [Slope Shading Implementation - Phil Thompson](https://philthompson.me/2022/Smooth-Colors-and-Slope-Shading-for-the-Mandelbrot-set.html)

### Software References

- [Kalles Fraktaler 2+ Manual](https://mathr.co.uk/kf/manual.html)
- [Ultra Fractal Coloring Algorithms](https://www.ultrafractal.com/help/coloring/standard/exponentialsmoothing.html)
- [Fractals Wikibook - Color Methods](https://en.wikibooks.org/wiki/Fractals/color_mandelbrot)

### Academic References

- Peitgen, H.-O., & Richter, P. H. (1986). *The Beauty of Fractals*
- Peitgen, H.-O., & Saupe, D. (1988). *The Science of Fractal Images*
- Milnor, J. (2006). *Dynamics in One Complex Variable*, Appendix G

### Additional Resources

- [Plotting Algorithms for the Mandelbrot Set - Wikipedia](https://en.wikipedia.org/wiki/Plotting_algorithms_for_the_Mandelbrot_set)
- [Orbit Traps - Inigo Quilez](https://iquilezles.org/articles/ftrapsgeometric/)
- [Buddhabrot Technique - Melinda Green](https://superliminal.com/fractals/bbrot/bbrot.htm)
- [Computing Distance to Julia Set - Inigo Quilez](https://iquilezles.org/articles/distancefractals/)
- [Normal Computation for SDFs - Inigo Quilez](https://iquilezles.org/articles/normalsSDF/)
