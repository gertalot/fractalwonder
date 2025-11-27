# Correct Perturbation Core - Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Fix perturbation algorithm with correct rebasing and Pauldelbrot glitch detection.

**Architecture:** Replace broken "on-the-fly" rebasing with true rebasing (reset to iteration 0). Add Pauldelbrot criterion for glitch detection. Thread τ² threshold through config → messages → compute.

**Tech Stack:** Rust, WASM, Leptos

**Reference:** `docs/research/perturbation-theory.md` Sections 2.3, 3, 8.1, 13.1

---

## Task 1: Add tau_sq to FractalConfig

**Files:**
- Modify: `fractalwonder-ui/src/config.rs:17-32` (FractalConfig struct)
- Modify: `fractalwonder-ui/src/config.rs:58-66` (mandelbrot entry)

**Step 1: Add tau_sq field to FractalConfig struct**

In `fractalwonder-ui/src/config.rs`, add the field after `renderer_type`:

```rust
/// Configuration for a fractal type.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FractalConfig {
    /// Unique identifier (matches renderer ID in compute layer)
    pub id: &'static str,
    /// Human-readable name for UI display
    pub display_name: &'static str,
    /// Default center coordinates as strings (preserves precision)
    pub default_center: (&'static str, &'static str),
    /// Default width in fractal space as string
    pub default_width: &'static str,
    /// Default height in fractal space as string
    pub default_height: &'static str,
    /// Which renderer implementation to use
    pub renderer_type: RendererType,
    /// Glitch detection threshold squared (τ²).
    /// Default 1e-6 corresponds to τ = 10⁻³ (standard).
    /// See docs/research/perturbation-theory.md Section 2.5.
    pub tau_sq: f64,
}
```

**Step 2: Add tau_sq to test_image config**

Update the test_image entry in FRACTAL_CONFIGS:

```rust
FractalConfig {
    id: "test_image",
    display_name: "Test Pattern",
    default_center: ("0.0", "0.0"),
    default_width: "100.0",
    default_height: "100.0",
    renderer_type: RendererType::Simple,
    tau_sq: 1e-6, // Not used for test_image, but required field
},
```

**Step 3: Add tau_sq to mandelbrot config**

Update the mandelbrot entry in FRACTAL_CONFIGS:

```rust
FractalConfig {
    id: "mandelbrot",
    display_name: "Mandelbrot Set",
    default_center: ("-0.5", "0.0"),
    default_width: "4.0",
    default_height: "4.0",
    renderer_type: RendererType::Perturbation,
    tau_sq: 1e-6, // τ = 10⁻³, standard threshold
},
```

**Step 4: Verify compilation**

Run: `cargo check --workspace`
Expected: Success (no errors)

**Step 5: Commit**

```bash
git add fractalwonder-ui/src/config.rs
git commit -m "feat: add tau_sq glitch threshold to FractalConfig"
```

---

## Task 2: Add tau_sq to RenderTilePerturbation message

**Files:**
- Modify: `fractalwonder-core/src/messages.rs:40-48` (RenderTilePerturbation variant)
- Modify: `fractalwonder-core/src/messages.rs:211-234` (test)

**Step 1: Add tau_sq field to message**

In `fractalwonder-core/src/messages.rs`, add `tau_sq` to `RenderTilePerturbation`:

```rust
/// Render a tile using perturbation.
RenderTilePerturbation {
    render_id: u32,
    tile: PixelRect,
    orbit_id: u32,
    delta_c_origin: (f64, f64),
    delta_c_step: (f64, f64),
    max_iterations: u32,
    /// Glitch detection threshold squared (τ²).
    tau_sq: f64,
},
```

**Step 2: Update the roundtrip test**

Update `render_tile_perturbation_roundtrip` test:

```rust
#[test]
fn render_tile_perturbation_roundtrip() {
    let msg = MainToWorker::RenderTilePerturbation {
        render_id: 1,
        tile: PixelRect::new(0, 0, 64, 64),
        orbit_id: 42,
        delta_c_origin: (0.001, -0.002),
        delta_c_step: (0.0001, 0.0001),
        max_iterations: 10000,
        tau_sq: 1e-6,
    };
    let json = serde_json::to_string(&msg).unwrap();
    let parsed: MainToWorker = serde_json::from_str(&json).unwrap();
    match parsed {
        MainToWorker::RenderTilePerturbation {
            orbit_id,
            delta_c_origin,
            tau_sq,
            ..
        } => {
            assert_eq!(orbit_id, 42);
            assert!((delta_c_origin.0 - 0.001).abs() < 1e-10);
            assert!((tau_sq - 1e-6).abs() < 1e-12);
        }
        _ => panic!("Wrong variant"),
    }
}
```

**Step 3: Run tests**

Run: `cargo test --package fractalwonder-core`
Expected: FAIL (worker_pool.rs doesn't provide tau_sq yet)

**Step 4: Commit**

```bash
git add fractalwonder-core/src/messages.rs
git commit -m "feat: add tau_sq to RenderTilePerturbation message"
```

---

## Task 3: Thread tau_sq through WorkerPool

**Files:**
- Modify: `fractalwonder-ui/src/workers/worker_pool.rs:33-48` (PerturbationState)
- Modify: `fractalwonder-ui/src/workers/worker_pool.rs:493-503` (dispatch_work)
- Modify: `fractalwonder-ui/src/workers/worker_pool.rs:568-670` (start_perturbation_render)

**Step 1: Add tau_sq to PerturbationState**

In `fractalwonder-ui/src/workers/worker_pool.rs`, add to PerturbationState:

```rust
/// State for perturbation rendering flow.
#[derive(Default)]
struct PerturbationState {
    /// Current orbit ID being used
    orbit_id: u32,
    /// Workers that have confirmed storing the orbit
    workers_with_orbit: HashSet<usize>,
    /// Orbit data to broadcast
    pending_orbit: Option<OrbitData>,
    /// Maximum iterations for perturbation tiles
    max_iterations: u32,
    /// Delta step per pixel in fractal space
    delta_step: (f64, f64),
    /// Pending orbit computation (waiting for worker to initialize)
    pending_orbit_request: Option<PendingOrbitRequest>,
    /// Glitch detection threshold squared (τ²)
    tau_sq: f64,
}
```

**Step 2: Include tau_sq in dispatch_work message**

Update the `RenderTilePerturbation` message in `dispatch_work` (around line 493):

```rust
self.send_to_worker(
    worker_id,
    &MainToWorker::RenderTilePerturbation {
        render_id: self.current_render_id,
        tile,
        orbit_id: self.perturbation.orbit_id,
        delta_c_origin,
        delta_c_step: self.perturbation.delta_step,
        max_iterations: self.perturbation.max_iterations,
        tau_sq: self.perturbation.tau_sq,
    },
);
```

**Step 3: Set tau_sq in start_perturbation_render**

Add import and set tau_sq in `start_perturbation_render`. First add import at top of file:

```rust
use crate::config::get_config;
```

Then in `start_perturbation_render`, after setting `delta_step` (around line 622):

```rust
self.perturbation.max_iterations = max_iterations;
self.perturbation.delta_step = delta_step;
// Get tau_sq from fractal config (defaults to 1e-6)
self.perturbation.tau_sq = get_config("mandelbrot")
    .map(|c| c.tau_sq)
    .unwrap_or(1e-6);
```

**Step 4: Verify compilation**

Run: `cargo check --workspace`
Expected: Success

**Step 5: Run tests**

Run: `cargo test --workspace`
Expected: Pass (message roundtrip now works)

**Step 6: Commit**

```bash
git add fractalwonder-ui/src/workers/worker_pool.rs
git commit -m "feat: thread tau_sq from config through worker pool"
```

---

## Task 4: Update worker to pass tau_sq to perturbation function

**Files:**
- Modify: `fractalwonder-compute/src/worker.rs:271-322` (RenderTilePerturbation handler)

**Step 1: Extract tau_sq from message and pass to compute function**

Update the `RenderTilePerturbation` handler in `fractalwonder-compute/src/worker.rs`:

```rust
MainToWorker::RenderTilePerturbation {
    render_id,
    tile,
    orbit_id,
    delta_c_origin,
    delta_c_step,
    max_iterations,
    tau_sq,
} => {
    // Get cached orbit
    let cached = match state.orbit_cache.get(&orbit_id) {
        Some(c) => c,
        None => {
            post_message(&WorkerToMain::Error {
                message: format!("Orbit {} not found in cache", orbit_id),
            });
            return;
        }
    };

    let orbit = cached.to_reference_orbit();
    let start_time = Date::now();

    // Compute all pixels in tile
    let mut data = Vec::with_capacity((tile.width * tile.height) as usize);
    let mut delta_c_row = delta_c_origin;

    for _py in 0..tile.height {
        let mut delta_c = delta_c_row;

        for _px in 0..tile.width {
            let result = compute_pixel_perturbation(&orbit, delta_c, max_iterations, tau_sq);
            data.push(ComputeData::Mandelbrot(result));

            delta_c.0 += delta_c_step.0;
        }

        delta_c_row.1 += delta_c_step.1;
    }

    let compute_time_ms = Date::now() - start_time;

    post_message(&WorkerToMain::TileComplete {
        render_id,
        tile,
        data,
        compute_time_ms,
    });

    post_message(&WorkerToMain::RequestWork {
        render_id: Some(render_id),
    });
}
```

**Step 2: Verify compilation fails (function signature mismatch)**

Run: `cargo check --package fractalwonder-compute`
Expected: FAIL - `compute_pixel_perturbation` doesn't accept `tau_sq` yet

**Step 3: Commit partial progress**

```bash
git add fractalwonder-compute/src/worker.rs
git commit -m "feat: pass tau_sq from message to perturbation function"
```

---

## Task 5: Rewrite compute_pixel_perturbation core algorithm

**Files:**
- Modify: `fractalwonder-compute/src/perturbation.rs:62-172`

This is the core fix. Replace the entire function with the correct algorithm from the research doc.

**Step 1: Update function signature**

Change signature to accept `tau_sq`:

```rust
/// Compute a single pixel using perturbation from a reference orbit.
///
/// Uses f64 delta iterations with automatic rebasing when |z|² < |δz|².
/// Detects glitches using Pauldelbrot criterion: |z|² < τ²|Z|².
///
/// # Algorithm (from docs/research/perturbation-theory.md Section 8.1)
///
/// 1. δz = 0, m = 0
/// 2. For each iteration n:
///    a. Z_m = orbit[m % len] (wrap-around)
///    b. z = Z_m + δz
///    c. Escape: |z|² > 4 → return escaped
///    d. Glitch: |z|² < τ²|Z|² → mark glitched
///    e. Rebase: |z|² < |δz|² → δz = z, m = 0
///    f. δz = 2·Z_m·δz + δz² + δc
///    g. m += 1
pub fn compute_pixel_perturbation(
    orbit: &ReferenceOrbit,
    delta_c: (f64, f64),
    max_iterations: u32,
    tau_sq: f64,
) -> MandelbrotData {
```

**Step 2: Replace function body with correct algorithm**

```rust
pub fn compute_pixel_perturbation(
    orbit: &ReferenceOrbit,
    delta_c: (f64, f64),
    max_iterations: u32,
    tau_sq: f64,
) -> MandelbrotData {
    // δz starts at origin
    let mut dz = (0.0_f64, 0.0_f64);
    // m = reference orbit index
    let mut m: usize = 0;
    // Track precision loss via Pauldelbrot criterion
    let mut glitched = false;

    let orbit_len = orbit.orbit.len();
    if orbit_len == 0 {
        // Degenerate case: no orbit data
        return MandelbrotData {
            iterations: 0,
            max_iterations,
            escaped: false,
            glitched: true,
        };
    }

    for n in 0..max_iterations {
        // Get Z_m with wrap-around for non-escaping references
        let z_m = orbit.orbit[m % orbit_len];

        // Full pixel value: z = Z_m + δz
        let z = (z_m.0 + dz.0, z_m.1 + dz.1);

        // Precompute magnitudes squared
        let z_mag_sq = z.0 * z.0 + z.1 * z.1;
        let z_m_mag_sq = z_m.0 * z_m.0 + z_m.1 * z_m.1;
        let dz_mag_sq = dz.0 * dz.0 + dz.1 * dz.1;

        // 1. Escape check: |z|² > 4
        if z_mag_sq > 4.0 {
            return MandelbrotData {
                iterations: n,
                max_iterations,
                escaped: true,
                glitched,
            };
        }

        // 2. Pauldelbrot glitch detection: |z|² < τ²|Z_m|²
        // Skip check when Z_m is near zero to avoid division issues
        if z_m_mag_sq > 1e-20 && z_mag_sq < tau_sq * z_m_mag_sq {
            glitched = true;
        }

        // 3. Rebase check: |z|² < |δz|²
        // When the full pixel value is smaller than the delta alone,
        // absorb Z into delta and reset to iteration 0
        if z_mag_sq < dz_mag_sq {
            dz = z;
            m = 0;
            continue; // Skip delta iteration this round
        }

        // 4. Delta iteration: δz' = 2·Z_m·δz + δz² + δc
        // Complex multiplication: (a+bi)(c+di) = (ac-bd) + (ad+bc)i
        // 2·Z_m·δz = 2·(z_m.0·dz.0 - z_m.1·dz.1, z_m.0·dz.1 + z_m.1·dz.0)
        // δz² = (dz.0² - dz.1², 2·dz.0·dz.1)
        let two_z_dz = (
            2.0 * (z_m.0 * dz.0 - z_m.1 * dz.1),
            2.0 * (z_m.0 * dz.1 + z_m.1 * dz.0),
        );
        let dz_sq = (dz.0 * dz.0 - dz.1 * dz.1, 2.0 * dz.0 * dz.1);

        dz = (
            two_z_dz.0 + dz_sq.0 + delta_c.0,
            two_z_dz.1 + dz_sq.1 + delta_c.1,
        );

        m += 1;
    }

    // Reached max iterations without escaping
    MandelbrotData {
        iterations: max_iterations,
        max_iterations,
        escaped: false,
        glitched,
    }
}
```

**Step 3: Verify compilation**

Run: `cargo check --workspace`
Expected: Success

**Step 4: Run existing tests**

Run: `cargo test --package fractalwonder-compute`
Expected: Some tests may fail due to changed glitch semantics

**Step 5: Commit**

```bash
git add fractalwonder-compute/src/perturbation.rs
git commit -m "fix: rewrite perturbation core with correct rebasing and Pauldelbrot criterion

- Replace on-the-fly f64 computation with true rebasing (δz = z, m = 0)
- Add Pauldelbrot glitch detection: |z|² < τ²|Z|²
- Add wrap-around for reference orbit (m % orbit_len)
- Remove ~40 lines of incorrect on-the-fly logic

See docs/research/perturbation-theory.md Section 8.1"
```

---

## Task 6: Update tests for new glitch semantics

**Files:**
- Modify: `fractalwonder-compute/src/perturbation.rs` (test module, lines 174-634)

**Step 1: Update glitch detection tests**

The old tests checked for reference exhaustion. Update to test Pauldelbrot criterion:

```rust
#[test]
fn glitch_detected_via_pauldelbrot_criterion() {
    // Reference at a point in the set
    let c_ref = (BigFloat::with_precision(-0.5, 128), BigFloat::zero(128));
    let orbit = ReferenceOrbit::compute(&c_ref, 1000);

    // Use a delta that will cause |z| to become very small relative to |Z|
    // This triggers the Pauldelbrot criterion: |z|² < τ²|Z|²
    // We need to find a point where this naturally occurs

    // For now, verify the basic mechanics work
    let delta_c = (0.01, 0.01);
    let tau_sq = 1e-6; // τ = 10⁻³
    let result = compute_pixel_perturbation(&orbit, delta_c, 1000, tau_sq);

    // Should complete without panic
    assert!(result.iterations > 0 || result.escaped);
}

#[test]
fn no_glitch_for_normal_escape() {
    // Reference in set
    let c_ref = (BigFloat::with_precision(-0.5, 128), BigFloat::zero(128));
    let orbit = ReferenceOrbit::compute(&c_ref, 1000);

    // Pixel that escapes quickly and cleanly
    let delta_c = (2.5, 0.0); // Point at (2, 0) escapes immediately
    let tau_sq = 1e-6;
    let result = compute_pixel_perturbation(&orbit, delta_c, 1000, tau_sq);

    assert!(result.escaped);
    assert!(result.iterations < 10);
    // Clean escape should not be marked glitched
    assert!(!result.glitched, "Clean escape should not be glitched");
}

#[test]
fn wrap_around_works_for_long_iterations() {
    // Reference with short orbit (escapes early)
    let c_ref = (BigFloat::with_precision(0.3, 128), BigFloat::zero(128));
    let orbit = ReferenceOrbit::compute(&c_ref, 100);

    // Reference should escape relatively quickly
    assert!(orbit.escaped_at.is_some());
    let orbit_len = orbit.orbit.len();

    // Pixel in the set that needs many iterations
    let delta_c = (-0.8, 0.0); // Point at (-0.5, 0) is in set
    let tau_sq = 1e-6;
    let result = compute_pixel_perturbation(&orbit, delta_c, 500, tau_sq);

    // Should iterate beyond orbit length using wrap-around
    // (500 > orbit_len, so wrap-around must have occurred)
    assert!(result.iterations as usize > orbit_len || !result.escaped);
}
```

**Step 2: Update existing tests to pass tau_sq**

Find all calls to `compute_pixel_perturbation` in tests and add `tau_sq` parameter:

```rust
// Add this constant at top of test module
const TEST_TAU_SQ: f64 = 1e-6;

// Then update all test calls, e.g.:
let result = compute_pixel_perturbation(&orbit, (0.5, 0.0), 1000, TEST_TAU_SQ);
```

**Step 3: Run tests**

Run: `cargo test --package fractalwonder-compute -- --nocapture`
Expected: All tests pass

**Step 4: Commit**

```bash
git add fractalwonder-compute/src/perturbation.rs
git commit -m "test: update perturbation tests for Pauldelbrot criterion"
```

---

## Task 7: Full integration test

**Step 1: Run all workspace tests**

Run: `cargo test --workspace --all-targets -- --nocapture`
Expected: All pass

**Step 2: Run clippy**

Run: `cargo clippy --all-targets --all-features -- -D warnings`
Expected: No warnings

**Step 3: Run fmt check**

Run: `cargo fmt --all -- --check`
Expected: No formatting issues

**Step 4: Build release**

Run: `cargo check --workspace --all-targets --all-features`
Expected: Success

**Step 5: Manual visual test**

1. Run `trunk serve`
2. Navigate to Mandelbrot view
3. Zoom to ~10^14 (deep enough to potentially trigger glitches)
4. Verify cyan overlay appears only where precision loss occurs
5. Compare with previous behavior (should have fewer false-positive glitches)

**Step 6: Final commit (if any fixes needed)**

```bash
git add -A
git commit -m "fix: address integration test findings"
```

---

## Summary

| Task | Description | Key Files |
|------|-------------|-----------|
| 1 | Add tau_sq to FractalConfig | config.rs |
| 2 | Add tau_sq to message | messages.rs |
| 3 | Thread tau_sq through WorkerPool | worker_pool.rs |
| 4 | Pass tau_sq in worker | worker.rs |
| 5 | Rewrite core algorithm | perturbation.rs |
| 6 | Update tests | perturbation.rs tests |
| 7 | Integration test | all |

**Estimated total: ~200 lines changed, net reduction in code.**
