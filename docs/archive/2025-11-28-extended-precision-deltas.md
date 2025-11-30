# Extended Precision Deltas Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Enable correct rendering at extreme zoom depths (10^1000+) by using BigFloat for delta values.

**Architecture:** Replace f64 delta arithmetic with BigFloat in `compute_pixel_perturbation`. Reference orbit stays f64 (proven sufficient). Message protocol changes to pass delta values as JSON strings to preserve precision. BigFloat's internal optimization (uses f64 when precision ≤ 64) means shallow zooms remain fast.

**Tech Stack:** Rust, BigFloat (Dashu FBig), serde_json

---

## Task 1: Add BigFloat delta test that fails with f64

**Files:**
- Modify: `fractalwonder-compute/src/perturbation.rs` (add test at end)

**Step 1: Write the failing test**

Add this test to verify BigFloat deltas work at extreme zoom:

```rust
#[test]
fn perturbation_with_bigfloat_deltas_no_underflow() {
    // At 10^500 zoom, f64 deltas would underflow to zero
    // This test verifies BigFloat deltas preserve the value

    let precision = 2048; // Enough for 10^500

    // Reference at origin (simple, in set)
    let c_ref = (BigFloat::zero(precision), BigFloat::zero(precision));
    let orbit = ReferenceOrbit::compute(&c_ref, 100);

    // Delta at 10^-500 scale - would be 0.0 in f64
    let delta_c = (
        BigFloat::from_string("1e-500", precision).unwrap(),
        BigFloat::from_string("1e-500", precision).unwrap(),
    );

    // This should NOT underflow - delta_c should remain non-zero
    let log2_delta = delta_c.0.log2_approx();
    assert!(log2_delta > -2000.0, "Delta should not underflow: log2 = {}", log2_delta);
    assert!(log2_delta < -1600.0, "Delta should be around 10^-500: log2 = {}", log2_delta);

    // Compute pixel - should complete without panic
    let result = compute_pixel_perturbation_bigfloat(&orbit, &delta_c, 100, TEST_TAU_SQ);

    // Point near origin with tiny offset should be in set
    assert!(!result.escaped, "Point near origin should be in set");
    assert_eq!(result.iterations, 100);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p fractalwonder-compute perturbation_with_bigfloat_deltas_no_underflow`

Expected: FAIL - `compute_pixel_perturbation_bigfloat` does not exist

**Step 3: Commit the failing test**

```bash
git add fractalwonder-compute/src/perturbation.rs
git commit -m "test: add failing test for BigFloat delta perturbation"
```

---

## Task 2: Implement compute_pixel_perturbation_bigfloat

**Files:**
- Modify: `fractalwonder-compute/src/perturbation.rs`

**Step 1: Add the BigFloat version of the function**

Add after the existing `compute_pixel_perturbation` function:

```rust
/// Compute a single pixel using perturbation with BigFloat deltas.
///
/// This version supports extreme zoom depths (10^1000+) where f64 deltas
/// would underflow to zero. The algorithm is identical to `compute_pixel_perturbation`
/// but uses BigFloat arithmetic for delta values.
///
/// # Arguments
/// * `orbit` - Pre-computed reference orbit (f64 values, bounded by escape radius)
/// * `delta_c` - Offset from reference point as BigFloat (can be 10^-1000 scale)
/// * `max_iterations` - Maximum iterations before declaring point in set
/// * `tau_sq` - Pauldelbrot glitch detection threshold squared (τ²)
pub fn compute_pixel_perturbation_bigfloat(
    orbit: &ReferenceOrbit,
    delta_c: &(BigFloat, BigFloat),
    max_iterations: u32,
    tau_sq: f64,
) -> MandelbrotData {
    let precision = delta_c.0.precision_bits();

    // δz starts at origin
    let mut dz_re = BigFloat::zero(precision);
    let mut dz_im = BigFloat::zero(precision);

    // m = reference orbit index
    let mut m: usize = 0;
    // Track precision loss via Pauldelbrot criterion
    let mut glitched = false;

    let orbit_len = orbit.orbit.len();
    if orbit_len == 0 {
        return MandelbrotData {
            iterations: 0,
            max_iterations,
            escaped: false,
            glitched: true,
        };
    }

    // Pre-create constants
    let two = BigFloat::with_precision(2.0, precision);
    let four = BigFloat::with_precision(4.0, precision);

    for n in 0..max_iterations {
        // Get Z_m with wrap-around for non-escaping references
        let z_m = orbit.orbit[m % orbit_len];
        let z_m_re = BigFloat::with_precision(z_m.0, precision);
        let z_m_im = BigFloat::with_precision(z_m.1, precision);

        // Full pixel value: z = Z_m + δz
        let z_re = z_m_re.add(&dz_re);
        let z_im = z_m_im.add(&dz_im);

        // Compute magnitudes squared (convert to f64 for comparisons - magnitudes are bounded)
        let z_mag_sq = z_re.mul(&z_re).add(&z_im.mul(&z_im)).to_f64();
        let z_m_mag_sq = z_m.0 * z_m.0 + z_m.1 * z_m.1;
        let dz_mag_sq = dz_re.mul(&dz_re).add(&dz_im.mul(&dz_im)).to_f64();

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
        if z_m_mag_sq > 1e-20 && z_mag_sq < tau_sq * z_m_mag_sq {
            glitched = true;
        }

        // 3. Rebase check: |z|² < |δz|²
        if z_mag_sq < dz_mag_sq {
            dz_re = z_re;
            dz_im = z_im;
            m = 0;
            continue;
        }

        // 4. Delta iteration: δz' = 2·Z_m·δz + δz² + δc
        // 2·Z_m·δz = 2·(z_m_re·dz_re - z_m_im·dz_im, z_m_re·dz_im + z_m_im·dz_re)
        let z_m_re_big = BigFloat::with_precision(z_m.0, precision);
        let z_m_im_big = BigFloat::with_precision(z_m.1, precision);

        let two_z_dz_re = two.mul(&z_m_re_big.mul(&dz_re).sub(&z_m_im_big.mul(&dz_im)));
        let two_z_dz_im = two.mul(&z_m_re_big.mul(&dz_im).add(&z_m_im_big.mul(&dz_re)));

        // δz² = (dz_re² - dz_im², 2·dz_re·dz_im)
        let dz_sq_re = dz_re.mul(&dz_re).sub(&dz_im.mul(&dz_im));
        let dz_sq_im = two.mul(&dz_re).mul(&dz_im);

        // δz' = 2·Z·δz + δz² + δc
        dz_re = two_z_dz_re.add(&dz_sq_re).add(&delta_c.0);
        dz_im = two_z_dz_im.add(&dz_sq_im).add(&delta_c.1);

        m += 1;
    }

    MandelbrotData {
        iterations: max_iterations,
        max_iterations,
        escaped: false,
        glitched,
    }
}
```

**Step 2: Run test to verify it passes**

Run: `cargo test -p fractalwonder-compute perturbation_with_bigfloat_deltas_no_underflow`

Expected: PASS

**Step 3: Run all perturbation tests**

Run: `cargo test -p fractalwonder-compute perturbation`

Expected: All tests pass

**Step 4: Commit**

```bash
git add fractalwonder-compute/src/perturbation.rs
git commit -m "feat: add compute_pixel_perturbation_bigfloat for deep zoom"
```

---

## Task 3: Add cross-validation test (BigFloat vs f64)

**Files:**
- Modify: `fractalwonder-compute/src/perturbation.rs` (add test)

**Step 1: Write test comparing BigFloat and f64 versions**

```rust
#[test]
fn bigfloat_matches_f64_for_shallow_zoom() {
    // At shallow zoom where f64 suffices, both versions should produce identical results
    let c_ref = (BigFloat::with_precision(-0.5, 128), BigFloat::zero(128));
    let orbit = ReferenceOrbit::compute(&c_ref, 500);

    // Test multiple delta values within f64 range
    let test_deltas = [
        (0.01, 0.01),
        (-0.005, 0.002),
        (0.1, -0.05),
        (0.0, 0.001),
    ];

    for (dx, dy) in test_deltas {
        // f64 version
        let f64_result = compute_pixel_perturbation(&orbit, (dx, dy), 500, TEST_TAU_SQ);

        // BigFloat version
        let bigfloat_delta = (
            BigFloat::with_precision(dx, 128),
            BigFloat::with_precision(dy, 128),
        );
        let bigfloat_result = compute_pixel_perturbation_bigfloat(&orbit, &bigfloat_delta, 500, TEST_TAU_SQ);

        assert_eq!(
            f64_result.escaped, bigfloat_result.escaped,
            "Escape status should match for delta ({}, {})", dx, dy
        );
        assert_eq!(
            f64_result.iterations, bigfloat_result.iterations,
            "Iteration count should match for delta ({}, {})", dx, dy
        );
    }
}
```

**Step 2: Run test**

Run: `cargo test -p fractalwonder-compute bigfloat_matches_f64_for_shallow_zoom`

Expected: PASS

**Step 3: Commit**

```bash
git add fractalwonder-compute/src/perturbation.rs
git commit -m "test: verify BigFloat matches f64 at shallow zoom"
```

---

## Task 4: Add deep zoom correctness test

**Files:**
- Modify: `fractalwonder-compute/src/perturbation.rs` (add test)

**Step 1: Write test for extreme zoom behavior**

```rust
#[test]
fn bigfloat_handles_extreme_zoom_without_artifacts() {
    // At 10^1000 zoom, verify computation completes and produces sensible results
    let precision = 4096; // ~1200 decimal digits

    // Reference at a point known to be in the set
    let c_ref = (
        BigFloat::from_string("-0.5", precision).unwrap(),
        BigFloat::zero(precision),
    );
    let orbit = ReferenceOrbit::compute(&c_ref, 1000);

    // Tiny delta - point should still be in set (near reference)
    let delta_c = (
        BigFloat::from_string("1e-1000", precision).unwrap(),
        BigFloat::from_string("1e-1000", precision).unwrap(),
    );

    let result = compute_pixel_perturbation_bigfloat(&orbit, &delta_c, 1000, TEST_TAU_SQ);

    // Nearby point should have similar behavior to reference
    assert!(!result.escaped, "Point very close to reference should be in set");
    assert_eq!(result.iterations, 1000, "Should reach max iterations");

    // Verify delta didn't underflow (would cause all points to behave identically)
    let log2_delta = delta_c.0.log2_approx();
    assert!(log2_delta.is_finite(), "Delta log2 should be finite");
    assert!(log2_delta < -3000.0, "Delta should be extremely small: {}", log2_delta);
}
```

**Step 2: Run test**

Run: `cargo test -p fractalwonder-compute bigfloat_handles_extreme_zoom_without_artifacts`

Expected: PASS

**Step 3: Commit**

```bash
git add fractalwonder-compute/src/perturbation.rs
git commit -m "test: verify BigFloat handles 10^1000 zoom"
```

---

## Task 5: Update RenderTilePerturbation message to use JSON strings

**Files:**
- Modify: `fractalwonder-core/src/messages.rs`

**Step 1: Change message definition**

Replace the `RenderTilePerturbation` variant:

```rust
    /// Render a tile using perturbation with extended precision deltas.
    RenderTilePerturbation {
        render_id: u32,
        tile: PixelRect,
        orbit_id: u32,
        /// JSON-serialized (BigFloat, BigFloat) for delta_c at tile origin
        delta_c_origin_json: String,
        /// JSON-serialized (BigFloat, BigFloat) for delta_c step per pixel
        delta_c_step_json: String,
        max_iterations: u32,
        /// Glitch detection threshold squared (τ²).
        tau_sq: f64,
    },
```

**Step 2: Update the roundtrip test**

Replace `render_tile_perturbation_roundtrip` test:

```rust
#[test]
fn render_tile_perturbation_roundtrip() {
    use crate::BigFloat;

    let delta_origin = (
        BigFloat::from_string("1e-500", 2048).unwrap(),
        BigFloat::from_string("-2e-500", 2048).unwrap(),
    );
    let delta_step = (
        BigFloat::from_string("1e-503", 2048).unwrap(),
        BigFloat::from_string("1e-503", 2048).unwrap(),
    );

    let msg = MainToWorker::RenderTilePerturbation {
        render_id: 1,
        tile: PixelRect::new(0, 0, 64, 64),
        orbit_id: 42,
        delta_c_origin_json: serde_json::to_string(&delta_origin).unwrap(),
        delta_c_step_json: serde_json::to_string(&delta_step).unwrap(),
        max_iterations: 10000,
        tau_sq: 1e-6,
    };
    let json = serde_json::to_string(&msg).unwrap();
    let parsed: MainToWorker = serde_json::from_str(&json).unwrap();
    match parsed {
        MainToWorker::RenderTilePerturbation {
            orbit_id,
            delta_c_origin_json,
            tau_sq,
            ..
        } => {
            assert_eq!(orbit_id, 42);
            assert!((tau_sq - 1e-6).abs() < 1e-12);

            // Verify BigFloat survives roundtrip
            let parsed_origin: (BigFloat, BigFloat) = serde_json::from_str(&delta_c_origin_json).unwrap();
            assert_eq!(parsed_origin.0.precision_bits(), 2048);

            // Verify extreme value preserved
            let log2 = parsed_origin.0.log2_approx();
            assert!(log2 < -1600.0, "Delta should be ~10^-500");
        }
        _ => panic!("Wrong variant"),
    }
}
```

**Step 3: Run tests**

Run: `cargo test -p fractalwonder-core messages`

Expected: PASS

**Step 4: Commit**

```bash
git add fractalwonder-core/src/messages.rs
git commit -m "feat: use JSON strings for delta values in RenderTilePerturbation"
```

---

## Task 6: Update worker to use BigFloat perturbation

**Files:**
- Modify: `fractalwonder-compute/src/worker.rs`

**Step 1: Update the import**

Change line 3:

```rust
use crate::{
    compute_pixel_perturbation_bigfloat, MandelbrotRenderer, ReferenceOrbit, Renderer,
    TestImageRenderer,
};
```

**Step 2: Update RenderTilePerturbation handler**

Replace the `MainToWorker::RenderTilePerturbation` match arm (lines 271-324):

```rust
        MainToWorker::RenderTilePerturbation {
            render_id,
            tile,
            orbit_id,
            delta_c_origin_json,
            delta_c_step_json,
            max_iterations,
            tau_sq,
        } => {
            // Parse BigFloat deltas from JSON
            let delta_c_origin: (BigFloat, BigFloat) = match serde_json::from_str(&delta_c_origin_json) {
                Ok(d) => d,
                Err(e) => {
                    post_message(&WorkerToMain::Error {
                        message: format!("Failed to parse delta_c_origin: {}", e),
                    });
                    return;
                }
            };

            let delta_c_step: (BigFloat, BigFloat) = match serde_json::from_str(&delta_c_step_json) {
                Ok(d) => d,
                Err(e) => {
                    post_message(&WorkerToMain::Error {
                        message: format!("Failed to parse delta_c_step: {}", e),
                    });
                    return;
                }
            };

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

            // Compute all pixels in tile using BigFloat deltas
            let mut data = Vec::with_capacity((tile.width * tile.height) as usize);
            let mut delta_c_row_re = delta_c_origin.0.clone();
            let mut delta_c_row_im = delta_c_origin.1.clone();

            for _py in 0..tile.height {
                let mut delta_c_re = delta_c_row_re.clone();
                let mut delta_c_im = delta_c_row_im.clone();

                for _px in 0..tile.width {
                    let delta_c = (delta_c_re.clone(), delta_c_im.clone());
                    let result =
                        compute_pixel_perturbation_bigfloat(&orbit, &delta_c, max_iterations, tau_sq);
                    data.push(ComputeData::Mandelbrot(result));

                    delta_c_re = delta_c_re.add(&delta_c_step.0);
                }

                delta_c_row_im = delta_c_row_im.add(&delta_c_step.1);
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

**Step 3: Run build check**

Run: `cargo check -p fractalwonder-compute`

Expected: Success (no errors)

**Step 4: Commit**

```bash
git add fractalwonder-compute/src/worker.rs
git commit -m "feat: worker uses BigFloat perturbation for deep zoom support"
```

---

## Task 7: Export the new function from lib.rs

**Files:**
- Modify: `fractalwonder-compute/src/lib.rs`

**Step 1: Check current exports and add new function**

Read the file first, then add `compute_pixel_perturbation_bigfloat` to the public exports alongside `compute_pixel_perturbation`.

**Step 2: Run full build**

Run: `cargo build -p fractalwonder-compute`

Expected: Success

**Step 3: Run all tests**

Run: `cargo test --workspace`

Expected: All tests pass

**Step 4: Commit**

```bash
git add fractalwonder-compute/src/lib.rs
git commit -m "feat: export compute_pixel_perturbation_bigfloat"
```

---

## Task 8: Run quality checks

**Step 1: Format**

Run: `cargo fmt --all`

**Step 2: Clippy**

Run: `cargo clippy --all-targets --all-features -- -D warnings -W clippy::all`

Expected: No warnings or errors

**Step 3: Full test suite**

Run: `cargo test --workspace --all-targets --all-features -- --nocapture`

Expected: All tests pass

**Step 4: Commit any formatting fixes**

```bash
git add -A
git commit -m "chore: format and lint fixes" --allow-empty
```

---

## Summary

After completing all tasks:

1. `compute_pixel_perturbation_bigfloat` handles delta arithmetic at any precision
2. Message protocol preserves BigFloat precision via JSON strings
3. Worker uses BigFloat perturbation for all tile rendering
4. Cross-validated against f64 version at shallow zoom
5. Tested at 10^1000 zoom depth

**Performance note:** This implementation prioritizes correctness. Increment 3 (FloatExp) can optimize if BigFloat proves too slow for interactive use.

---

Plan complete and saved to `docs/plans/2025-11-28-extended-precision-deltas.md`. Two execution options:

**1. Subagent-Driven (this session)** - I dispatch fresh subagent per task, review between tasks, fast iteration

**2. Parallel Session (separate)** - Open new session with executing-plans, batch execution with checkpoints

Which approach?
