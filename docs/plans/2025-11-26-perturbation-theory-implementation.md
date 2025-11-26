# Perturbation Theory Renderer - Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add `ParallelPerturbationRenderer` to enable extreme deep zoom (10^1000+) using perturbation theory.

**Architecture:** Single reference orbit computed at viewport center using BigFloat, broadcast to all workers as f64 arrays. Workers perform fast f64 delta iterations with per-pixel rebasing for glitch handling.

**Tech Stack:** Rust, wasm-bindgen, Leptos, Web Workers, serde_json

**Design Document:** `docs/plans/2025-11-26-perturbation-theory-design.md`

---

## Task 1: Add Perturbation Message Types to Core

**Files:**
- Modify: `fractalwonder-core/src/messages.rs`

**Step 1: Add new message variants**

Add these variants to `MainToWorker`:

```rust
/// Compute a reference orbit at high precision
ComputeReferenceOrbit {
    render_id: u32,
    orbit_id: u32,
    c_ref_json: String,
    max_iterations: u32,
},

/// Store a reference orbit for use in tile rendering
StoreReferenceOrbit {
    orbit_id: u32,
    c_ref: (f64, f64),
    orbit: Vec<(f64, f64)>,
    escaped_at: Option<u32>,
},

/// Render a tile using perturbation
RenderTilePerturbation {
    render_id: u32,
    tile: PixelRect,
    orbit_id: u32,
    delta_c_origin: (f64, f64),
    delta_c_step: (f64, f64),
    max_iterations: u32,
},

/// Discard a cached orbit
DiscardOrbit { orbit_id: u32 },
```

Add these variants to `WorkerToMain`:

```rust
/// Reference orbit computation complete
ReferenceOrbitComplete {
    render_id: u32,
    orbit_id: u32,
    c_ref: (f64, f64),
    orbit: Vec<(f64, f64)>,
    escaped_at: Option<u32>,
},

/// Orbit stored and ready
OrbitStored { orbit_id: u32 },
```

**Step 2: Run tests to verify serialization**

Run: `cargo test -p fractalwonder-core messages`
Expected: All existing tests pass

**Step 3: Add serialization tests for new messages**

Add to `messages.rs` tests:

```rust
#[test]
fn compute_reference_orbit_roundtrip() {
    let msg = MainToWorker::ComputeReferenceOrbit {
        render_id: 1,
        orbit_id: 42,
        c_ref_json: r#"{"x":"-0.5","y":"0.0"}"#.to_string(),
        max_iterations: 10000,
    };
    let json = serde_json::to_string(&msg).unwrap();
    let parsed: MainToWorker = serde_json::from_str(&json).unwrap();
    match parsed {
        MainToWorker::ComputeReferenceOrbit { orbit_id, max_iterations, .. } => {
            assert_eq!(orbit_id, 42);
            assert_eq!(max_iterations, 10000);
        }
        _ => panic!("Wrong variant"),
    }
}

#[test]
fn store_reference_orbit_roundtrip() {
    let msg = MainToWorker::StoreReferenceOrbit {
        orbit_id: 1,
        c_ref: (-0.5, 0.0),
        orbit: vec![(0.0, 0.0), (-0.5, 0.0), (-0.25, 0.0)],
        escaped_at: None,
    };
    let json = serde_json::to_string(&msg).unwrap();
    let parsed: MainToWorker = serde_json::from_str(&json).unwrap();
    match parsed {
        MainToWorker::StoreReferenceOrbit { orbit_id, orbit, .. } => {
            assert_eq!(orbit_id, 1);
            assert_eq!(orbit.len(), 3);
        }
        _ => panic!("Wrong variant"),
    }
}

#[test]
fn render_tile_perturbation_roundtrip() {
    let msg = MainToWorker::RenderTilePerturbation {
        render_id: 1,
        tile: PixelRect::new(0, 0, 64, 64),
        orbit_id: 42,
        delta_c_origin: (0.001, -0.002),
        delta_c_step: (0.0001, 0.0001),
        max_iterations: 10000,
    };
    let json = serde_json::to_string(&msg).unwrap();
    let parsed: MainToWorker = serde_json::from_str(&json).unwrap();
    match parsed {
        MainToWorker::RenderTilePerturbation { orbit_id, delta_c_origin, .. } => {
            assert_eq!(orbit_id, 42);
            assert!((delta_c_origin.0 - 0.001).abs() < 1e-10);
        }
        _ => panic!("Wrong variant"),
    }
}

#[test]
fn reference_orbit_complete_roundtrip() {
    let msg = WorkerToMain::ReferenceOrbitComplete {
        render_id: 1,
        orbit_id: 42,
        c_ref: (-0.5, 0.0),
        orbit: vec![(0.0, 0.0), (-0.5, 0.0)],
        escaped_at: Some(1000),
    };
    let json = serde_json::to_string(&msg).unwrap();
    let parsed: WorkerToMain = serde_json::from_str(&json).unwrap();
    match parsed {
        WorkerToMain::ReferenceOrbitComplete { orbit_id, escaped_at, .. } => {
            assert_eq!(orbit_id, 42);
            assert_eq!(escaped_at, Some(1000));
        }
        _ => panic!("Wrong variant"),
    }
}

#[test]
fn orbit_stored_roundtrip() {
    let msg = WorkerToMain::OrbitStored { orbit_id: 42 };
    let json = serde_json::to_string(&msg).unwrap();
    let parsed: WorkerToMain = serde_json::from_str(&json).unwrap();
    match parsed {
        WorkerToMain::OrbitStored { orbit_id } => assert_eq!(orbit_id, 42),
        _ => panic!("Wrong variant"),
    }
}
```

**Step 4: Run tests**

Run: `cargo test -p fractalwonder-core messages`
Expected: All tests pass

**Step 5: Commit**

```bash
git add fractalwonder-core/src/messages.rs
git commit -m "feat(core): add perturbation theory message types"
```

---

## Task 2: Add RendererType to Config

**Files:**
- Modify: `fractalwonder-ui/src/config.rs`

**Step 1: Add RendererType enum**

Add before `FractalConfig`:

```rust
/// Determines which renderer implementation to use
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum RendererType {
    /// Simple per-pixel BigFloat computation
    #[default]
    Simple,
    /// Perturbation theory with f64 delta iterations
    Perturbation,
}
```

**Step 2: Add field to FractalConfig**

```rust
pub struct FractalConfig {
    pub id: &'static str,
    pub display_name: &'static str,
    pub default_center: (&'static str, &'static str),
    pub default_width: &'static str,
    pub default_height: &'static str,
    pub renderer_type: RendererType,
}
```

**Step 3: Update FRACTAL_CONFIGS**

```rust
pub static FRACTAL_CONFIGS: &[FractalConfig] = &[
    FractalConfig {
        id: "test_image",
        display_name: "Test Pattern",
        default_center: ("0.0", "0.0"),
        default_width: "100.0",
        default_height: "100.0",
        renderer_type: RendererType::Simple,
    },
    FractalConfig {
        id: "mandelbrot",
        display_name: "Mandelbrot Set",
        default_center: ("-0.5", "0.0"),
        default_width: "4.0",
        default_height: "4.0",
        renderer_type: RendererType::Perturbation,
    },
];
```

**Step 4: Run tests**

Run: `cargo test -p fractalwonder-ui config`
Expected: All tests pass

**Step 5: Commit**

```bash
git add fractalwonder-ui/src/config.rs
git commit -m "feat(ui): add RendererType to FractalConfig"
```

---

## Task 3: Add Perturbation Max Iterations Formula

**Files:**
- Modify: `fractalwonder-core/src/transforms.rs`
- Modify: `fractalwonder-core/src/lib.rs`

**Step 1: Add the function to transforms.rs**

Add after `calculate_max_iterations`:

```rust
/// Calculate maximum iterations for perturbation rendering based on zoom exponent.
///
/// Uses empirical formula: 50 * zoom_exp^1.25
/// Clamped to [1000, 10_000_000] for practical bounds.
///
/// At deep zoom (10^1000), this returns ~280,000 iterations.
/// At extreme zoom (10^2000), this returns ~750,000 iterations.
pub fn calculate_max_iterations_perturbation(zoom_exponent: f64) -> u32 {
    let iterations = 50.0 * zoom_exponent.abs().max(1.0).powf(1.25);
    iterations.max(1000.0).min(10_000_000.0) as u32
}
```

**Step 2: Export from lib.rs**

Add to the `pub use transforms::` line:

```rust
pub use transforms::{
    // ... existing exports ...
    calculate_max_iterations_perturbation,
};
```

**Step 3: Add tests**

Add to `transforms.rs` tests section:

```rust
// ============================================================================
// calculate_max_iterations_perturbation tests
// ============================================================================

#[test]
fn perturbation_max_iterations_at_low_zoom() {
    // zoom_exp = 1 (10^1 zoom)
    let result = calculate_max_iterations_perturbation(1.0);
    // 50 * 1^1.25 = 50, clamped to 1000
    assert_eq!(result, 1000);
}

#[test]
fn perturbation_max_iterations_at_10x_zoom() {
    // zoom_exp = 10 (10^10 zoom)
    let result = calculate_max_iterations_perturbation(10.0);
    // 50 * 10^1.25 ≈ 890
    // Clamped to minimum 1000
    assert_eq!(result, 1000);
}

#[test]
fn perturbation_max_iterations_at_100x_zoom() {
    // zoom_exp = 100 (10^100 zoom)
    let result = calculate_max_iterations_perturbation(100.0);
    // 50 * 100^1.25 ≈ 15,811
    assert!(result > 15000 && result < 17000, "Expected ~15811, got {}", result);
}

#[test]
fn perturbation_max_iterations_at_1000x_zoom() {
    // zoom_exp = 1000 (10^1000 zoom)
    let result = calculate_max_iterations_perturbation(1000.0);
    // 50 * 1000^1.25 ≈ 280,884
    assert!(result > 250000 && result < 300000, "Expected ~280884, got {}", result);
}

#[test]
fn perturbation_max_iterations_at_2000x_zoom() {
    // zoom_exp = 2000 (10^2000 zoom)
    let result = calculate_max_iterations_perturbation(2000.0);
    // 50 * 2000^1.25 ≈ 750,000
    assert!(result > 700000 && result < 800000, "Expected ~750000, got {}", result);
}

#[test]
fn perturbation_max_iterations_capped_at_10m() {
    // Very extreme zoom should cap at 10 million
    let result = calculate_max_iterations_perturbation(100000.0);
    assert_eq!(result, 10_000_000);
}

#[test]
fn perturbation_max_iterations_floor_at_1000() {
    // Low/negative zoom should floor at 1000
    let result = calculate_max_iterations_perturbation(0.5);
    assert_eq!(result, 1000);
}
```

**Step 4: Run tests**

Run: `cargo test -p fractalwonder-core perturbation_max_iterations`
Expected: All tests pass

**Step 5: Commit**

```bash
git add fractalwonder-core/src/transforms.rs fractalwonder-core/src/lib.rs
git commit -m "feat(core): add perturbation max iterations formula"
```

---

## Task 4: Create Perturbation Module in Compute Crate

**Files:**
- Create: `fractalwonder-compute/src/perturbation.rs`
- Modify: `fractalwonder-compute/src/lib.rs`

**Step 1: Create perturbation.rs with ReferenceOrbit struct**

Create `fractalwonder-compute/src/perturbation.rs`:

```rust
//! Perturbation theory computation for deep Mandelbrot zoom.
//!
//! Computes reference orbits at high precision, then uses fast f64
//! delta iterations for individual pixels.

use fractalwonder_core::{BigFloat, MandelbrotData, Viewport};

/// A pre-computed reference orbit for perturbation rendering.
#[derive(Clone)]
pub struct ReferenceOrbit {
    /// Reference point C as f64 (for on-the-fly computation after escape/rebase)
    pub c_ref: (f64, f64),
    /// Pre-computed orbit values X_n as f64
    pub orbit: Vec<(f64, f64)>,
    /// Iteration at which reference escaped (None if never escaped)
    pub escaped_at: Option<u32>,
}

impl ReferenceOrbit {
    /// Compute a reference orbit using BigFloat precision.
    ///
    /// The orbit is computed at full precision but stored as f64
    /// since orbit values are bounded by escape radius (~2).
    pub fn compute(c_ref: &(BigFloat, BigFloat), max_iterations: u32) -> Self {
        let precision = c_ref.0.precision_bits();
        let mut orbit = Vec::with_capacity(max_iterations as usize);

        let mut x = BigFloat::zero(precision);
        let mut y = BigFloat::zero(precision);
        let four = BigFloat::with_precision(4.0, precision);

        let mut escaped_at = None;

        for n in 0..max_iterations {
            // Store current X_n as f64
            orbit.push((x.to_f64(), y.to_f64()));

            // Check escape: |z|^2 > 4
            let x_sq = x.mul(&x);
            let y_sq = y.mul(&y);
            if x_sq.add(&y_sq).gt(&four) {
                escaped_at = Some(n);
                break;
            }

            // z = z^2 + c
            let two = BigFloat::with_precision(2.0, precision);
            let new_x = x_sq.sub(&y_sq).add(&c_ref.0);
            let new_y = two.mul(&x).mul(&y).add(&c_ref.1);
            x = new_x;
            y = new_y;
        }

        Self {
            c_ref: (c_ref.0.to_f64(), c_ref.1.to_f64()),
            orbit,
            escaped_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reference_orbit_in_set_never_escapes() {
        // Point (-0.5, 0) is in the main cardioid
        let c_ref = (
            BigFloat::with_precision(-0.5, 128),
            BigFloat::zero(128),
        );
        let orbit = ReferenceOrbit::compute(&c_ref, 1000);

        assert_eq!(orbit.escaped_at, None);
        assert_eq!(orbit.orbit.len(), 1000);
        assert!((orbit.c_ref.0 - (-0.5)).abs() < 1e-10);
        assert!((orbit.c_ref.1 - 0.0).abs() < 1e-10);
    }

    #[test]
    fn reference_orbit_outside_set_escapes() {
        // Point (2, 0) escapes quickly
        let c_ref = (
            BigFloat::with_precision(2.0, 128),
            BigFloat::zero(128),
        );
        let orbit = ReferenceOrbit::compute(&c_ref, 1000);

        assert!(orbit.escaped_at.is_some());
        assert!(orbit.escaped_at.unwrap() < 10);
    }

    #[test]
    fn reference_orbit_values_bounded() {
        // All orbit values should be bounded by escape radius
        let c_ref = (
            BigFloat::with_precision(-0.5, 128),
            BigFloat::zero(128),
        );
        let orbit = ReferenceOrbit::compute(&c_ref, 100);

        for (x, y) in &orbit.orbit {
            let mag_sq = x * x + y * y;
            assert!(mag_sq <= 4.0, "Orbit value escaped: ({}, {})", x, y);
        }
    }
}
```

**Step 2: Add module to lib.rs**

Add to `fractalwonder-compute/src/lib.rs`:

```rust
mod perturbation;

pub use perturbation::ReferenceOrbit;
```

**Step 3: Run tests**

Run: `cargo test -p fractalwonder-compute perturbation`
Expected: All tests pass

**Step 4: Commit**

```bash
git add fractalwonder-compute/src/perturbation.rs fractalwonder-compute/src/lib.rs
git commit -m "feat(compute): add ReferenceOrbit computation"
```

---

## Task 5: Add Delta Iteration Algorithm

**Files:**
- Modify: `fractalwonder-compute/src/perturbation.rs`

**Step 1: Add compute_pixel_perturbation function**

Add to `perturbation.rs`:

```rust
/// Compute a single pixel using perturbation from a reference orbit.
///
/// Uses f64 delta iterations with automatic rebasing when delta grows too large.
/// Falls back to on-the-fly computation when reference orbit escapes or after rebasing.
pub fn compute_pixel_perturbation(
    orbit: &ReferenceOrbit,
    delta_c: (f64, f64),
    max_iterations: u32,
) -> MandelbrotData {
    let mut dx = 0.0;
    let mut dy = 0.0;

    // For on-the-fly mode after rebasing or reference escape
    let mut x = 0.0;
    let mut y = 0.0;
    let mut on_the_fly = false;

    let orbit_len = orbit.orbit.len() as u32;
    let reference_escaped = orbit.escaped_at.unwrap_or(u32::MAX);

    for n in 0..max_iterations {
        // Get X_n from orbit or compute on-the-fly
        let (xn, yn) = if !on_the_fly && n < orbit_len && n < reference_escaped {
            orbit.orbit[n as usize]
        } else {
            if !on_the_fly {
                // Switching to on-the-fly mode
                on_the_fly = true;
                // Initialize x, y from last known Z = X + delta
                if n > 0 && n <= orbit_len {
                    let prev_n = (n - 1) as usize;
                    if prev_n < orbit.orbit.len() {
                        x = orbit.orbit[prev_n].0 + dx;
                        y = orbit.orbit[prev_n].1 + dy;
                    }
                }
                dx = 0.0;
                dy = 0.0;
            }
            // Compute next X on-the-fly
            let new_x = x * x - y * y + orbit.c_ref.0;
            let new_y = 2.0 * x * y + orbit.c_ref.1;
            x = new_x;
            y = new_y;
            (x, y)
        };

        // Escape check: |X_n + delta_n|^2 > 4
        let zx = xn + dx;
        let zy = yn + dy;
        let mag_sq = zx * zx + zy * zy;

        if mag_sq > 4.0 {
            return MandelbrotData {
                iterations: n,
                max_iterations,
                escaped: true,
            };
        }

        // Rebase check: |delta|^2 > 0.25 * |X|^2 (threshold 0.5)
        if !on_the_fly {
            let delta_mag_sq = dx * dx + dy * dy;
            let x_mag_sq = xn * xn + yn * yn;

            if delta_mag_sq > 0.25 * x_mag_sq && x_mag_sq > 1e-20 {
                // Rebase: switch to on-the-fly with Z as new reference
                x = zx;
                y = zy;
                dx = 0.0;
                dy = 0.0;
                on_the_fly = true;
                continue;
            }
        }

        // Delta iteration: delta_{n+1} = 2*X_n*delta_n + delta_n^2 + delta_c
        let new_dx = 2.0 * (xn * dx - yn * dy) + dx * dx - dy * dy + delta_c.0;
        let new_dy = 2.0 * (xn * dy + yn * dx) + 2.0 * dx * dy + delta_c.1;
        dx = new_dx;
        dy = new_dy;
    }

    MandelbrotData {
        iterations: max_iterations,
        max_iterations,
        escaped: false,
    }
}
```

**Step 2: Add tests for delta iteration**

Add to tests module in `perturbation.rs`:

```rust
#[test]
fn perturbation_origin_in_set() {
    // Reference at (-0.5, 0), delta_c = (0.5, 0) gives point (0, 0) which is in set
    let c_ref = (
        BigFloat::with_precision(-0.5, 128),
        BigFloat::zero(128),
    );
    let orbit = ReferenceOrbit::compute(&c_ref, 1000);

    let result = compute_pixel_perturbation(&orbit, (0.5, 0.0), 1000);

    assert!(!result.escaped);
    assert_eq!(result.iterations, 1000);
}

#[test]
fn perturbation_far_point_escapes() {
    // Reference at (-0.5, 0), delta_c = (2.5, 0) gives point (2, 0) which escapes
    let c_ref = (
        BigFloat::with_precision(-0.5, 128),
        BigFloat::zero(128),
    );
    let orbit = ReferenceOrbit::compute(&c_ref, 1000);

    let result = compute_pixel_perturbation(&orbit, (2.5, 0.0), 1000);

    assert!(result.escaped);
    assert!(result.iterations < 10);
}

#[test]
fn perturbation_matches_direct_for_nearby_point() {
    // Compare perturbation result with direct BigFloat computation
    let c_ref = (
        BigFloat::with_precision(-0.5, 128),
        BigFloat::zero(128),
    );
    let orbit = ReferenceOrbit::compute(&c_ref, 500);

    // Small delta: pixel at (-0.49, 0.01)
    let delta_c = (0.01, 0.01);
    let perturbation_result = compute_pixel_perturbation(&orbit, delta_c, 500);

    // Direct computation at same point
    let pixel_c = (
        BigFloat::with_precision(-0.49, 128),
        BigFloat::with_precision(0.01, 128),
    );
    let direct_result = compute_direct(&pixel_c, 500);

    // Results should match (both escaped or both didn't, similar iteration count)
    assert_eq!(perturbation_result.escaped, direct_result.escaped);
    if perturbation_result.escaped {
        // Allow small difference due to floating point
        let diff = (perturbation_result.iterations as i32 - direct_result.iterations as i32).abs();
        assert!(diff <= 1, "Iteration difference too large: {}", diff);
    }
}

// Helper for direct computation comparison
fn compute_direct(c: &(BigFloat, BigFloat), max_iter: u32) -> MandelbrotData {
    let precision = c.0.precision_bits();
    let mut x = BigFloat::zero(precision);
    let mut y = BigFloat::zero(precision);
    let four = BigFloat::with_precision(4.0, precision);

    for n in 0..max_iter {
        let x_sq = x.mul(&x);
        let y_sq = y.mul(&y);
        if x_sq.add(&y_sq).gt(&four) {
            return MandelbrotData { iterations: n, max_iterations: max_iter, escaped: true };
        }
        let two = BigFloat::with_precision(2.0, precision);
        let new_x = x_sq.sub(&y_sq).add(&c.0);
        let new_y = two.mul(&x).mul(&y).add(&c.1);
        x = new_x;
        y = new_y;
    }
    MandelbrotData { iterations: max_iter, max_iterations: max_iter, escaped: false }
}

#[test]
fn perturbation_handles_rebasing() {
    // Use a reference point where rebasing will be triggered
    // Point on boundary has chaotic behavior
    let c_ref = (
        BigFloat::with_precision(-0.75, 128),
        BigFloat::with_precision(0.1, 128),
    );
    let orbit = ReferenceOrbit::compute(&c_ref, 500);

    // Offset that should trigger rebasing
    let delta_c = (0.1, 0.05);
    let result = compute_pixel_perturbation(&orbit, delta_c, 500);

    // Should complete without panic
    assert!(result.iterations > 0);
}
```

**Step 3: Export function**

Update `fractalwonder-compute/src/lib.rs`:

```rust
pub use perturbation::{compute_pixel_perturbation, ReferenceOrbit};
```

**Step 4: Run tests**

Run: `cargo test -p fractalwonder-compute perturbation`
Expected: All tests pass

**Step 5: Commit**

```bash
git add fractalwonder-compute/src/perturbation.rs fractalwonder-compute/src/lib.rs
git commit -m "feat(compute): add perturbation delta iteration algorithm"
```

---

## Task 6: Add Worker Orbit Cache

**Files:**
- Modify: `fractalwonder-compute/src/worker.rs`

**Step 1: Add CachedOrbit struct and state**

Add at top of `worker.rs` after imports:

```rust
use std::collections::HashMap;
use crate::perturbation::{compute_pixel_perturbation, ReferenceOrbit};

/// Cached reference orbit for perturbation rendering
struct CachedOrbit {
    c_ref: (f64, f64),
    orbit: Vec<(f64, f64)>,
    escaped_at: Option<u32>,
}

impl CachedOrbit {
    fn to_reference_orbit(&self) -> ReferenceOrbit {
        ReferenceOrbit {
            c_ref: self.c_ref,
            orbit: self.orbit.clone(),
            escaped_at: self.escaped_at,
        }
    }
}
```

**Step 2: Add orbit cache to worker state**

Change the worker state from just `BoxedRenderer` to a struct. Replace:

```rust
let renderer: Rc<RefCell<Option<BoxedRenderer>>> = Rc::new(RefCell::new(None));
```

With a `WorkerState` struct that holds both the renderer and orbit cache:

```rust
struct WorkerState {
    renderer: Option<BoxedRenderer>,
    orbit_cache: HashMap<u32, CachedOrbit>,
}

impl WorkerState {
    fn new() -> Self {
        Self {
            renderer: None,
            orbit_cache: HashMap::new(),
        }
    }
}
```

**Step 3: Update handle_message signature**

Change `handle_message` to take `&mut WorkerState`:

```rust
fn handle_message(state: &mut WorkerState, data: JsValue) {
    // ... existing code, replace renderer.borrow() with state.renderer ...
}
```

**Step 4: Add StoreReferenceOrbit handler**

Add new match arm in `handle_message`:

```rust
MainToWorker::StoreReferenceOrbit {
    orbit_id,
    c_ref,
    orbit,
    escaped_at,
} => {
    state.orbit_cache.insert(orbit_id, CachedOrbit {
        c_ref,
        orbit,
        escaped_at,
    });
    post_message(&WorkerToMain::OrbitStored { orbit_id });
}

MainToWorker::DiscardOrbit { orbit_id } => {
    state.orbit_cache.remove(&orbit_id);
}
```

**Step 5: Update init_message_worker**

```rust
#[wasm_bindgen]
pub fn init_message_worker() {
    console_error_panic_hook::set_once();
    web_sys::console::log_1(&"[Worker] Started".into());

    let state = Rc::new(RefCell::new(WorkerState::new()));

    let state_clone = Rc::clone(&state);
    let onmessage = Closure::wrap(Box::new(move |e: web_sys::MessageEvent| {
        handle_message(&mut state_clone.borrow_mut(), e.data());
    }) as Box<dyn FnMut(_)>);

    let global: web_sys::DedicatedWorkerGlobalScope =
        js_sys::global().dyn_into().expect("Not in worker context");

    global.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
    onmessage.forget();

    post_message(&WorkerToMain::Ready);
}
```

**Step 6: Run build to check compilation**

Run: `cargo build -p fractalwonder-compute`
Expected: Compiles without errors

**Step 7: Commit**

```bash
git add fractalwonder-compute/src/worker.rs
git commit -m "feat(compute): add orbit cache to worker state"
```

---

## Task 7: Handle ComputeReferenceOrbit Message

**Files:**
- Modify: `fractalwonder-compute/src/worker.rs`

**Step 1: Add ComputeReferenceOrbit handler**

Add match arm in `handle_message`:

```rust
MainToWorker::ComputeReferenceOrbit {
    render_id,
    orbit_id,
    c_ref_json,
    max_iterations,
} => {
    // Parse c_ref from JSON (BigFloat coordinates)
    let c_ref: (BigFloat, BigFloat) = match serde_json::from_str(&c_ref_json) {
        Ok(c) => c,
        Err(e) => {
            post_message(&WorkerToMain::Error {
                message: format!("Failed to parse c_ref: {}", e),
            });
            return;
        }
    };

    let start_time = Date::now();

    // Compute reference orbit
    let orbit = ReferenceOrbit::compute(&c_ref, max_iterations);

    let compute_time = Date::now() - start_time;
    web_sys::console::log_1(
        &format!(
            "[Worker] Reference orbit computed: {} iterations in {:.0}ms, escaped_at={:?}",
            orbit.orbit.len(),
            compute_time,
            orbit.escaped_at
        ).into()
    );

    // Send result back
    post_message(&WorkerToMain::ReferenceOrbitComplete {
        render_id,
        orbit_id,
        c_ref: orbit.c_ref,
        orbit: orbit.orbit,
        escaped_at: orbit.escaped_at,
    });
}
```

**Step 2: Add import for BigFloat**

Add to imports at top:

```rust
use fractalwonder_core::BigFloat;
```

**Step 3: Run build**

Run: `cargo build -p fractalwonder-compute`
Expected: Compiles without errors

**Step 4: Commit**

```bash
git add fractalwonder-compute/src/worker.rs
git commit -m "feat(compute): handle ComputeReferenceOrbit message"
```

---

## Task 8: Handle RenderTilePerturbation Message

**Files:**
- Modify: `fractalwonder-compute/src/worker.rs`

**Step 1: Add RenderTilePerturbation handler**

Add match arm in `handle_message`:

```rust
MainToWorker::RenderTilePerturbation {
    render_id,
    tile,
    orbit_id,
    delta_c_origin,
    delta_c_step,
    max_iterations,
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
            let result = compute_pixel_perturbation(&orbit, delta_c, max_iterations);
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

**Step 2: Add import for ComputeData**

Ensure `ComputeData` is imported:

```rust
use fractalwonder_core::{BigFloat, ComputeData, MainToWorker, Viewport, WorkerToMain};
```

**Step 3: Run build**

Run: `cargo build -p fractalwonder-compute`
Expected: Compiles without errors

**Step 4: Commit**

```bash
git add fractalwonder-compute/src/worker.rs
git commit -m "feat(compute): handle RenderTilePerturbation message"
```

---

## Task 9: Create ParallelPerturbationRenderer

**Files:**
- Create: `fractalwonder-ui/src/rendering/parallel_perturbation_renderer.rs`
- Modify: `fractalwonder-ui/src/rendering/mod.rs`

**Step 1: Create the new renderer file**

Create `fractalwonder-ui/src/rendering/parallel_perturbation_renderer.rs`:

```rust
//! Parallel renderer using perturbation theory for deep Mandelbrot zoom.

use crate::config::FractalConfig;
use crate::rendering::canvas_utils::{draw_pixels_to_canvas, get_2d_context};
use crate::rendering::colorizers::colorize;
use crate::rendering::tiles::{calculate_tile_size, generate_tiles};
use crate::rendering::RenderProgress;
use crate::workers::{TileResult, WorkerPool};
use fractalwonder_core::{
    calculate_max_iterations_perturbation, BigFloat, MainToWorker, PixelRect, Viewport,
    WorkerToMain,
};
use leptos::*;
use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;
use wasm_bindgen::JsValue;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement};

/// Render phase for perturbation renderer
#[derive(Clone, Copy, Debug, PartialEq)]
enum RenderPhase {
    /// Computing reference orbit
    ComputingOrbit,
    /// Broadcasting orbit to workers
    BroadcastingOrbit,
    /// Rendering tiles
    RenderingTiles,
    /// Idle
    Idle,
}

/// Parallel renderer using perturbation theory.
pub struct ParallelPerturbationRenderer {
    config: &'static FractalConfig,
    worker_pool: Rc<RefCell<WorkerPool>>,
    progress: RwSignal<RenderProgress>,
    canvas_ctx: Rc<RefCell<Option<CanvasRenderingContext2d>>>,
    phase: Rc<RefCell<RenderPhase>>,
    current_orbit_id: Rc<RefCell<u32>>,
    workers_with_orbit: Rc<RefCell<HashSet<usize>>>,
    pending_viewport: Rc<RefCell<Option<Viewport>>>,
    pending_canvas_size: Rc<RefCell<(u32, u32)>>,
}

impl ParallelPerturbationRenderer {
    pub fn new(config: &'static FractalConfig) -> Result<Self, JsValue> {
        let progress = create_rw_signal(RenderProgress::default());
        let canvas_ctx: Rc<RefCell<Option<CanvasRenderingContext2d>>> = Rc::new(RefCell::new(None));
        let phase = Rc::new(RefCell::new(RenderPhase::Idle));
        let current_orbit_id = Rc::new(RefCell::new(0u32));
        let workers_with_orbit: Rc<RefCell<HashSet<usize>>> = Rc::new(RefCell::new(HashSet::new()));
        let pending_viewport = Rc::new(RefCell::new(None));
        let pending_canvas_size = Rc::new(RefCell::new((0, 0)));

        // Clone Rcs for callbacks
        let ctx_clone = Rc::clone(&canvas_ctx);
        let phase_clone = Rc::clone(&phase);
        let orbit_id_clone = Rc::clone(&current_orbit_id);
        let workers_orbit_clone = Rc::clone(&workers_with_orbit);
        let pending_vp_clone = Rc::clone(&pending_viewport);
        let pending_size_clone = Rc::clone(&pending_canvas_size);
        let progress_signal = progress;

        let on_tile_complete = move |result: TileResult| {
            if let Some(ctx) = ctx_clone.borrow().as_ref() {
                let pixels: Vec<u8> = result.data.iter().flat_map(colorize).collect();
                let _ = draw_pixels_to_canvas(
                    ctx,
                    &pixels,
                    result.tile.width,
                    result.tile.x as f64,
                    result.tile.y as f64,
                );
            }
        };

        let worker_pool = WorkerPool::new(config.id, on_tile_complete, progress)?;

        Ok(Self {
            config,
            worker_pool,
            progress,
            canvas_ctx,
            phase,
            current_orbit_id,
            workers_with_orbit,
            pending_viewport,
            pending_canvas_size,
        })
    }

    pub fn progress(&self) -> RwSignal<RenderProgress> {
        self.progress
    }

    pub fn cancel(&self) {
        *self.phase.borrow_mut() = RenderPhase::Idle;
        self.worker_pool.borrow_mut().cancel();
    }

    pub fn render(&self, viewport: &Viewport, canvas: &HtmlCanvasElement) {
        let width = canvas.width();
        let height = canvas.height();

        if width == 0 || height == 0 {
            return;
        }

        // Store canvas context
        if let Ok(ctx) = get_2d_context(canvas) {
            *self.canvas_ctx.borrow_mut() = Some(ctx);
        }

        // Store pending render info
        *self.pending_viewport.borrow_mut() = Some(viewport.clone());
        *self.pending_canvas_size.borrow_mut() = (width, height);

        // Increment orbit ID for new render
        *self.current_orbit_id.borrow_mut() += 1;
        let orbit_id = *self.current_orbit_id.borrow();

        // Clear workers that have the orbit
        self.workers_with_orbit.borrow_mut().clear();

        // Calculate max iterations based on zoom
        let reference_width = self
            .config
            .default_viewport(viewport.precision_bits())
            .width;
        let log2_zoom = reference_width.log2_approx() - viewport.width.log2_approx();
        let zoom_exponent = log2_zoom / std::f64::consts::LOG2_10;
        let max_iterations = calculate_max_iterations_perturbation(zoom_exponent);

        web_sys::console::log_1(
            &format!(
                "[PerturbationRenderer] Starting render: zoom_exp={:.1}, max_iter={}",
                zoom_exponent, max_iterations
            )
            .into(),
        );

        // Phase 1: Compute reference orbit at viewport center
        *self.phase.borrow_mut() = RenderPhase::ComputingOrbit;

        // Serialize c_ref as JSON to preserve BigFloat precision
        let c_ref = (&viewport.center.0, &viewport.center.1);
        let c_ref_json = serde_json::to_string(&c_ref).unwrap_or_default();

        // Send to first available worker
        // TODO: This needs proper integration with WorkerPool
        // For now, we'll handle this through the existing message system

        // Reset progress
        self.progress.set(RenderProgress::default());
    }

    pub fn switch_config(&mut self, config: &'static FractalConfig) -> Result<(), JsValue> {
        self.config = config;
        self.worker_pool.borrow_mut().switch_renderer(config.id);
        Ok(())
    }
}
```

**Step 2: Add to mod.rs**

Add to `fractalwonder-ui/src/rendering/mod.rs`:

```rust
mod parallel_perturbation_renderer;

pub use parallel_perturbation_renderer::ParallelPerturbationRenderer;
```

**Step 3: Run build**

Run: `cargo build -p fractalwonder-ui`
Expected: Compiles (with warnings about incomplete implementation)

**Step 4: Commit**

```bash
git add fractalwonder-ui/src/rendering/parallel_perturbation_renderer.rs
git add fractalwonder-ui/src/rendering/mod.rs
git commit -m "feat(ui): add ParallelPerturbationRenderer skeleton"
```

---

## Task 10: Integrate Renderer Selection in App

**Files:**
- Modify: `fractalwonder-ui/src/app.rs`

**Step 1: Import new types**

Add imports:

```rust
use crate::config::RendererType;
use crate::rendering::ParallelPerturbationRenderer;
```

**Step 2: Update renderer creation logic**

Find where `ParallelRenderer` is created and add conditional:

```rust
// Create renderer based on config type
let renderer = match config.renderer_type {
    RendererType::Simple => {
        // Existing ParallelRenderer
    }
    RendererType::Perturbation => {
        ParallelPerturbationRenderer::new(config)?
    }
};
```

Note: The exact integration depends on the current app.rs structure. The renderer needs to be stored in a way that allows calling render/cancel on it.

**Step 3: Run build**

Run: `cargo build -p fractalwonder-ui`
Expected: Compiles

**Step 4: Test in browser**

Run: Open `http://localhost:8080` with Mandelbrot selected
Expected: App loads without errors (rendering may not work yet)

**Step 5: Commit**

```bash
git add fractalwonder-ui/src/app.rs
git commit -m "feat(ui): integrate renderer selection based on config type"
```

---

## Task 11: Complete WorkerPool Integration for Perturbation

This task completes the integration by:
1. Adding perturbation-specific message handling to WorkerPool
2. Implementing the full render flow (orbit compute → broadcast → tiles)

**Files:**
- Modify: `fractalwonder-ui/src/workers/worker_pool.rs`
- Modify: `fractalwonder-ui/src/rendering/parallel_perturbation_renderer.rs`

This is a larger task that may need to be broken down further during implementation based on the current WorkerPool structure.

**Key changes needed:**
1. WorkerPool needs to handle `ReferenceOrbitComplete` and `OrbitStored` messages
2. ParallelPerturbationRenderer needs to coordinate the three-phase render
3. Delta calculation for each tile needs to be implemented

**Step 1: Review current WorkerPool implementation**

Read and understand the message handling flow in `worker_pool.rs`

**Step 2: Add perturbation message handlers**

Add handlers for:
- `WorkerToMain::ReferenceOrbitComplete`
- `WorkerToMain::OrbitStored`

**Step 3: Implement delta calculation**

For each tile, calculate:
```rust
let tile_fractal_origin = pixel_to_fractal(tile.x, tile.y, viewport, canvas_size);
let delta_c_origin = (
    tile_fractal_origin.0.to_f64() - c_ref.0,
    tile_fractal_origin.1.to_f64() - c_ref.1,
);
let delta_c_step = (
    viewport.width.to_f64() / canvas_size.0 as f64,
    viewport.height.to_f64() / canvas_size.1 as f64,
);
```

**Step 4: Complete three-phase render flow**

1. Send `ComputeReferenceOrbit` to one worker
2. On `ReferenceOrbitComplete`, broadcast `StoreReferenceOrbit` to all workers
3. On all `OrbitStored`, send `RenderTilePerturbation` for each tile

**Step 5: Test full flow**

Run: Open app, select Mandelbrot, verify rendering works
Expected: See Mandelbrot set rendered

**Step 6: Commit**

```bash
git add fractalwonder-ui/src/workers/worker_pool.rs
git add fractalwonder-ui/src/rendering/parallel_perturbation_renderer.rs
git commit -m "feat(ui): complete perturbation renderer integration"
```

---

## Task 12: Add Browser Tests

**Files:**
- Create/modify test files as appropriate

**Step 1: Test at moderate zoom**

Verify perturbation renders correctly at 10^10 zoom

**Step 2: Test at deep zoom**

Verify perturbation renders at 10^100 zoom without hanging

**Step 3: Verify visual correctness**

Compare output at same location with Simple renderer (at accessible zoom levels)

**Step 4: Performance comparison**

Measure render time: Perturbation vs Simple at 10^50 zoom

---

## Summary

| Task | Description | Files |
|------|-------------|-------|
| 1 | Add perturbation message types | messages.rs |
| 2 | Add RendererType to config | config.rs |
| 3 | Add perturbation max iterations formula | transforms.rs |
| 4 | Create perturbation module with ReferenceOrbit | perturbation.rs |
| 5 | Add delta iteration algorithm | perturbation.rs |
| 6 | Add worker orbit cache | worker.rs |
| 7 | Handle ComputeReferenceOrbit | worker.rs |
| 8 | Handle RenderTilePerturbation | worker.rs |
| 9 | Create ParallelPerturbationRenderer | parallel_perturbation_renderer.rs |
| 10 | Integrate renderer selection | app.rs |
| 11 | Complete WorkerPool integration | worker_pool.rs |
| 12 | Browser tests | tests |
