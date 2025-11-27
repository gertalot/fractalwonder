# Multi-Reference Perturbation: Incremental Implementation Plan

> **For Claude:** Use superpowers:executing-plans to implement this plan phase-by-phase.
> Each phase MUST be fully verified before proceeding to the next.

**Goal:** Fix perturbation glitches at deep zoom (10^14+) using adaptive multi-reference system.

**Key principle:** Every phase must be independently verifiable. No moving forward until current phase is 100% correct.

**Testing:** `cargo test --workspace`, `cargo clippy`, `cargo fmt`

---

## Phase 0: Visualization Infrastructure

**Goal:** Add ability to SEE glitched pixels without changing computation behavior.

**Files:**
- `fractalwonder-core/src/compute_data.rs` - Add `glitched` field
- `fractalwonder-ui/src/rendering/` - Modify colorizer

**Changes:**
1. Add `glitched: bool` field to `MandelbrotData` (always `false` initially)
2. Modify colorizer to render `glitched: true` pixels in cyan, with brightness based on iteration count
3. Add toggle for glitch visualization (can reuse x-ray mode or separate toggle)

**Acceptance criteria:**
- [ ] Render at 10^14 zoom → **zero cyan pixels** (nothing sets glitched=true yet)
- [ ] Manually set `glitched: true` for a test pixel → see cyan
- [ ] Cyan brightness varies with iteration count
- [ ] `cargo test --workspace` passes
- [ ] `cargo clippy` clean

---

## Phase 1: Glitch Detection

**Goal:** Perturbation DETECTS and MARKS glitches, keeping current fallback behavior.

**Files:**
- `fractalwonder-compute/src/perturbation.rs`

**Mathematical basis:**
```
IF iteration_n >= reference_escaped_at THEN
    pixel is glitched (reference has no data for this iteration)
```

**Changes:**
1. In `compute_pixel_perturbation`, when reference orbit runs out, set `glitched: true`
2. Keep existing fallback behavior (we're adding detection, not changing behavior)

**Tests (using real tile data):**

```rust
#[test]
fn perturbation_marks_glitched_when_reference_exhausted() {
    // Reference escapes at iteration 42135, max_iterations 152295
    // Pixel needing iteration 42136+ should be marked glitched
    let orbit = create_orbit_escaping_at(42135);
    let result = compute_pixel_perturbation(&orbit, delta_c, 152295);

    if result.iterations >= 42135 && !result.escaped {
        assert!(result.glitched,
            "Pixel at iteration {} should be glitched (reference ended at 42135)",
            result.iterations);
    }
}
```

**Acceptance criteria:**
- [ ] Render glitchy tiles from test data → cyan overlay appears on wrong pixels
- [ ] Cyan regions MATCH the visual curved smears/stripes
- [ ] Correct tiles show no cyan (no false positives)
- [ ] Tests verify detection matches mathematical condition
- [ ] `cargo test --workspace` passes

**Test fixtures (real glitchy tile data):**

Tile 1 - Vertically offset stripes (512, 256):
```json
{"tile":{"x":512,"y":256,"width":32,"height":32},"escaped_at":42135,"max_iterations":152295}
```

Tile 2 - Curved smear (320, 128):
```json
{"tile":{"x":320,"y":128,"width":32,"height":32},"escaped_at":42135,"max_iterations":152295}
```

Tile 3 - Curved patches (256, 192):
```json
{"tile":{"x":256,"y":192,"width":32,"height":32},"escaped_at":115848,"max_iterations":312071}
```

---

## Phase 2: Quadtree Structure

**Goal:** Add quadtree with rigorous mathematical tests. No integration yet.

**Files:**
- Create: `fractalwonder-ui/src/workers/quadtree.rs`
- Modify: `fractalwonder-ui/src/workers/mod.rs`

**Mathematical invariants to test:**

1. **Area conservation:** sum(child_areas) = parent_area
2. **No gaps/overlaps:** Every point maps to exactly one child
3. **Containment:** All children within parent bounds
4. **Boundary alignment:** Adjacent children share exact boundaries
5. **Recursive preservation:** Invariants hold after multiple subdivisions
6. **Limit enforcement:** Cannot subdivide past MAX_DEPTH or below MIN_CELL_SIZE

**Test suite:**

```rust
#[test]
fn subdivision_conserves_area_for_all_dimensions() {
    let test_sizes = [
        (8, 8), (9, 9), (15, 16), (17, 17), (33, 33),
        (64, 65), (100, 101), (602, 559), // Real canvas
    ];

    for (width, height) in test_sizes {
        let mut root = QuadtreeCell::new_root((width, height));
        let parent_area = width * height;
        root.subdivide();

        let child_area_sum: u32 = root.children.as_ref().unwrap()
            .iter().map(|c| c.bounds.width * c.bounds.height).sum();

        assert_eq!(child_area_sum, parent_area,
            "{}x{}: children={}, parent={}", width, height, child_area_sum, parent_area);
    }
}

#[test]
fn every_point_in_exactly_one_child_exhaustive() {
    for size in [8, 9, 15, 16, 17] {
        let mut root = QuadtreeCell::new_root((size, size));
        root.subdivide();
        let children = root.children.as_ref().unwrap();

        for x in 0..size {
            for y in 0..size {
                let count = children.iter().filter(|c| c.contains(x, y)).count();
                assert_eq!(count, 1, "{}x{}: ({},{}) in {} children", size, size, x, y, count);
            }
        }
    }
}

#[test]
fn child_boundaries_align_perfectly() {
    for (width, height) in [(32, 32), (33, 33), (602, 559)] {
        let mut root = QuadtreeCell::new_root((width, height));
        root.subdivide();
        let c = root.children.as_ref().unwrap();

        // Horizontal: TL right edge == TR left edge
        assert_eq!(c[0].bounds.x + c[0].bounds.width, c[1].bounds.x);
        // Vertical: TL bottom == BL top
        assert_eq!(c[0].bounds.y + c[0].bounds.height, c[2].bounds.y);
        // Full coverage
        assert_eq!(c[0].bounds.width + c[1].bounds.width, width);
        assert_eq!(c[0].bounds.height + c[2].bounds.height, height);
    }
}

#[test]
fn recursive_subdivision_preserves_invariants() {
    let mut root = QuadtreeCell::new_root((602, 559));
    subdivide_to_depth(&mut root, 4);

    let mut leaves = Vec::new();
    root.collect_all_leaves(&mut leaves);

    // Total area preserved
    let total: u32 = leaves.iter().map(|l| l.bounds.width * l.bounds.height).sum();
    assert_eq!(total, 602 * 559);

    // Every point in exactly one leaf (sampled)
    for x in (0..602).step_by(7) {
        for y in (0..559).step_by(7) {
            let count = leaves.iter().filter(|l| l.contains(x, y)).count();
            assert_eq!(count, 1, "({},{}) in {} leaves", x, y, count);
        }
    }
}
```

**Acceptance criteria:**
- [ ] All 6+ mathematical invariant tests pass
- [ ] Tests cover real canvas (602x559) and tricky dimensions
- [ ] App renders exactly as before (quadtree not used yet)
- [ ] `cargo test --workspace` passes

---

## Phase 3: Message Protocol + Precision Tests

**Goal:** Update message types with precision preservation verified.

**Files:**
- `fractalwonder-core/src/messages.rs`
- Related serialization code

**Precision tests:**

```rust
#[test]
fn bigfloat_json_roundtrip_preserves_precision() {
    let original = BigFloat::parse(
        "-1.100001011100000110010110001110110111101101001001010100101100101011110010000001111001",
        128
    );
    let json = serde_json::to_string(&original).unwrap();
    let parsed: BigFloat = serde_json::from_str(&json).unwrap();

    assert_eq!(original.to_binary_string(), parsed.to_binary_string());
}

#[test]
fn orbit_diverges_with_precision_loss() {
    // Proves precision matters - tiny difference causes divergence
    let c1 = (-1.5224851508349150000, 0.0);
    let c2 = (-1.5224851508349151, 0.0);

    let orbit1 = ReferenceOrbit::compute(&c1, 50000);
    let orbit2 = ReferenceOrbit::compute(&c2, 50000);

    assert!(orbit1.escaped_at != orbit2.escaped_at || orbit1.orbit.last() != orbit2.orbit.last(),
        "Orbits should diverge - test sensitivity may need adjustment");
}

#[test]
fn real_tile_data_survives_roundtrip() {
    let c_ref_json = r#"[{"value":"-1.1000010111...","precision_bits":86},...]"#;
    let parsed: (BigFloat, BigFloat) = serde_json::from_str(c_ref_json).unwrap();
    let reserialized = serde_json::to_string(&parsed).unwrap();
    let reparsed: (BigFloat, BigFloat) = serde_json::from_str(&reserialized).unwrap();

    assert_eq!(parsed.0.to_binary_string(), reparsed.0.to_binary_string());
}
```

**Acceptance criteria:**
- [ ] All precision roundtrip tests pass
- [ ] Existing messages still work
- [ ] App renders exactly as before
- [ ] `cargo test --workspace` passes

---

## Phase 4: Glitch Counting (Observation Only)

**Goal:** WorkerPool logs glitch counts without taking action.

**Files:**
- `fractalwonder-ui/src/workers/worker_pool.rs`

**Changes:**
1. In `TileComplete` handler, count pixels where `glitched == true`
2. Log: `"Tile (x,y): N/1024 pixels glitched"`
3. At render completion: `"Render complete: M tiles had glitches"`

**Acceptance criteria:**
- [ ] Console shows glitch counts correlating with visual glitches
- [ ] Your test tiles report >0 glitched pixels
- [ ] Visually-correct tiles report 0 glitched
- [ ] Rendering behavior unchanged

---

## Phase 5: Quadtree Tracking

**Goal:** Connect quadtree to WorkerPool for tracking only.

**Files:**
- `fractalwonder-ui/src/workers/worker_pool.rs`

**Changes:**
1. Create quadtree at render start
2. Associate glitched tiles with quadtree cells
3. Log at completion: `"Quadtree cell (0,0)-(602,559): N glitched tiles"`

**Acceptance criteria:**
- [ ] Quadtree created with correct canvas dimensions
- [ ] Glitch counts match Phase 4 logs
- [ ] Rendering unchanged

---

## Phase 6: Quadtree Subdivision (Manual)

**Goal:** Subdivide cells via "d" key when x-ray enabled.

**Files:**
- `fractalwonder-ui/src/hooks/` or event handling

**Changes:**
1. "d" key (when x-ray enabled) subdivides cells with glitched tiles
2. Log new structure: `"Cell (0,0)-(301,279): 5 glitched tiles"`

**Acceptance criteria:**
- [ ] "d" does nothing when x-ray disabled
- [ ] "d" subdivides when x-ray enabled
- [ ] Console shows correct bounds (verify vs Phase 2 tests)
- [ ] Can press "d" multiple times for deeper subdivision
- [ ] Rendering unchanged

---

## Phase 7: Compute Reference Orbits

**Goal:** Compute orbits for subdivided cell centers with verified correctness.

**Files:**
- Reference orbit computation code

**Mathematical tests:**

```rust
#[test]
fn orbit_satisfies_recurrence_relation() {
    let c_ref = (-0.5, 0.1);
    let orbit = ReferenceOrbit::compute(&c_ref, 100);

    for n in 0..orbit.orbit.len() - 1 {
        let (xn, yn) = orbit.orbit[n];
        let (xn1, yn1) = orbit.orbit[n + 1];

        let expected_x = xn * xn - yn * yn + c_ref.0;
        let expected_y = 2.0 * xn * yn + c_ref.1;

        assert!((xn1 - expected_x).abs() < 1e-10);
        assert!((yn1 - expected_y).abs() < 1e-10);
    }
}

#[test]
fn orbit_starts_at_origin() {
    let orbit = ReferenceOrbit::compute(&(-0.5, 0.1), 100);
    assert_eq!(orbit.orbit[0], (0.0, 0.0));
}

#[test]
fn orbit_known_values_c_equals_neg1() {
    // c = -1: orbit is 0, -1, 0, -1, ... (period 2)
    let orbit = ReferenceOrbit::compute(&(-1.0, 0.0), 100);
    assert!(orbit.escaped_at.is_none());
    assert_eq!(orbit.orbit[1], (-1.0, 0.0));
    assert_eq!(orbit.orbit[2], (0.0, 0.0));
}
```

**Acceptance criteria:**
- [ ] All orbit mathematical tests pass
- [ ] Press "d" → subdivide AND compute orbits
- [ ] Console shows c_ref values (cell centers)
- [ ] Rendering still uses original reference

---

## Phase 8: Distribute Orbits to Workers

**Goal:** Send new orbits to workers (but workers don't use them yet).

**Changes:**
1. Broadcast new orbits via `StoreReferenceOrbit`
2. Workers store in HashMap by orbit_id
3. Log: `"Worker N stored orbit #M"`

**Acceptance criteria:**
- [ ] Press "d" → orbits distributed to workers
- [ ] Console confirms worker receipt
- [ ] Rendering unchanged

---

## Phase 9: Re-render Single Tile (Manual)

**Goal:** "r" key re-renders hovered glitched tile with closer reference.

**Changes:**
1. "r" key (x-ray + hovering glitched tile) re-renders that tile
2. Use quadtree cell's orbit_id
3. Log: `"Re-rendering (x,y) with orbit #N (was #1)"`

**Acceptance criteria:**
- [ ] Single tile re-renders
- [ ] Console shows which orbit used
- [ ] **Visual:** tile looks better (less cyan, more detail)
- [ ] Other tiles unchanged

---

## Phase 10: Re-render All Glitched (Manual)

**Goal:** "a" key re-renders ALL glitched tiles.

**Changes:**
1. "a" key queues all glitched tiles for re-render
2. Each uses its quadtree cell's orbit
3. Log progress and final counts

**Acceptance criteria:**
- [ ] Press "d" then "a"
- [ ] Many/most cyan pixels disappear
- [ ] Glitch count decreases
- [ ] Some cyan may remain (need deeper subdivision)

---

## Phase 11: Iterative Manual Refinement

**Goal:** Repeated d → a cycles progressively eliminate glitches.

**Acceptance criteria:**
- [ ] Can cycle: d → a → d → a → ...
- [ ] Each cycle reduces glitch count
- [ ] Eventually: zero cyan OR cells at max depth
- [ ] Console tracks: `"Pass 1: 150 → Pass 2: 23 → Pass 3: 0"`

---

## Phase 12: Automatic Multi-Pass

**Goal:** Automate the refinement loop.

**Changes:**
1. After initial render, if glitches > 0: auto-refine
2. Config: `max_refinement_passes` (default: 5)
3. Log each pass

**Acceptance criteria:**
- [ ] Render at 10^14 → auto-refinement kicks in
- [ ] Final result: zero or near-zero cyan
- [ ] No infinite loops
- [ ] Performance acceptable

---

## Phase 13: BigFloat Fallback

**Goal:** Handle pixels that still glitch after hitting subdivision limits.

**Changes:**
1. When cell can't subdivide but has glitched pixels
2. Compute those pixels via direct BigFloat (slow but correct)
3. Log: `"BigFloat fallback for N pixels"`

**Acceptance criteria:**
- [ ] At extreme zoom, fallback triggers for edge cases
- [ ] Result correct (no glitches)
- [ ] Only affects few pixels (not entire tiles)

---

## Quick Reference

| Phase | Trigger | Key verification |
|-------|---------|------------------|
| 0 | - | No cyan visible |
| 1 | - | Cyan matches glitches |
| 2 | - | Math tests pass |
| 3 | - | Precision tests pass |
| 4 | - | Console logs match visual |
| 5 | - | Quadtree counts correct |
| 6 | "d" | Bounds in console correct |
| 7 | "d" | Orbit tests pass |
| 8 | "d" | Workers acknowledge |
| 9 | "r" | Single tile improves |
| 10 | "a" | Many tiles improve |
| 11 | d→a→d→a | Counts decrease |
| 12 | auto | Works like manual |
| 13 | auto | Extreme zoom correct |
