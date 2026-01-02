# BLA and Derivative Tracking for Deep Zoom Mandelbrot Rendering

This paper explains how Bivariate Linear Approximation (BLA) accelerates Mandelbrot set rendering,
why derivatives matter for 3D lighting effects, and how to extend BLA to track derivatives correctly.

## Table of Contents

1. [Introduction](#1-introduction)
2. [Perturbation Theory Fundamentals](#2-perturbation-theory-fundamentals)
3. [Bivariate Linear Approximation (BLA)](#3-bivariate-linear-approximation-bla)
4. [Derivatives for Distance Estimation and 3D Lighting](#4-derivatives-for-distance-estimation-and-3d-lighting)
5. [Our Current Implementation](#5-our-current-implementation)
6. [The Problem: BLA Does Not Track Derivatives](#6-the-problem-bla-does-not-track-derivatives)
7. [Research: How Production Renderers Handle This](#7-research-how-production-renderers-handle-this)
8. [Mathematical Derivation: Extending BLA with Derivative Coefficients](#8-mathematical-derivation-extending-bla-with-derivative-coefficients)
9. [Implementation Options](#9-implementation-options)
10. [Conclusion](#10-conclusion)
11. [References](#11-references)

---

## 1. Introduction

Rendering the Mandelbrot set at extreme zoom depths ($10^{100}$ and beyond) requires specialized algorithms.
Standard floating-point arithmetic cannot represent the tiny coordinate differences between adjacent
pixels at such depths. Arbitrary-precision arithmetic solves the precision problem but runs too slowly
when millions of iterations per pixel are required.

Two techniques address these challenges:

1. **Perturbation theory** computes one high-precision "reference orbit" and derives all other pixels
   from it using fast low-precision arithmetic.

2. **BLA (Bivariate Linear Approximation)** skips many iterations at once by approximating the
   iteration formula as a linear function.

Together, these techniques reduce render times from hours to seconds.

However, 3D lighting effects require **derivatives**—the rate of change of the orbit with respect to
the pixel coordinate. Our current implementation tracks derivatives during standard iterations but
**fails to update them during BLA skips**. This causes visible artifacts in 3D-shaded renders.

This paper derives the mathematics needed to extend BLA with derivative tracking.

---

## 2. Perturbation Theory Fundamentals

### 2.1 The Problem with Deep Zoom

The Mandelbrot set iteration formula is:

```math
z_{n+1} = z_n² + c
```

Starting from $z_0 = 0$, we iterate until $|z|$ exceeds an escape radius (typically 256) or we reach a
maximum iteration count. The number of iterations before escape determines the pixel color.

At zoom depth $10^{300}$, adjacent pixels differ by approximately $10^{-300}$ in their $c$ values. Standard
64-bit floating-point numbers can only represent differences as small as $\sim 10^{-308}$, and they lose
precision long before that limit. Arbitrary-precision arithmetic can represent any precision, but
iterating millions of times per pixel with 1000+ bit numbers takes far too long.

### 2.2 The Key Insight

The iteration formula $z \to z^2 + c$ is **continuous**: nearby points remain nearby under iteration.
If two pixels start with similar $c$ values, their orbits stay similar for many iterations.

This insight enables a crucial optimization:

1. Compute one "reference orbit" at full precision for a point $C$ at the image center
2. For each pixel with value $c = C + \delta c$, compute only the **difference** from the reference orbit

The difference (called the "delta" or "perturbation") stays small, so low-precision arithmetic suffices.

### 2.3 The Delta Iteration Formula

Let $Z_n$ denote the reference orbit (computed at high precision) and $\delta z_n$ denote the perturbation
for a particular pixel. The full orbit value is $z_n = Z_n + \delta z_n$.

Substituting into the iteration formula:

```math
z_{n+1} = z_n^2 + c\\
(Z_{n+1} + \delta z_{n+1}) = (Z_n + \delta z_n)^2 + (C + \delta c)
```

Expanding the square:

```math
Z_{n+1} + \delta z_{n+1} = Z_n^2 + 2 \cdot Z_n \cdot \delta z_n + \delta z_n^2 + C + \delta c
```

Since $Z_{n+1} = Z_n^2 + C$ (the reference orbit satisfies the iteration formula), we subtract it:

```math
\delta z_{n+1} = 2 \cdot Z_n \cdot \delta z_n + \delta z_n^2 + \delta c
```

This is the **delta iteration formula**. It computes the next perturbation from:
- The current perturbation $\delta z_n$
- The reference orbit value $Z_n$ (precomputed)
- The pixel's offset $\delta c$ (constant for each pixel)

The quadratic term $\delta z_n^2$ is typically negligible when $|\delta z|$ is small.

### 2.4 Rebasing

Sometimes the perturbation $\delta z$ grows large enough that precision degrades. When $|Z_n + \delta z_n| < |\delta z_n|$
(the full value is smaller than the perturbation), we **rebase**:

1. Set $\delta z = Z_n + \delta z_n$ (absorb the reference into the delta)
2. Reset the reference orbit index to 0
3. Continue iterating

Rebasing keeps perturbations small without requiring additional reference orbits.

---

## 3. Bivariate Linear Approximation (BLA)

### 3.1 Motivation

The delta iteration formula requires one multiplication and one addition per iteration (ignoring
the small $\delta z^2$ term). At zoom depth $10^{300}$, pixels may require millions of iterations. Even with
fast perturbation arithmetic, this takes significant time.

BLA exploits the observation that when $|\delta z|$ is very small, the iteration behaves **linearly**.
Instead of iterating one step at a time, BLA skips many iterations at once.

### 3.2 The Linear Approximation

When $|\delta z|$ is small, the quadratic term $\delta z^2$ becomes negligible:

```math
\delta z_{n+1} \approx 2 \cdot Z_n \cdot \delta z_n + \delta c
```

This is a **linear** function of $\delta z$ and $\delta c$. More generally, after $L$ iterations starting from
iteration $m$, the perturbation can be approximated as:

```math
\delta z_{m+L} \approx A \cdot \delta z_m + B \cdot \delta c
```

Where $A$ and $B$ are complex coefficients that depend on the reference orbit values $Z_m$ through $Z_{m+L-1}$.

### 3.3 Single-Step Coefficients

For a single iteration ($L = 1$), comparing the linear approximation to the delta formula:

```math
\delta z_{m+1} = 2 \cdot Z_m \cdot \delta z_m + \delta c\\
\delta z_{m+1} \approx A \cdot \delta z_m + B \cdot \delta c
```

We read off:

```math
A = 2 \cdot Z_m\\
B = 1
```

### 3.4 Merging BLA Entries

The power of BLA comes from **composing** consecutive approximations. Given:

- $\text{BLA}_x$: skips $L_x$ iterations with coefficients $(A_x, B_x)$
- $\text{BLA}_y$: skips $L_y$ iterations with coefficients $(A_y, B_y)$

Applying $\text{BLA}_x$ then $\text{BLA}_y$:

```math
\text{After BLA}_x: \delta z' = A_x \cdot \delta z + B_x \cdot \delta c\\
\text{After BLA}_y: \delta z'' = A_y \cdot \delta z' + B_y \cdot \delta c\\
                               = A_y \cdot (A_x \cdot \delta z + B_x \cdot \delta c) + B_y \cdot \delta c\\
                               = A_y \cdot A_x \cdot \delta z + (A_y \cdot B_x + B_y) \cdot \delta c\\
```

The merged BLA skips $L_x + L_y$ iterations with coefficients:

```math
A_{\text{merged}} = A_y \cdot A_x\\
B_{\text{merged}} = A_y \cdot B_x + B_y\\
L_{\text{merged}} = L_x + L_y
```

### 3.5 Validity Radius

The linear approximation holds only when $|\delta z|$ is small enough that the quadratic term is negligible.
Each BLA entry has a **validity radius** $r$. The BLA is applicable when $|\delta z|^2 < r^2$.

For a single-step BLA:

```math
r = \varepsilon \cdot |Z_m|
```

Where $\varepsilon$ is a precision threshold (typically $2^{-53}$ for double precision).

For merged BLAs, the validity radius shrinks:

```math
r_{\text{merged}} = \min(r_x, \max(0, (r_y - |B_x| \cdot |\delta c|_{\max}) / |A_x|))
```

### 3.6 BLA Table Construction

For a reference orbit of $M$ iterations, we build a **binary tree** of BLA entries:

1. **Level 0**: $M$ single-step BLAs, one for each orbit point
2. **Level 1**: $M/2$ two-step BLAs, merging consecutive pairs from level 0
3. **Level 2**: $M/4$ four-step BLAs, merging pairs from level 1
4. Continue until reaching a single entry spanning the entire orbit

This produces $O(2M)$ total entries. During rendering, we find the largest valid BLA at the current
orbit position and apply it, potentially skipping thousands of iterations at once.

### 3.7 Performance Impact

BLA typically skips 99%+ of iterations at deep zoom. Benchmarks show 10-40x speedups over
perturbation without BLA, and 100x+ speedups over series approximation methods.

---

## 4. Derivatives for Distance Estimation and 3D Lighting

### 4.1 Why Track Derivatives?

The derivative $\rho = dz/dc$ measures how the orbit changes as the pixel coordinate changes. This
derivative enables:

1. **Distance estimation**: Computing the distance from a point to the Mandelbrot set boundary
2. **3D lighting/slope shading**: Creating surface normals for pseudo-3D lighting effects
3. **Anti-aliasing**: Determining appropriate sampling density near the boundary

### 4.2 The Derivative Iteration Formula

For $z_{n+1} = z_n^2 + c$, the derivative with respect to $c$ is:

```math
\frac{d}{dc}[z_{n+1}] = \frac{d}{dc}[z_n^2 + c]\\
                      = 2 \cdot z_n \cdot \frac{dz_n}{dc} + 1
```

Defining $\rho_n = dz_n/dc$:

```math
\rho_0 = 0\\
\rho_{n+1} = 2 \cdot z_n \cdot \rho_n + 1
```

At each iteration, the derivative doubles (times the current $z$ value) and adds 1.

### 4.3 Derivatives with Perturbation Theory

With perturbation, we track the **delta derivative** $\delta\rho$ separately from the reference derivative
$\text{Der}_m$. The full derivative is $\rho = \text{Der}_m + \delta\rho$.

The delta derivative iteration formula:

```math
\delta\rho_{n+1} = 2 \cdot Z_m \cdot \delta\rho_n + 2 \cdot \delta z_n \cdot \text{Der}_m + 2 \cdot \delta z_n \cdot \delta\rho_n
```

This formula has three terms:
1. $2 \cdot Z_m \cdot \delta\rho_n$: The reference orbit scales the delta derivative
2. $2 \cdot \delta z_n \cdot \text{Der}_m$: The perturbation interacts with the reference derivative
3. $2 \cdot \delta z_n \cdot \delta\rho_n$: The perturbation interacts with the delta derivative (often negligible)

### 4.4 Surface Normals for 3D Lighting

At escape, the surface normal direction is computed from $z$ and $\rho$:

```math
u = z / \rho = z \cdot \overline{\rho} / |\rho|^2
```

Since only the direction matters (not the magnitude), we normalize to a unit vector:

```math
u_{\text{normalized}} = u / |u|
```

This 2D direction becomes a 3D surface normal by adding a height component:

```math
\text{normal}_{3D} = (\text{Re}(u), \text{Im}(u), 1) / |(\text{Re}(u), \text{Im}(u), 1)|
```

Blinn-Phong or similar lighting models then produce the final shading.

---

## 5. Our Current Implementation

### 5.1 Architecture Overview

Our renderer (Fractalwonder) supports both CPU and GPU rendering with:

- Perturbation theory for deep zoom
- HDRFloat (high dynamic range float) for extended exponent range
- BLA acceleration for iteration skipping
- Derivative tracking for 3D lighting
- Rebasing to avoid glitches

### 5.2 BLA Entry Structure

Each BLA entry contains (from `bla.rs`):

```rust
pub struct BlaEntry {
    pub a: HDRComplex,      // Coefficient A (multiplies δz)
    pub b: HDRComplex,      // Coefficient B (multiplies δc)
    pub l: u32,             // Number of iterations to skip
    pub r_sq: HDRFloat,     // Validity radius squared
}
```

### 5.3 BLA Application

When BLA is valid, we apply it (from `pixel_hdr_bla.rs`):

```rust
if let Some(bla) = bla_entry {
    // Apply BLA: δz_new = A·δz + B·δc
    let a_dz = bla.a.mul(&dz);
    let b_dc = bla.b.mul(&delta_c);
    dz = a_dz.add(&b_dc);

    m += bla.l as usize;
    n += bla.l;
}
```

### 5.4 Derivative Tracking (Standard Iteration)

During standard (non-BLA) iterations, we update the derivative delta:

```rust
// δρ' = 2·Z_m·δρ + 2·δz·Der_m + 2·δz·δρ
let two_z_drho = 2.0 * Z_m * drho;
let two_dz_der = 2.0 * dz * Der_m;
let two_dz_drho = 2.0 * dz * drho;
drho = two_z_drho + two_dz_der + two_dz_drho;
```

---

## 6. The Problem: BLA Does Not Track Derivatives

### 6.1 The Bug

When BLA skips $L$ iterations, our code updates $\delta z$ but **does not update $\delta\rho$**:

```rust
if let Some(bla) = bla_entry {
    dz = bla.a.mul(&dz).add(&bla.b.mul(&delta_c));
    // δρ is NOT updated here!
    m += bla.l as usize;
    n += bla.l;
}
```

After the BLA skip:
- $\delta z$ correctly reflects $L$ iterations of evolution
- $\delta\rho$ still has its value from **before** the skip
- The reference orbit index $m$ has advanced by $L$

At escape, the surface normal is computed from $\rho = \text{Der}_m + \delta\rho$, but:
- $\text{Der}_m$ is now at position $m + L$
- $\delta\rho$ is stale (from position $m$)

This mismatch produces incorrect surface normals.

### 6.2 Visual Symptoms

The bug manifests as:

1. **Semi-circular artifacts** centered on the image center (the reference point)
2. **Streaks along iteration gradients** that don't follow natural Mandelbrot contours
3. **Upper/lower image asymmetry** due to varying BLA skip patterns

The artifacts appear only with 3D lighting enabled. Without lighting, iteration counts are
correct and the image appears normal.

### 6.3 Verification

Disabling BLA eliminates the artifacts, confirming BLA as the root cause. With BLA disabled:
- All iterations update $\delta\rho$ correctly
- Surface normals are accurate
- 3D lighting renders correctly
- But rendering is 10-40x slower

---

## 7. Research: How Production Renderers Handle This

### 7.1 Survey of Existing Renderers

| Renderer | BLA | Derivatives | Combined Approach |
|----------|-----|-------------|-------------------|
| Kalles Fraktaler 2+ | No (uses SA) | Yes | N/A |
| Imagina | Yes | Likely | Undocumented |
| FractalShark | Yes | Yes | Via linear approximation |
| Fraktaler-3 | Yes | Yes | Dual numbers |
| Fractalshades | Yes | Yes | Explicit derivative coefficients |

### 7.2 Approaches Found

**Dual Numbers (Fraktaler-3, mathr.co.uk)**

Claude Heiland-Allen recommends using dual numbers for automatic differentiation. A dual number
pairs a value with its derivative: $(z, \rho)$. Operations on dual numbers automatically propagate
derivatives.

However, applying BLA to dual numbers doesn't correctly handle the cross-term $2 \cdot \delta z \cdot \text{Der}_m$, because
the reference derivative $\text{Der}_m$ varies over the skipped iterations.

**Explicit Derivative Coefficients (Fractalshades)**

The Python library Fractalshades supports derivative tracking with BLA through separate
`calc_dzndc` options, though the implementation details are not publicly documented.

**Separate Computation**

Some renderers compute derivatives in a separate pass without BLA acceleration, accepting the
performance penalty for 3D-shaded renders.

### 7.3 Key Insight

No renderer we surveyed documents the mathematical extension of BLA coefficients for derivatives.
This appears to be either:
- An open problem in the fractal rendering community
- A solved problem with undocumented solutions
- Handled through alternative approaches (dual numbers, separate passes)

---

## 8. Mathematical Derivation: Extending BLA with Derivative Coefficients

### 8.1 Goal

Extend the BLA formula to include derivatives:

```math
\text{Current:}  \delta z_{m+L} = A \cdot \delta z_m + B \cdot \delta c\\
\text{New:}      \delta\rho_{m+L} = C \cdot \delta\rho_m + D \cdot \delta z_m + E \cdot \delta c
```

We need to derive coefficients $C$, $D$, $E$ and their merge formulas.

### 8.2 Single-Step Derivative Formula (Linear Approximation)

The full derivative iteration is:

```math
\delta\rho_{n+1} = 2 \cdot Z_m \cdot \delta\rho_n + 2 \cdot \delta z_n \cdot \text{Der}_m + 2 \cdot \delta z_n \cdot \delta\rho_n
```

Applying the same linear approximation used for BLA (ignoring the $\delta z \cdot \delta\rho$ term):

```math
\delta\rho_{n+1} \approx 2 \cdot Z_m \cdot \delta\rho_n + 2 \cdot \text{Der}_m \cdot \delta z_n
```

This is linear in both $\delta\rho$ and $\delta z$.

### 8.3 Single-Step Coefficients

Comparing to our target form $\delta\rho' = C \cdot \delta\rho + D \cdot \delta z + E \cdot \delta c$:

```math
\delta\rho_{m+1} = 2 \cdot Z_m \cdot \delta\rho_m + 2 \cdot \text{Der}_m \cdot \delta z_m + 0 \cdot \delta c
```

We read off the single-step coefficients:

```math
C = 2 \cdot Z_m    \quad \text{(same as } A \text{)}\\
D = 2 \cdot \text{Der}_m  \quad \text{(NEW: reference derivative)}\\
E = 0        \quad \text{(} \delta c \text{ doesn't directly affect } \delta\rho \text{ in one step)}
```

### 8.4 Deriving Merge Formulas

Given two consecutive BLAs:

**$\text{BLA}_x$ (applied first):**
```math
\delta z'  = A_x \cdot \delta z + B_x \cdot \delta c\\
\delta\rho'  = C_x \cdot \delta\rho + D_x \cdot \delta z + E_x \cdot \delta c
```

**$\text{BLA}_y$ (applied second):**
```math
\delta z'' = A_y \cdot \delta z' + B_y \cdot \delta c\\
\delta\rho'' = C_y \cdot \delta\rho' + D_y \cdot \delta z' + E_y \cdot \delta c
```

**Substituting $\delta z'$ and $\delta\rho'$ into $\text{BLA}_y$:**

For $\delta z''$:
```math
\delta z'' = A_y \cdot (A_x \cdot \delta z + B_x \cdot \delta c) + B_y \cdot \delta c\\
           = A_y \cdot A_x \cdot \delta z + A_y \cdot B_x \cdot \delta c + B_y \cdot \delta c\\
           = A_y \cdot A_x \cdot \delta z + (A_y \cdot B_x + B_y) \cdot \delta c
```

This confirms the existing merge formulas:
```math
A_{\text{merged}} = A_y \cdot A_x\\
B_{\text{merged}} = A_y \cdot B_x + B_y
```

For $\delta\rho''$:
```math
\delta\rho'' = C_y \cdot \delta\rho' + D_y \cdot \delta z' + E_y \cdot \delta c\\
             = C_y \cdot (C_x \cdot \delta\rho + D_x \cdot \delta z + E_x \cdot \delta c) + D_y \cdot (A_x \cdot \delta z + B_x \cdot \delta c) + E_y \cdot \delta c
```

Expanding:
```math
\delta\rho'' = C_y \cdot C_x \cdot \delta\rho + C_y \cdot D_x \cdot \delta z + C_y \cdot E_x \cdot \delta c + D_y \cdot A_x \cdot \delta z + D_y \cdot B_x \cdot \delta c + E_y \cdot \delta c
```

Grouping by $\delta\rho$, $\delta z$, and $\delta c$:
```math
\delta\rho'' = (C_y \cdot C_x) \cdot \delta\rho + (C_y \cdot D_x + D_y \cdot A_x) \cdot \delta z + (C_y \cdot E_x + D_y \cdot B_x + E_y) \cdot \delta c
```

The merged derivative coefficients are:

```math
C_{\text{merged}} = C_y \cdot C_x\\
D_{\text{merged}} = C_y \cdot D_x + D_y \cdot A_x\\
E_{\text{merged}} = C_y \cdot E_x + D_y \cdot B_x + E_y
```

### 8.5 Summary of All Coefficients

**Single-step BLA from orbit point $(Z_m, \text{Der}_m)$:**

| Coefficient | Formula | Purpose |
|-------------|---------|---------|
| $A$ | $2 \cdot Z_m$ | Multiplies $\delta z$ |
| $B$ | $1$ | Multiplies $\delta c$ |
| $C$ | $2 \cdot Z_m$ | Multiplies $\delta\rho$ (same as $A$) |
| $D$ | $2 \cdot \text{Der}_m$ | Cross-term: $\delta z$ affects $\delta\rho$ |
| $E$ | $0$ | $\delta c$ contribution to $\delta\rho$ |

**Merge formulas (combining $\text{BLA}_x$ then $\text{BLA}_y$):**

| Merged | Formula |
|--------|---------|
| $A$ | $A_y \cdot A_x$ |
| $B$ | $A_y \cdot B_x + B_y$ |
| $C$ | $C_y \cdot C_x$ |
| $D$ | $C_y \cdot D_x + D_y \cdot A_x$ |
| $E$ | $C_y \cdot E_x + D_y \cdot B_x + E_y$ |
| $L$ | $L_x + L_y$ |

### 8.6 Verification: Single Step

Let's verify the single-step case. Starting with:
- $\delta z_m$, $\delta\rho_m$ at iteration $m$
- Reference values $Z_m$, $\text{Der}_m$

After one standard iteration:
```math
\delta z_{m+1} = 2 \cdot Z_m \cdot \delta z_m + \delta z_m^2 + \delta c \approx 2 \cdot Z_m \cdot \delta z_m + \delta c\\
\delta\rho_{m+1} = 2 \cdot Z_m \cdot \delta\rho_m + 2 \cdot \delta z_m \cdot \text{Der}_m + 2 \cdot \delta z_m \cdot \delta\rho_m \approx 2 \cdot Z_m \cdot \delta\rho_m + 2 \cdot \text{Der}_m \cdot \delta z_m
```

Applying BLA with our coefficients:
```math
\delta z_{m+1} = A \cdot \delta z_m + B \cdot \delta c = 2 \cdot Z_m \cdot \delta z_m + 1 \cdot \delta c \quad \checkmark\\
\delta\rho_{m+1} = C \cdot \delta\rho_m + D \cdot \delta z_m + E \cdot \delta c = 2 \cdot Z_m \cdot \delta\rho_m + 2 \cdot \text{Der}_m \cdot \delta z_m + 0 \quad \checkmark
```

The formulas match.

### 8.7 Verification: Two-Step Merge

Consider two consecutive iterations at $m$ and $m+1$:

**Step 1 (from $m$ to $m+1$):**
```math
A_1 = 2 \cdot Z_m,      B_1 = 1\\
C_1 = 2 \cdot Z_m,      D_1 = 2 \cdot \text{Der}_m,      E_1 = 0
```

**Step 2 (from $m+1$ to $m+2$):**
```math
A_2 = 2 \cdot Z_{m+1},  B_2 = 1\\
C_2 = 2 \cdot Z_{m+1},  D_2 = 2 \cdot \text{Der}_{m+1},  E_2 = 0
```

**Merged (skipping 2 iterations):**
```math
A = A_2 \cdot A_1 = 4 \cdot Z_{m+1} \cdot Z_m\\
B = A_2 \cdot B_1 + B_2 = 2 \cdot Z_{m+1} + 1\\
C = C_2 \cdot C_1 = 4 \cdot Z_{m+1} \cdot Z_m\\
D = C_2 \cdot D_1 + D_2 \cdot A_1 = 2 \cdot Z_{m+1} \cdot 2 \cdot \text{Der}_m + 2 \cdot \text{Der}_{m+1} \cdot 2 \cdot Z_m\\
  = 4 \cdot Z_{m+1} \cdot \text{Der}_m + 4 \cdot \text{Der}_{m+1} \cdot Z_m\\
E = C_2 \cdot E_1 + D_2 \cdot B_1 + E_2 = 0 + 2 \cdot \text{Der}_{m+1} \cdot 1 + 0 = 2 \cdot \text{Der}_{m+1}
```

Let's verify by computing two iterations manually:

**After iteration 1:**
```math
\delta z_1 = 2 \cdot Z_m \cdot \delta z_0 + \delta c\\
\delta\rho_1 = 2 \cdot Z_m \cdot \delta\rho_0 + 2 \cdot \text{Der}_m \cdot \delta z_0
```

**After iteration 2:**
```math
\delta z_2 = 2 \cdot Z_{m+1} \cdot \delta z_1 + \delta c\\
           = 2 \cdot Z_{m+1} \cdot (2 \cdot Z_m \cdot \delta z_0 + \delta c) + \delta c\\
           = 4 \cdot Z_{m+1} \cdot Z_m \cdot \delta z_0 + 2 \cdot Z_{m+1} \cdot \delta c + \delta c\\
           = 4 \cdot Z_{m+1} \cdot Z_m \cdot \delta z_0 + (2 \cdot Z_{m+1} + 1) \cdot \delta c \quad \checkmark\\
\\
\delta\rho_2 = 2 \cdot Z_{m+1} \cdot \delta\rho_1 + 2 \cdot \text{Der}_{m+1} \cdot \delta z_1\\
             = 2 \cdot Z_{m+1} \cdot (2 \cdot Z_m \cdot \delta\rho_0 + 2 \cdot \text{Der}_m \cdot \delta z_0) + 2 \cdot \text{Der}_{m+1} \cdot (2 \cdot Z_m \cdot \delta z_0 + \delta c)\\
             = 4 \cdot Z_{m+1} \cdot Z_m \cdot \delta\rho_0 + 4 \cdot Z_{m+1} \cdot \text{Der}_m \cdot \delta z_0 + 4 \cdot \text{Der}_{m+1} \cdot Z_m \cdot \delta z_0 + 2 \cdot \text{Der}_{m+1} \cdot \delta c\\
             = 4 \cdot Z_{m+1} \cdot Z_m \cdot \delta\rho_0 + (4 \cdot Z_{m+1} \cdot \text{Der}_m + 4 \cdot \text{Der}_{m+1} \cdot Z_m) \cdot \delta z_0 + 2 \cdot \text{Der}_{m+1} \cdot \delta c \quad \checkmark
```

The merged formulas produce the correct result.

### 8.8 Validity Considerations

The existing validity radius $r^2$ applies to both $\delta z$ and $\delta\rho$ approximations because both use the
same linear approximation (ignoring quadratic terms in $\delta z$). No separate radius is needed for
derivative tracking.

---

## 9. Implementation Options

### 9.1 Option A: Extend BLA Entries with Derivative Coefficients

**Changes required:**

1. **Extend BlaEntry structure:**
```rust
pub struct BlaEntry {
    pub a: HDRComplex,      // Existing
    pub b: HDRComplex,      // Existing
    pub c: HDRComplex,      // NEW: same as a, can share
    pub d: HDRComplex,      // NEW: 2·Der_m
    pub e: HDRComplex,      // NEW: accumulated δc contribution
    pub l: u32,
    pub r_sq: HDRFloat,
}
```

Since $C = A$, we can optimize by not storing $C$ separately.

2. **Update single-step creation:**
```rust
fn from_orbit_point(z_re: f64, z_im: f64, der_re: f64, der_im: f64) -> Self {
    // A = C = 2Z, B = 1, D = 2·Der, E = 0
    let a = HDRComplex::from_f64(2.0 * z_re, 2.0 * z_im);
    let d = HDRComplex::from_f64(2.0 * der_re, 2.0 * der_im);
    let e = HDRComplex::ZERO;
    // ... rest unchanged
}
```

3. **Update merge function:**
```rust
fn merge(x: &BlaEntry, y: &BlaEntry, dc_max: &HDRFloat) -> BlaEntry {
    let a = y.a.mul(&x.a);
    let b = y.a.mul(&x.b).add(&y.b);
    // C = A (shared)
    let d = y.a.mul(&x.d).add(&y.d.mul(&x.a));  // C_y·D_x + D_y·A_x
    let e = y.a.mul(&x.e).add(&y.d.mul(&x.b)).add(&y.e);  // C_y·E_x + D_y·B_x + E_y
    // ... radius calculation unchanged
}
```

4. **Update BLA application:**
```rust
if let Some(bla) = bla_entry {
    let new_dz = bla.a.mul(&dz).add(&bla.b.mul(&delta_c));
    let new_drho = bla.a.mul(&drho)  // C·δρ (C = A)
        .add(&bla.d.mul(&dz))        // D·δz
        .add(&bla.e.mul(&delta_c));  // E·δc
    dz = new_dz;
    drho = new_drho;
    m += bla.l as usize;
    n += bla.l;
}
```

**Pros:**
- Mathematically correct
- Full BLA acceleration preserved
- No performance penalty at render time (just larger BLA entries)

**Cons:**
- Increased memory for BLA table (~50% more per entry)
- Requires changes to both CPU and GPU code
- Need to update BLA upload to GPU

### 9.2 Option B: Compute Derivatives Without BLA

**Approach:** When 3D lighting is enabled, skip BLA entirely and compute all iterations normally.

**Pros:**
- No changes to BLA structure
- Guaranteed correctness

**Cons:**
- 10-40x slower rendering with 3D lighting
- Poor user experience at deep zoom

### 9.3 Option C: Hybrid Approach

**Approach:** Use BLA for fast iteration counting, then recompute derivatives in a separate pass.

**Challenges:**
- Need to store intermediate states for derivative recomputation
- Complex bookkeeping with rebasing
- May not be faster than Option B

### 9.4 Recommendation

**Option A is the recommended approach.** The mathematical derivation is sound, the implementation
changes are localized to the BLA module, and performance remains optimal.

The memory overhead (two additional HDRComplex values per BLA entry) is acceptable given the
correctness benefits.

---

## 10. Conclusion

BLA acceleration is essential for practical deep zoom rendering, but our current implementation
fails to track derivatives during BLA skips. This causes visible artifacts in 3D-shaded renders.

We derived the mathematical extension needed: three new coefficients ($C$, $D$, $E$) with straightforward
merge formulas. Since $C = A$, only $D$ and $E$ require additional storage.

The fix requires:
1. Extending BlaEntry with $D$ and $E$ coefficients
2. Updating single-step creation to include $\text{Der}_m$
3. Updating merge to compute $D$ and $E$ correctly
4. Updating BLA application to evolve $\delta\rho$ alongside $\delta z$

With these changes, 3D lighting will render correctly at full BLA-accelerated speed.

---

## 11. References

### Primary Sources

1. **Claude Heiland-Allen (mathr.co.uk)**
   - [Deep zoom theory and practice (2021)](https://mathr.co.uk/blog/2021-05-14_deep_zoom_theory_and_practice.html)
   - [Deep zoom theory and practice (again) (2022)](https://mathr.co.uk/blog/2022-02-21_deep_zoom_theory_and_practice_again.html)
   - [Perturbation techniques applied to the Mandelbrot set (PDF)](https://mathr.co.uk/mandelbrot/perturbation.pdf)
   - [Deep Zoom reference](https://mathr.co.uk/web/deep-zoom.html)
   - [Kalles Fraktaler 2+](https://mathr.co.uk/kf/kf.html)

2. **Phil Thompson (philthompson.me)**
   - [Perturbation Theory and the Mandelbrot set](https://philthompson.me/2022/Perturbation-Theory-and-the-Mandelbrot-set.html)
   - [Faster Mandelbrot Set Rendering with BLA](https://philthompson.me/2023/Faster-Mandelbrot-Set-Rendering-with-BLA-Bivariate-Linear-Approximation.html)
   - [Smooth Colors and Slope Shading](https://philthompson.me/2022/Smooth-Colors-and-Slope-Shading-for-the-Mandelbrot-set.html)

### Software References

3. **Imagina** - [GitHub](https://github.com/5E-324/Imagina) - BLA + rebasing implementation
4. **FractalShark** - [GitHub](https://github.com/mattsaccount364/FractalShark) - GPU BLA implementation
5. **Fractalshades** - [GitHub](https://github.com/GBillotey/Fractalshades) - Python with derivative support
6. **Fraktaler-3** - [Website](https://fraktaler.mathr.co.uk/) - Dual numbers approach

### Historical

7. **K.I. Martin** - SuperFractalThing (2013) - Original perturbation paper
8. **Pauldelbrot** - Glitch detection criterion (2014)
9. **Zhuoran** - Rebasing and BLA introduction (2021, fractalforums.org)

### Distance Estimation

10. **Mu-Ency (MROB)** - [Distance Estimator](http://www.mrob.com/pub/muency/distanceestimator.html)
11. **Inigo Quilez** - [Distance to Julia set](https://iquilezles.org/articles/distancefractals/)
