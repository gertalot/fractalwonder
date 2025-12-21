# CPU Derivative Message Passing Fix

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Fix the missing derivative field in `StoreReferenceOrbit` message so CPU workers receive derivative data for 3D lighting.

**Architecture:** The derivative is correctly computed in `ReferenceOrbit::compute` and returned in `ReferenceOrbitComplete`, but dropped when broadcasting via `StoreReferenceOrbit` because that message lacks a `derivative` field. Workers currently substitute zeros. Fix by adding the field to the message and threading it through the worker pool to the worker cache.

**Tech Stack:** Rust, serde (JSON serialization), Web Workers

---

## Task 1: Add `derivative` Field to `StoreReferenceOrbit` Message

**Files:**
- Modify: `fractalwonder-core/src/messages.rs:33-42`

**Step 1: Add the derivative field to the message enum variant**

Locate the `StoreReferenceOrbit` variant and add the `derivative` field after `orbit`:

```rust
/// Store a reference orbit for use in tile rendering.
StoreReferenceOrbit {
    orbit_id: u32,
    c_ref: (f64, f64),
    orbit: Vec<(f64, f64)>,
    derivative: Vec<(f64, f64)>,
    escaped_at: Option<u32>,
    /// Maximum |δc| for any pixel in viewport (for BLA table construction)
    dc_max: f64,
    /// Whether to build BLA table for this orbit
    bla_enabled: bool,
},
```

**Step 2: Run cargo check to identify test failures**

Run: `cargo check --package fractalwonder-core`
Expected: Compilation errors in tests at lines 211 and 319 about missing `derivative` field

---

## Task 2: Update `StoreReferenceOrbit` Tests

**Files:**
- Modify: `fractalwonder-core/src/messages.rs:211-227`
- Modify: `fractalwonder-core/src/messages.rs:319-335`

**Step 1: Update first test (store_reference_orbit_roundtrip)**

```rust
#[test]
fn store_reference_orbit_roundtrip() {
    let msg = MainToWorker::StoreReferenceOrbit {
        orbit_id: 1,
        c_ref: (-0.5, 0.0),
        orbit: vec![(0.0, 0.0), (-0.5, 0.0), (-0.25, 0.0)],
        derivative: vec![(0.0, 0.0), (1.0, 0.0), (1.5, 0.0)],
        escaped_at: None,
        dc_max: 0.01,
        bla_enabled: true,
    };
    let json = serde_json::to_string(&msg).unwrap();
    let parsed: MainToWorker = serde_json::from_str(&json).unwrap();
    match parsed {
        MainToWorker::StoreReferenceOrbit {
            orbit_id, orbit, derivative, ..
        } => {
            assert_eq!(orbit_id, 1);
            assert_eq!(orbit.len(), 3);
            assert_eq!(derivative.len(), 3);
        }
        _ => panic!("Wrong variant"),
    }
}
```

**Step 2: Update second test (store_reference_orbit_with_dc_max_roundtrip)**

```rust
#[test]
fn store_reference_orbit_with_dc_max_roundtrip() {
    let msg = MainToWorker::StoreReferenceOrbit {
        orbit_id: 1,
        c_ref: (-0.5, 0.0),
        orbit: vec![(0.0, 0.0), (-0.5, 0.0)],
        derivative: vec![(0.0, 0.0), (1.0, 0.0)],
        escaped_at: None,
        dc_max: 0.001,
        bla_enabled: true,
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

**Step 3: Run tests to verify they pass**

Run: `cargo test --package fractalwonder-core -- --nocapture`
Expected: All tests PASS

**Step 4: Commit**

```bash
git add fractalwonder-core/src/messages.rs
git commit -m "feat(messages): add derivative field to StoreReferenceOrbit"
```

---

## Task 3: Add `derivative` Field to `OrbitData` Struct

**Files:**
- Modify: `fractalwonder-ui/src/workers/worker_pool.rs:20-24`

**Step 1: Add derivative field to OrbitData**

```rust
struct OrbitData {
    c_ref: (f64, f64),
    orbit: Vec<(f64, f64)>,
    derivative: Vec<(f64, f64)>,
    escaped_at: Option<u32>,
}
```

**Step 2: Run cargo check to identify all construction sites**

Run: `cargo check --package fractalwonder-ui 2>&1 | head -100`
Expected: Compilation error at line ~430 about missing `derivative` field in OrbitData construction

---

## Task 4: Store Derivative When Receiving `ReferenceOrbitComplete`

**Files:**
- Modify: `fractalwonder-ui/src/workers/worker_pool.rs:430-434`

**Step 1: Update OrbitData construction to include derivative**

Find the `ReferenceOrbitComplete` handler (around line 430) and update:

```rust
// Store orbit data
self.perturbation.pending_orbit = Some(OrbitData {
    c_ref,
    orbit: orbit.clone(),
    derivative: derivative.clone(),
    escaped_at,
});
```

**Step 2: Run cargo check**

Run: `cargo check --package fractalwonder-ui`
Expected: Compilation errors at lines ~459 and ~1092 about missing `derivative` field in StoreReferenceOrbit

---

## Task 5: Pass Derivative in Single-Reference Worker Broadcast

**Files:**
- Modify: `fractalwonder-ui/src/workers/worker_pool.rs:456-467`

**Step 1: Update StoreReferenceOrbit message to include derivative**

Find the CPU mode broadcast loop (around line 456) and update:

```rust
// CPU mode: broadcast to all workers
for worker_id in 0..self.workers.len() {
    self.send_to_worker(
        worker_id,
        &MainToWorker::StoreReferenceOrbit {
            orbit_id,
            c_ref,
            orbit: orbit.clone(),
            derivative: derivative.clone(),
            escaped_at,
            dc_max: self.perturbation.dc_max,
            bla_enabled: self.perturbation.bla_enabled,
        },
    );
}
```

**Step 2: Run cargo check**

Run: `cargo check --package fractalwonder-ui`
Expected: Compilation error at line ~1092 about missing `derivative` field

---

## Task 6: Pass Derivative in Cell-Orbits Worker Broadcast

**Files:**
- Modify: `fractalwonder-ui/src/workers/worker_pool.rs:1092-1099`

**Step 1: Update StoreReferenceOrbit message to include derivative**

Find the cell orbits broadcast (around line 1092) and update:

```rust
let msg = MainToWorker::StoreReferenceOrbit {
    orbit_id,
    c_ref: orbit.c_ref,
    orbit: orbit.orbit.clone(),
    derivative: orbit.derivative.clone(),
    escaped_at: orbit.escaped_at,
    dc_max: self.perturbation.dc_max,
    bla_enabled: self.perturbation.bla_enabled,
};
```

**Step 2: Run cargo check**

Run: `cargo check --package fractalwonder-ui`
Expected: SUCCESS (worker_pool.rs now compiles)

**Step 3: Commit**

```bash
git add fractalwonder-ui/src/workers/worker_pool.rs
git commit -m "feat(worker-pool): pass derivative in StoreReferenceOrbit messages"
```

---

## Task 7: Update Worker to Use Passed Derivative

**Files:**
- Modify: `fractalwonder-compute/src/worker.rs:262-322`

**Step 1: Run cargo check to see current error**

Run: `cargo check --package fractalwonder-compute`
Expected: Compilation error about missing `derivative` field in pattern match

**Step 2: Add derivative to message destructuring**

Find the `StoreReferenceOrbit` handler (around line 262) and update the pattern:

```rust
MainToWorker::StoreReferenceOrbit {
    orbit_id,
    c_ref,
    orbit,
    derivative,
    escaped_at,
    dc_max,
    bla_enabled,
} => {
```

**Step 3: Run cargo check**

Run: `cargo check --package fractalwonder-compute`
Expected: Warning about unused `derivative` variable (the TODO code shadows it)

---

## Task 8: Remove Zeroed Derivative Placeholder in BLA Path

**Files:**
- Modify: `fractalwonder-compute/src/worker.rs:277-286`

**Step 1: Remove the TODO and zeroed derivative creation**

Find the BLA table construction (around line 277) and change from:

```rust
let bla_table = if bla_enabled && bla_useful {
    // TODO: derivative should come from the message in future tasks
    let derivative = vec![(0.0, 0.0); orbit.len()];
    let ref_orbit = ReferenceOrbit {
        c_ref,
        orbit: orbit.clone(),
        derivative,
        escaped_at,
    };
```

To:

```rust
let bla_table = if bla_enabled && bla_useful {
    let ref_orbit = ReferenceOrbit {
        c_ref,
        orbit: orbit.clone(),
        derivative: derivative.clone(),
        escaped_at,
    };
```

**Step 2: Run cargo check**

Run: `cargo check --package fractalwonder-compute`
Expected: Warning about unused `derivative` in cache insert section

---

## Task 9: Remove Zeroed Derivative Placeholder in Cache Insert

**Files:**
- Modify: `fractalwonder-compute/src/worker.rs:310-322`

**Step 1: Remove the TODO and zeroed derivative creation**

Find the cache insert (around line 310) and change from:

```rust
// TODO: derivative should come from the message in future tasks
let derivative = vec![(0.0, 0.0); orbit.len()];
state.orbit_cache.insert(
    orbit_id,
    CachedOrbit {
        c_ref,
        orbit,
        derivative,
        escaped_at,
        bla_table,
    },
);
```

To:

```rust
state.orbit_cache.insert(
    orbit_id,
    CachedOrbit {
        c_ref,
        orbit,
        derivative,
        escaped_at,
        bla_table,
    },
);
```

**Step 2: Run tests**

Run: `cargo test --package fractalwonder-compute -- --nocapture`
Expected: All tests PASS

**Step 3: Commit**

```bash
git add fractalwonder-compute/src/worker.rs
git commit -m "fix(worker): use passed derivative instead of zeros"
```

---

## Task 10: Full Verification

**Step 1: Run full test suite**

Run: `cargo test --workspace -- --nocapture`
Expected: All tests PASS

**Step 2: Run clippy**

Run: `cargo clippy --all-targets --all-features -- -D warnings -W clippy::all`
Expected: No errors or warnings

**Step 3: Run cargo fmt**

Run: `cargo fmt --all`
Expected: No changes needed (or apply formatting)

**Step 4: Commit any formatting changes**

```bash
git add -A
git commit -m "chore: formatting cleanup"
```

---

## Summary

| Task | Description | Files |
|------|-------------|-------|
| 1 | Add `derivative` field to message | `messages.rs` |
| 2 | Update message tests | `messages.rs` |
| 3 | Add `derivative` to `OrbitData` | `worker_pool.rs` |
| 4 | Store derivative from `ReferenceOrbitComplete` | `worker_pool.rs` |
| 5 | Pass derivative in single-reference broadcast | `worker_pool.rs` |
| 6 | Pass derivative in cell-orbits broadcast | `worker_pool.rs` |
| 7 | Add derivative to worker message pattern | `worker.rs` |
| 8 | Remove zeroed derivative in BLA path | `worker.rs` |
| 9 | Remove zeroed derivative in cache insert | `worker.rs` |
| 10 | Full verification | all |

Total: ~30 lines of code changes across 3 files.
