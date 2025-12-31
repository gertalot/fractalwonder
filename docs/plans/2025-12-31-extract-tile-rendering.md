# Extract Tile Rendering from Worker Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Extract tile rendering logic from `worker.rs` into a testable `perturbation/tile.rs` module, following the established patterns in the perturbation module.

**Architecture:** Create a pure `render_tile` function that takes parsed inputs (orbit, deltas, config) and returns tile data + stats. The worker handler becomes thin dispatch: parse → call → serialize. This matches how `compute_pixel_perturbation` is already structured.

**Tech Stack:** Rust, BigFloat, HDRFloat, HDRComplex, existing perturbation module types.

---

## Context

### Current State
- `worker.rs:203-367` contains ~165 lines of inline tile rendering logic
- Two code paths: f64 (lines 273-298) and HDRFloat (lines 299-350)
- Logic is untestable without WASM infrastructure

### Target State
- New `perturbation/tile.rs` with pure functions
- Worker handler reduced to ~20 lines of dispatch
- Tile rendering testable alongside other perturbation tests

### Files Involved
- Create: `fractalwonder-compute/src/perturbation/tile.rs`
- Modify: `fractalwonder-compute/src/perturbation/mod.rs`
- Modify: `fractalwonder-compute/src/lib.rs`
- Modify: `fractalwonder-compute/src/worker.rs`
- Create: `fractalwonder-compute/src/perturbation/tests/tile.rs`
- Modify: `fractalwonder-compute/src/perturbation/tests/mod.rs`

---

## Task 1: Create Tile Result Types

**Files:**
- Create: `fractalwonder-compute/src/perturbation/tile.rs`
- Modify: `fractalwonder-compute/src/perturbation/mod.rs`

**Step 1: Create the tile module with result types**

Create `fractalwonder-compute/src/perturbation/tile.rs`:

```rust
//! Tile rendering for perturbation-based Mandelbrot computation.
//!
//! Provides pure functions for rendering tiles using pre-computed reference orbits.
//! Supports both f64 (fast path) and HDRFloat (deep zoom) precision.

use fractalwonder_core::ComputeData;

/// Statistics from rendering a tile.
#[derive(Clone, Debug, Default)]
pub struct TileStats {
    /// Iterations skipped via BLA across all pixels.
    pub bla_iterations: u64,
    /// Total iterations computed (BLA + standard) across all pixels.
    pub total_iterations: u64,
}

/// Result of rendering a tile.
#[derive(Clone, Debug)]
pub struct TileRenderResult {
    /// Computed data for each pixel in row-major order.
    pub data: Vec<ComputeData>,
    /// Rendering statistics.
    pub stats: TileStats,
}
```

**Step 2: Export from mod.rs**

Add to `fractalwonder-compute/src/perturbation/mod.rs` after line 8:

```rust
mod tile;

pub use tile::{TileRenderResult, TileStats};
```

**Step 3: Run clippy to verify no errors**

Run: `cargo clippy -p fractalwonder-compute --all-targets -- -D warnings`
Expected: No errors (warnings about unused code are OK at this stage)

**Step 4: Commit**

```bash
git add fractalwonder-compute/src/perturbation/tile.rs fractalwonder-compute/src/perturbation/mod.rs
git commit -m "refactor: add TileRenderResult and TileStats types"
```

---

## Task 2: Add Tile Rendering Configuration

**Files:**
- Modify: `fractalwonder-compute/src/perturbation/tile.rs`

**Step 1: Add configuration struct**

Add to `tile.rs` after the `TileRenderResult` struct:

```rust
/// Configuration for tile rendering.
#[derive(Clone, Debug)]
pub struct TileConfig {
    /// Tile dimensions (width, height).
    pub size: (u32, u32),
    /// Maximum iterations for escape check.
    pub max_iterations: u32,
    /// Glitch detection threshold squared (τ²).
    pub tau_sq: f64,
    /// Enable BLA acceleration (only applies to HDRFloat path).
    pub bla_enabled: bool,
}
```

**Step 2: Run clippy**

Run: `cargo clippy -p fractalwonder-compute --all-targets -- -D warnings`
Expected: No errors

**Step 3: Commit**

```bash
git add fractalwonder-compute/src/perturbation/tile.rs
git commit -m "refactor: add TileConfig for tile rendering parameters"
```

---

## Task 3: Extract f64 Tile Rendering Function

**Files:**
- Modify: `fractalwonder-compute/src/perturbation/tile.rs`
- Create: `fractalwonder-compute/src/perturbation/tests/tile.rs`
- Modify: `fractalwonder-compute/src/perturbation/tests/mod.rs`

**Step 1: Write the failing test**

Create `fractalwonder-compute/src/perturbation/tests/tile.rs`:

```rust
use crate::perturbation::tile::{render_tile_f64, TileConfig, TileRenderResult};
use crate::ReferenceOrbit;
use fractalwonder_core::{BigFloat, ComputeData};

#[test]
fn render_tile_f64_produces_correct_pixel_count() {
    // Create a simple reference orbit at c = -0.5
    let c_ref = (BigFloat::with_precision(-0.5, 64), BigFloat::zero(64));
    let orbit = ReferenceOrbit::compute(&c_ref, 100);

    let config = TileConfig {
        size: (4, 4),
        max_iterations: 100,
        tau_sq: 1e-6,
        bla_enabled: false,
    };

    // Delta origin and step for a 4x4 tile
    let delta_origin = (0.1, 0.1);
    let delta_step = (0.01, 0.01);

    let result = render_tile_f64(&orbit, delta_origin, delta_step, &config);

    assert_eq!(result.data.len(), 16, "4x4 tile should produce 16 pixels");
    assert!(
        result.data.iter().all(|d| matches!(d, ComputeData::Mandelbrot(_))),
        "All pixels should be Mandelbrot data"
    );
}

#[test]
fn render_tile_f64_escapes_outside_set() {
    // Reference at origin
    let c_ref = (BigFloat::zero(64), BigFloat::zero(64));
    let orbit = ReferenceOrbit::compute(&c_ref, 100);

    let config = TileConfig {
        size: (2, 2),
        max_iterations: 100,
        tau_sq: 1e-6,
        bla_enabled: false,
    };

    // Delta puts pixels outside the set (|c| > 2)
    let delta_origin = (2.5, 2.5);
    let delta_step = (0.1, 0.1);

    let result = render_tile_f64(&orbit, delta_origin, delta_step, &config);

    // All pixels should escape quickly
    for pixel in &result.data {
        if let ComputeData::Mandelbrot(m) = pixel {
            assert!(m.escaped, "Pixels at |c| > 2 should escape");
            assert!(m.iterations < 10, "Should escape within few iterations");
        }
    }
}
```

**Step 2: Register test module**

Add to `fractalwonder-compute/src/perturbation/tests/mod.rs`:

```rust
mod tile;
```

**Step 3: Run test to verify it fails**

Run: `cargo test -p fractalwonder-compute render_tile_f64 -- --nocapture`
Expected: FAIL with "cannot find function `render_tile_f64`"

**Step 4: Implement render_tile_f64**

Add to `fractalwonder-compute/src/perturbation/tile.rs`:

```rust
use super::{compute_pixel_perturbation, ReferenceOrbit};
use fractalwonder_core::{ComputeData, F64Complex};

/// Render a tile using f64 precision (fast path for moderate zoom levels).
///
/// This path is used when delta values fit comfortably in f64 range (~10^±300).
/// BLA is not supported in this path.
pub fn render_tile_f64(
    orbit: &ReferenceOrbit,
    delta_origin: (f64, f64),
    delta_step: (f64, f64),
    config: &TileConfig,
) -> TileRenderResult {
    let capacity = (config.size.0 * config.size.1) as usize;
    let mut data = Vec::with_capacity(capacity);
    let mut stats = TileStats::default();

    let mut delta_c_row = delta_origin;

    for _py in 0..config.size.1 {
        let mut delta_c = delta_c_row;

        for _px in 0..config.size.0 {
            let result = compute_pixel_perturbation(
                orbit,
                F64Complex::from_f64_pair(delta_c.0, delta_c.1),
                config.max_iterations,
                config.tau_sq,
            );
            stats.total_iterations += result.iterations as u64;
            data.push(ComputeData::Mandelbrot(result));

            delta_c.0 += delta_step.0;
        }

        delta_c_row.1 += delta_step.1;
    }

    TileRenderResult { data, stats }
}
```

**Step 5: Run tests to verify they pass**

Run: `cargo test -p fractalwonder-compute render_tile_f64 -- --nocapture`
Expected: 2 tests PASS

**Step 6: Run full test suite**

Run: `cargo test --workspace -- --nocapture`
Expected: All tests pass

**Step 7: Commit**

```bash
git add fractalwonder-compute/src/perturbation/tile.rs fractalwonder-compute/src/perturbation/tests/tile.rs fractalwonder-compute/src/perturbation/tests/mod.rs
git commit -m "feat: add render_tile_f64 for f64 precision tile rendering"
```

---

## Task 4: Extract HDRFloat Tile Rendering Function

**Files:**
- Modify: `fractalwonder-compute/src/perturbation/tile.rs`
- Modify: `fractalwonder-compute/src/perturbation/tests/tile.rs`

**Step 1: Write the failing test**

Add to `fractalwonder-compute/src/perturbation/tests/tile.rs`:

```rust
use crate::perturbation::tile::render_tile_hdr;
use crate::BlaTable;
use fractalwonder_core::HDRFloat;

#[test]
fn render_tile_hdr_produces_correct_pixel_count() {
    let c_ref = (BigFloat::with_precision(-0.5, 64), BigFloat::zero(64));
    let orbit = ReferenceOrbit::compute(&c_ref, 100);

    let dc_max = HDRFloat::from_f64(0.1);
    let bla_table = BlaTable::compute(&orbit, &dc_max);

    let config = TileConfig {
        size: (4, 4),
        max_iterations: 100,
        tau_sq: 1e-6,
        bla_enabled: true,
    };

    // Use HDRFloat deltas
    let delta_origin = (HDRFloat::from_f64(0.1), HDRFloat::from_f64(0.1));
    let delta_step = (HDRFloat::from_f64(0.01), HDRFloat::from_f64(0.01));

    let result = render_tile_hdr(&orbit, Some(&bla_table), delta_origin, delta_step, &config);

    assert_eq!(result.data.len(), 16, "4x4 tile should produce 16 pixels");
}

#[test]
fn render_tile_hdr_without_bla_table() {
    let c_ref = (BigFloat::with_precision(-0.5, 64), BigFloat::zero(64));
    let orbit = ReferenceOrbit::compute(&c_ref, 100);

    let config = TileConfig {
        size: (2, 2),
        max_iterations: 100,
        tau_sq: 1e-6,
        bla_enabled: true, // Enabled but no table provided
    };

    let delta_origin = (HDRFloat::from_f64(0.1), HDRFloat::from_f64(0.1));
    let delta_step = (HDRFloat::from_f64(0.01), HDRFloat::from_f64(0.01));

    // Should work without BLA table (falls back to standard iteration)
    let result = render_tile_hdr(&orbit, None, delta_origin, delta_step, &config);

    assert_eq!(result.data.len(), 4);
    assert_eq!(result.stats.bla_iterations, 0, "No BLA without table");
}

#[test]
fn render_tile_hdr_tracks_bla_iterations() {
    // Create orbit with enough iterations for BLA to kick in
    let c_ref = (BigFloat::with_precision(-0.5, 64), BigFloat::zero(64));
    let orbit = ReferenceOrbit::compute(&c_ref, 1000);

    // Small dc_max to enable BLA
    let dc_max = HDRFloat::from_f64(1e-10);
    let bla_table = BlaTable::compute(&orbit, &dc_max);

    let config = TileConfig {
        size: (2, 2),
        max_iterations: 1000,
        tau_sq: 1e-6,
        bla_enabled: true,
    };

    // Very small deltas so BLA validity checks pass
    let delta_origin = (HDRFloat::from_f64(1e-12), HDRFloat::from_f64(1e-12));
    let delta_step = (HDRFloat::from_f64(1e-14), HDRFloat::from_f64(1e-14));

    let result = render_tile_hdr(&orbit, Some(&bla_table), delta_origin, delta_step, &config);

    // Should have used some BLA iterations
    assert!(
        result.stats.total_iterations > 0,
        "Should have computed iterations"
    );
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p fractalwonder-compute render_tile_hdr -- --nocapture`
Expected: FAIL with "cannot find function `render_tile_hdr`"

**Step 3: Implement render_tile_hdr**

Add to `fractalwonder-compute/src/perturbation/tile.rs`:

```rust
use super::compute_pixel_perturbation_hdr_bla;
use crate::BlaTable;
use fractalwonder_core::{HDRComplex, HDRFloat};

/// Render a tile using HDRFloat precision with optional BLA acceleration.
///
/// This path handles arbitrary exponent ranges, necessary for deep zoom
/// where f64 would underflow. BLA acceleration is applied when available.
pub fn render_tile_hdr(
    orbit: &ReferenceOrbit,
    bla_table: Option<&BlaTable>,
    delta_origin: (HDRFloat, HDRFloat),
    delta_step: (HDRFloat, HDRFloat),
    config: &TileConfig,
) -> TileRenderResult {
    let capacity = (config.size.0 * config.size.1) as usize;
    let mut data = Vec::with_capacity(capacity);
    let mut stats = TileStats::default();

    let delta_origin_complex = HDRComplex {
        re: delta_origin.0,
        im: delta_origin.1,
    };
    let delta_step_complex = HDRComplex {
        re: delta_step.0,
        im: delta_step.1,
    };

    let mut delta_c_row = delta_origin_complex;

    for _py in 0..config.size.1 {
        let mut delta_c = delta_c_row;

        for _px in 0..config.size.0 {
            if config.bla_enabled {
                if let Some(bla) = bla_table {
                    let (result, pixel_stats) = compute_pixel_perturbation_hdr_bla(
                        orbit,
                        bla,
                        delta_c,
                        config.max_iterations,
                        config.tau_sq,
                    );
                    stats.bla_iterations += pixel_stats.bla_iterations as u64;
                    stats.total_iterations += pixel_stats.total_iterations as u64;
                    data.push(ComputeData::Mandelbrot(result));
                } else {
                    // BLA enabled but no table - fall back to generic path
                    let result = compute_pixel_perturbation(
                        orbit,
                        delta_c,
                        config.max_iterations,
                        config.tau_sq,
                    );
                    stats.total_iterations += result.iterations as u64;
                    data.push(ComputeData::Mandelbrot(result));
                }
            } else {
                let result = compute_pixel_perturbation(
                    orbit,
                    delta_c,
                    config.max_iterations,
                    config.tau_sq,
                );
                stats.total_iterations += result.iterations as u64;
                data.push(ComputeData::Mandelbrot(result));
            }

            delta_c.re = delta_c.re.add(&delta_step_complex.re);
        }

        delta_c_row.im = delta_c_row.im.add(&delta_step_complex.im);
    }

    TileRenderResult { data, stats }
}
```

**Step 4: Run tests to verify they pass**

Run: `cargo test -p fractalwonder-compute render_tile_hdr -- --nocapture`
Expected: 3 tests PASS

**Step 5: Run full test suite**

Run: `cargo test --workspace -- --nocapture`
Expected: All tests pass

**Step 6: Commit**

```bash
git add fractalwonder-compute/src/perturbation/tile.rs fractalwonder-compute/src/perturbation/tests/tile.rs
git commit -m "feat: add render_tile_hdr for HDRFloat precision with BLA"
```

---

## Task 5: Export Tile Functions from Library

**Files:**
- Modify: `fractalwonder-compute/src/perturbation/mod.rs`
- Modify: `fractalwonder-compute/src/lib.rs`

**Step 1: Export from perturbation mod.rs**

Update the exports in `fractalwonder-compute/src/perturbation/mod.rs`:

```rust
pub use tile::{render_tile_f64, render_tile_hdr, TileConfig, TileRenderResult, TileStats};
```

**Step 2: Export from lib.rs**

Add to the `pub use perturbation` line in `fractalwonder-compute/src/lib.rs`:

```rust
pub use perturbation::{
    compute_pixel_perturbation, compute_pixel_perturbation_hdr_bla, render_tile_f64,
    render_tile_hdr, BlaStats, ReferenceOrbit, TileConfig, TileRenderResult, TileStats,
};
```

**Step 3: Run clippy and tests**

Run: `cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
Expected: All pass

**Step 4: Commit**

```bash
git add fractalwonder-compute/src/perturbation/mod.rs fractalwonder-compute/src/lib.rs
git commit -m "refactor: export tile rendering functions from library"
```

---

## Task 6: Refactor Worker to Use Tile Functions

**Files:**
- Modify: `fractalwonder-compute/src/worker.rs`

**Step 1: Add imports**

Add to the imports at top of `worker.rs`:

```rust
use crate::{
    compute_pixel_perturbation, compute_pixel_perturbation_hdr_bla, render_tile_f64,
    render_tile_hdr, BlaTable, ReferenceOrbit, TileConfig,
};
```

**Step 2: Replace RenderTilePerturbation handler**

Replace the entire `MainToWorker::RenderTilePerturbation { .. }` match arm (lines 203-367) with:

```rust
        MainToWorker::RenderTilePerturbation {
            render_id,
            tile,
            orbit_id,
            delta_c_origin_json,
            delta_c_step_json,
            max_iterations,
            tau_sq,
            bigfloat_threshold_bits: _,
            bla_enabled,
            force_hdr_float,
        } => {
            // Parse BigFloat deltas from JSON
            let delta_c_origin: (BigFloat, BigFloat) =
                match serde_json::from_str(&delta_c_origin_json) {
                    Ok(d) => d,
                    Err(e) => {
                        post_message(&WorkerToMain::Error {
                            message: format!("Failed to parse delta_c_origin: {}", e),
                        });
                        return;
                    }
                };

            let delta_c_step: (BigFloat, BigFloat) =
                match serde_json::from_str(&delta_c_step_json) {
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

            let config = TileConfig {
                size: (tile.width, tile.height),
                max_iterations,
                tau_sq,
                bla_enabled,
            };

            // Dispatch based on delta magnitude
            let delta_log2 = delta_c_origin
                .0
                .log2_approx()
                .max(delta_c_origin.1.log2_approx());
            let use_f64 = !force_hdr_float && delta_log2 > -900.0 && delta_log2 < 900.0;

            let result = if use_f64 {
                let delta_origin = (delta_c_origin.0.to_f64(), delta_c_origin.1.to_f64());
                let delta_step = (delta_c_step.0.to_f64(), delta_c_step.1.to_f64());
                render_tile_f64(&orbit, delta_origin, delta_step, &config)
            } else {
                let delta_origin = (
                    HDRFloat::from_bigfloat(&delta_c_origin.0),
                    HDRFloat::from_bigfloat(&delta_c_origin.1),
                );
                let delta_step = (
                    HDRFloat::from_bigfloat(&delta_c_step.0),
                    HDRFloat::from_bigfloat(&delta_c_step.1),
                );
                render_tile_hdr(&orbit, cached.bla_table.as_ref(), delta_origin, delta_step, &config)
            };

            let compute_time_ms = Date::now() - start_time;

            post_message(&WorkerToMain::TileComplete {
                render_id,
                tile,
                data: result.data,
                compute_time_ms,
                bla_iterations: result.stats.bla_iterations,
                total_iterations: result.stats.total_iterations,
            });

            post_message(&WorkerToMain::RequestWork {
                render_id: Some(render_id),
            });
        }
```

**Step 3: Clean up unused imports**

Remove these imports from worker.rs if they become unused:
- `F64Complex` (now used inside tile.rs)
- `HDRComplex` (now used inside tile.rs)

Run clippy to identify any unused imports:

Run: `cargo clippy -p fractalwonder-compute --all-targets -- -D warnings`

**Step 4: Run full test suite**

Run: `cargo test --workspace -- --nocapture`
Expected: All tests pass

**Step 5: Verify WASM builds**

Run: `cargo check --target wasm32-unknown-unknown -p fractalwonder-compute`
Expected: No errors

**Step 6: Commit**

```bash
git add fractalwonder-compute/src/worker.rs
git commit -m "refactor: use render_tile_f64 and render_tile_hdr in worker"
```

---

## Task 7: Manual Integration Test

**Files:** None (manual testing)

**Step 1: Verify trunk serve is running**

Check that `trunk serve` is running on localhost:8080.

**Step 2: Test in browser**

1. Open http://localhost:8080
2. Navigate to a zoom level that uses f64 path (zoom < 10^270)
3. Verify tiles render correctly
4. Navigate to a deep zoom (zoom > 10^300) to test HDRFloat path
5. Verify tiles render correctly with BLA acceleration

**Step 3: Check console for errors**

Open browser dev tools and verify no JavaScript/WASM errors appear.

**Step 4: Final commit if any fixes needed**

If any fixes were needed, commit them.

---

## Summary

After completing all tasks:

| Metric | Before | After |
|--------|--------|-------|
| `RenderTilePerturbation` handler | ~165 lines | ~70 lines |
| Tile rendering testable? | No | Yes |
| Test coverage for tile logic | 0 tests | 5+ tests |
| Matches perturbation module pattern? | No | Yes |

### Files Created
- `fractalwonder-compute/src/perturbation/tile.rs`
- `fractalwonder-compute/src/perturbation/tests/tile.rs`

### Files Modified
- `fractalwonder-compute/src/perturbation/mod.rs`
- `fractalwonder-compute/src/perturbation/tests/mod.rs`
- `fractalwonder-compute/src/lib.rs`
- `fractalwonder-compute/src/worker.rs`
