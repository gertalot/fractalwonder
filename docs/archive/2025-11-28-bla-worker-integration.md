# BLA Worker Integration Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Integrate BLA acceleration into the rendering pipeline so perturbation renders use iteration skipping.

**Architecture:**
- Worker builds `BlaTable` when storing a reference orbit (requires `dc_max` parameter)
- `dc_max` is computed from viewport dimensions and sent with orbit data
- Worker uses `compute_pixel_perturbation_floatexp_bla` instead of `compute_pixel_perturbation_floatexp`

**Tech Stack:** Rust, WASM workers, message passing

---

## Task 1: Add dc_max to StoreReferenceOrbit Message

**Files:**
- Modify: `fractalwonder-core/src/messages.rs:32-38`
- Test: `fractalwonder-core/src/messages.rs` (existing roundtrip tests)

**Step 1: Write failing test**

Add to `fractalwonder-core/src/messages.rs` tests:

```rust
#[test]
fn store_reference_orbit_with_dc_max_roundtrip() {
    let msg = MainToWorker::StoreReferenceOrbit {
        orbit_id: 1,
        c_ref: (-0.5, 0.0),
        orbit: vec![(0.0, 0.0), (-0.5, 0.0)],
        escaped_at: None,
        dc_max: 0.001,
    };
    let json = serde_json::to_string(&msg).unwrap();
    let parsed: MainToWorker = serde_json::from_str(&json).unwrap();
    match parsed {
        MainToWorker::StoreReferenceOrbit { dc_max, .. } => {
            assert!((dc_max - 0.001).abs() < 1e-12);
        }
        _ => panic!("Wrong variant"),
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p fractalwonder-core store_reference_orbit_with_dc_max_roundtrip`
Expected: FAIL - `dc_max` field doesn't exist

**Step 3: Add dc_max field to StoreReferenceOrbit**

In `fractalwonder-core/src/messages.rs`, modify `StoreReferenceOrbit`:

```rust
/// Store a reference orbit for use in tile rendering.
StoreReferenceOrbit {
    orbit_id: u32,
    c_ref: (f64, f64),
    orbit: Vec<(f64, f64)>,
    escaped_at: Option<u32>,
    /// Maximum |δc| for any pixel in viewport (for BLA table construction)
    dc_max: f64,
},
```

**Step 4: Update existing test to include dc_max**

In the existing `store_reference_orbit_roundtrip` test, add `dc_max: 0.01`:

```rust
#[test]
fn store_reference_orbit_roundtrip() {
    let msg = MainToWorker::StoreReferenceOrbit {
        orbit_id: 1,
        c_ref: (-0.5, 0.0),
        orbit: vec![(0.0, 0.0), (-0.5, 0.0), (-0.25, 0.0)],
        escaped_at: None,
        dc_max: 0.01,
    };
    // ... rest unchanged
}
```

**Step 5: Run tests to verify they pass**

Run: `cargo test -p fractalwonder-core`
Expected: PASS

**Step 6: Commit**

```bash
git add fractalwonder-core/src/messages.rs
git commit -m "$(cat <<'EOF'
feat(messages): add dc_max field to StoreReferenceOrbit

Required for BLA table construction - workers need the maximum delta_c
magnitude to compute validity radii when merging BLA entries.

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude <noreply@anthropic.com>
EOF
)"
```

---

## Task 2: Compute dc_max in WorkerPool

**Files:**
- Modify: `fractalwonder-ui/src/workers/worker_pool.rs:380-401`

**Step 1: Add dc_max calculation helper**

Add this function near the top of `worker_pool.rs` (after `performance_now`):

```rust
/// Calculate maximum |δc| for any pixel in the viewport.
/// This is the distance from viewport center to the farthest corner.
fn calculate_dc_max(viewport: &Viewport, canvas_size: (u32, u32)) -> f64 {
    // Half-width and half-height in fractal coordinates
    let half_width = viewport.width.to_f64() / 2.0;
    let half_height = viewport.height.to_f64() / 2.0;

    // Euclidean distance to corner
    (half_width * half_width + half_height * half_height).sqrt()
}
```

**Step 2: Store dc_max in PerturbationState**

Add field to `PerturbationState` struct:

```rust
struct PerturbationState {
    // ... existing fields ...
    /// Maximum |δc| for BLA table construction
    dc_max: f64,
}
```

And update `Default` impl:

```rust
impl Default for PerturbationState {
    fn default() -> Self {
        Self {
            // ... existing fields ...
            dc_max: 0.0,
        }
    }
}
```

**Step 3: Calculate and store dc_max in start_perturbation_render**

In `start_perturbation_render`, after computing `delta_step`, add:

```rust
// Calculate dc_max for BLA table construction
self.perturbation.dc_max = calculate_dc_max(&viewport, canvas_size);
```

**Step 4: Pass dc_max when broadcasting orbit**

In `handle_message`, `ReferenceOrbitComplete` handler (~line 391-401), update the broadcast:

```rust
// Broadcast to all workers
for worker_id in 0..self.workers.len() {
    self.send_to_worker(
        worker_id,
        &MainToWorker::StoreReferenceOrbit {
            orbit_id,
            c_ref,
            orbit: orbit.clone(),
            escaped_at,
            dc_max: self.perturbation.dc_max,
        },
    );
}
```

**Step 5: Build and verify compilation**

Run: `cargo check -p fractalwonder-ui`
Expected: Compilation errors in worker.rs (needs dc_max handling)

**Step 6: Commit (partial - UI side done)**

```bash
git add fractalwonder-ui/src/workers/worker_pool.rs
git commit -m "$(cat <<'EOF'
feat(worker-pool): calculate and pass dc_max for BLA

Computes maximum |δc| from viewport dimensions (distance to corner)
and includes it in StoreReferenceOrbit messages for BLA table construction.

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude <noreply@anthropic.com>
EOF
)"
```

---

## Task 3: Build BlaTable in Worker

**Files:**
- Modify: `fractalwonder-compute/src/worker.rs:18-32` (CachedOrbit struct)
- Modify: `fractalwonder-compute/src/worker.rs:256-271` (StoreReferenceOrbit handler)

**Step 1: Add BlaTable to CachedOrbit**

Update imports at top of `worker.rs`:

```rust
use crate::{
    compute_pixel_perturbation, compute_pixel_perturbation_bigfloat,
    compute_pixel_perturbation_floatexp, compute_pixel_perturbation_floatexp_bla,
    BlaTable, MandelbrotRenderer, ReferenceOrbit, Renderer, TestImageRenderer,
};
```

Update `CachedOrbit` struct:

```rust
/// Cached reference orbit for perturbation rendering.
struct CachedOrbit {
    c_ref: (f64, f64),
    orbit: Vec<(f64, f64)>,
    escaped_at: Option<u32>,
    bla_table: BlaTable,
}
```

**Step 2: Update to_reference_orbit method**

The method stays the same - it only returns ReferenceOrbit (BlaTable accessed separately):

```rust
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

**Step 3: Build BlaTable in StoreReferenceOrbit handler**

Update the `StoreReferenceOrbit` handler (~line 256-271):

```rust
MainToWorker::StoreReferenceOrbit {
    orbit_id,
    c_ref,
    orbit,
    escaped_at,
    dc_max,
} => {
    // Build reference orbit for BLA table construction
    let ref_orbit = ReferenceOrbit {
        c_ref,
        orbit: orbit.clone(),
        escaped_at,
    };

    // Build BLA table
    let bla_table = BlaTable::compute(&ref_orbit, dc_max);

    web_sys::console::log_1(
        &format!(
            "[Worker] Built BLA table: {} entries, {} levels",
            bla_table.entries.len(),
            bla_table.num_levels
        )
        .into(),
    );

    state.orbit_cache.insert(
        orbit_id,
        CachedOrbit {
            c_ref,
            orbit,
            escaped_at,
            bla_table,
        },
    );
    post_message(&WorkerToMain::OrbitStored { orbit_id });
}
```

**Step 4: Build and verify compilation**

Run: `cargo check -p fractalwonder-compute`
Expected: PASS

**Step 5: Commit**

```bash
git add fractalwonder-compute/src/worker.rs
git commit -m "$(cat <<'EOF'
feat(worker): build BlaTable when storing reference orbit

Workers now construct BLA acceleration tables when receiving orbits,
enabling iteration skipping in the render loop. Logs table size for
debugging.

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude <noreply@anthropic.com>
EOF
)"
```

---

## Task 4: Use BLA-Accelerated Function in Worker

**Files:**
- Modify: `fractalwonder-compute/src/worker.rs:356-386` (FloatExp rendering path)

**Step 1: Update FloatExp path to use BLA**

In `RenderTilePerturbation` handler, replace the FloatExp path (~line 356-386):

```rust
} else if delta_log2 > -1000.0 || precision <= bigfloat_threshold_bits {
    // Medium path: FloatExp with BLA acceleration
    let delta_origin = (
        FloatExp::from_bigfloat(&delta_c_origin.0),
        FloatExp::from_bigfloat(&delta_c_origin.1),
    );
    let delta_step = (
        FloatExp::from_bigfloat(&delta_c_step.0),
        FloatExp::from_bigfloat(&delta_c_step.1),
    );

    let mut delta_c_row = delta_origin;

    for _py in 0..tile.height {
        let mut delta_c = delta_c_row;

        for _px in 0..tile.width {
            let result = compute_pixel_perturbation_floatexp_bla(
                &orbit,
                &cached.bla_table,
                delta_c,
                max_iterations,
                tau_sq,
            );
            data.push(ComputeData::Mandelbrot(result));

            delta_c.0 = delta_c.0.add(&delta_step.0);
        }

        delta_c_row.1 = delta_c_row.1.add(&delta_step.1);
    }
}
```

**Step 2: Build and run tests**

Run: `cargo test --workspace`
Expected: PASS

**Step 3: Commit**

```bash
git add fractalwonder-compute/src/worker.rs
git commit -m "$(cat <<'EOF'
feat(worker): use BLA-accelerated perturbation for FloatExp path

Replaces compute_pixel_perturbation_floatexp with the BLA-accelerated
version, enabling iteration skipping for significant speedup at deep zoom.

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude <noreply@anthropic.com>
EOF
)"
```

---

## Task 5: Update cell orbit broadcasting for dc_max

**Files:**
- Modify: `fractalwonder-ui/src/workers/worker_pool.rs:985-1057` (broadcast_cell_orbits_to_workers)

**Step 1: Pass dc_max when broadcasting cell orbits**

In `broadcast_cell_orbits_to_workers`, update the message construction (~line 1019):

```rust
// Broadcast to all workers
let msg = MainToWorker::StoreReferenceOrbit {
    orbit_id,
    c_ref: orbit.c_ref,
    orbit: orbit.orbit.clone(),
    escaped_at: orbit.escaped_at,
    dc_max: self.perturbation.dc_max,
};
```

**Step 2: Build and verify**

Run: `cargo check --workspace`
Expected: PASS

**Step 3: Commit**

```bash
git add fractalwonder-ui/src/workers/worker_pool.rs
git commit -m "$(cat <<'EOF'
fix(worker-pool): include dc_max in cell orbit broadcasts

Ensures multi-reference cell orbits also get proper BLA table construction.

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude <noreply@anthropic.com>
EOF
)"
```

---

## Task 6: Final Quality Checks

**Step 1: Format code**

Run: `cargo fmt --all`

**Step 2: Run Clippy**

Run: `cargo clippy --all-targets --all-features -- -D warnings`
Expected: No warnings

**Step 3: Run all tests**

Run: `cargo test --workspace --all-targets --all-features -- --nocapture`
Expected: All tests pass

**Step 4: Check WASM build**

Run: `cargo check --target wasm32-unknown-unknown -p fractalwonder-ui -p fractalwonder-compute`
Expected: PASS

**Step 5: Commit any formatting changes**

```bash
git add -A
git commit -m "$(cat <<'EOF'
chore: apply formatting fixes

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude <noreply@anthropic.com>
EOF
)"
```

---

## Summary

| Task | Description | Key Change |
|------|-------------|------------|
| 1 | Add dc_max to message | `StoreReferenceOrbit { dc_max: f64 }` |
| 2 | Compute dc_max in pool | `calculate_dc_max()` from viewport |
| 3 | Build BlaTable in worker | `BlaTable::compute(&orbit, dc_max)` |
| 4 | Use BLA function | `compute_pixel_perturbation_floatexp_bla` |
| 5 | Fix cell orbit broadcast | Include dc_max for multi-reference |
| 6 | Quality checks | Format, clippy, tests |

**After completion:**
- Workers build BLA tables when receiving orbits
- Perturbation renders use O(log n) iteration skipping
- Multi-reference orbits also benefit from BLA
