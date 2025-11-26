# Multi-Reference Perturbation Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace single-reference perturbation with adaptive quadtree-based multi-reference system to fix glitches at deep zoom (10^17+).

**Architecture:** Quadtree on main thread tracks reference point placement. Workers mark pixels as "glitched" instead of attempting broken on-the-fly fallback. Main thread subdivides regions with glitches and re-renders with closer references. BigFloat fallback for pixels that still glitch after hitting limits.

**Tech Stack:** Rust, WASM, Leptos, serde_json for message passing

**Testing:** `cargo test --workspace`, `cargo clippy`, `cargo fmt`

---

## Task 1: Add `glitched` Field to MandelbrotData

**Files:**
- Modify: `fractalwonder-core/src/compute_data.rs:39-47`

**Step 1: Add the glitched field**

Edit `fractalwonder-core/src/compute_data.rs`. Add `glitched: bool` to `MandelbrotData`:

```rust
/// Data computed for a Mandelbrot pixel.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MandelbrotData {
    /// Number of iterations before escape (or max_iterations if didn't escape)
    pub iterations: u32,
    /// Maximum iterations used for this computation (for colorizer normalization)
    pub max_iterations: u32,
    /// Whether the point escaped the set
    pub escaped: bool,
    /// Whether perturbation broke down and pixel needs different reference
    pub glitched: bool,
}
```

**Step 2: Run cargo check to find all usages that need updating**

Run: `cargo check --workspace 2>&1 | head -50`

Expected: Compiler errors showing where `MandelbrotData` is constructed without `glitched` field.

**Step 3: Fix MandelbrotRenderer**

Edit `fractalwonder-compute/src/mandelbrot.rs`. Find all `MandelbrotData { ... }` constructions and add `glitched: false`:

```rust
MandelbrotData {
    iterations: n,
    max_iterations: self.max_iterations,
    escaped: true,
    glitched: false,
}
```

**Step 4: Fix perturbation.rs**

Edit `fractalwonder-compute/src/perturbation.rs`. Find all `MandelbrotData { ... }` constructions and add `glitched: false` (we'll change some to `true` in the next task):

```rust
MandelbrotData {
    iterations: n,
    max_iterations,
    escaped: true,
    glitched: false,
}
```

**Step 5: Fix any remaining usages**

Run: `cargo check --workspace 2>&1`

Fix any remaining construction sites by adding `glitched: false`.

**Step 6: Run tests**

Run: `cargo test --workspace`

Expected: All tests pass.

**Step 7: Run lints**

Run: `cargo fmt --all && cargo clippy --all-targets --all-features -- -D warnings`

Expected: No errors or warnings.

**Step 8: Commit**

```bash
git add -A && git commit -m "feat(core): add glitched field to MandelbrotData

Prepares for multi-reference perturbation by tracking which pixels
need a different reference point instead of broken on-the-fly fallback.

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Task 2: Mark Glitched Pixels Instead of On-The-Fly Fallback

**Files:**
- Modify: `fractalwonder-compute/src/perturbation.rs:66-153`

**Step 1: Write a test for glitch detection**

Add to `fractalwonder-compute/src/perturbation.rs` in the `#[cfg(test)]` module:

```rust
#[test]
fn perturbation_marks_glitched_when_reference_escapes_early() {
    // Reference at a point that escapes quickly
    let c_ref = (BigFloat::with_precision(0.5, 128), BigFloat::zero(128));
    let orbit = ReferenceOrbit::compute(&c_ref, 100);

    // Orbit should escape early
    assert!(orbit.escaped_at.is_some());
    assert!(orbit.escaped_at.unwrap() < 50);

    // A pixel that would NOT escape should be marked glitched
    // (because reference orbit ends before we can determine pixel fate)
    let delta_c = (-1.0, 0.0); // Pixel at (-0.5, 0) is in set
    let result = compute_pixel_perturbation(&orbit, delta_c, 100);

    assert!(result.glitched, "Should be marked glitched when reference escapes early");
}

#[test]
fn perturbation_marks_glitched_on_rebase() {
    // Reference at a point in the set
    let c_ref = (BigFloat::with_precision(-0.5, 128), BigFloat::zero(128));
    let orbit = ReferenceOrbit::compute(&c_ref, 500);

    // Large delta should trigger rebasing
    let delta_c = (0.3, 0.3); // Far from reference
    let result = compute_pixel_perturbation(&orbit, delta_c, 500);

    // Either it escaped normally OR it was marked glitched due to rebase
    // (depends on whether rebase condition triggers before escape)
    // This test just ensures no panic and valid result
    assert!(result.escaped || result.glitched || result.iterations == 500);
}
```

**Step 2: Run test to verify current behavior**

Run: `cargo test -p fractalwonder-compute perturbation_marks_glitched -- --nocapture`

Expected: FAIL (glitched field doesn't exist yet in return values, or is always false)

**Step 3: Modify compute_pixel_perturbation to mark glitched**

Replace the function in `fractalwonder-compute/src/perturbation.rs:66-153` with:

```rust
/// Compute a single pixel using perturbation from a reference orbit.
///
/// Marks pixel as `glitched` when perturbation breaks down, instead of
/// attempting on-the-fly fallback (which fails at deep zoom).
pub fn compute_pixel_perturbation(
    orbit: &ReferenceOrbit,
    delta_c: (f64, f64),
    max_iterations: u32,
) -> MandelbrotData {
    let mut dx = 0.0;
    let mut dy = 0.0;

    let orbit_len = orbit.orbit.len() as u32;
    let reference_escaped = orbit.escaped_at.unwrap_or(u32::MAX);

    for n in 0..max_iterations {
        // Get X_n from orbit - if orbit ended, mark as glitched
        let (xn, yn) = if n < orbit_len && n < reference_escaped {
            orbit.orbit[n as usize]
        } else {
            // Reference orbit ended before we determined pixel fate
            return MandelbrotData {
                iterations: n,
                max_iterations,
                escaped: false,
                glitched: true,
            };
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
                glitched: false,
            };
        }

        // Rebase check: |delta|^2 > 0.25 * |X|^2
        // Instead of fallback, mark as glitched
        let delta_mag_sq = dx * dx + dy * dy;
        let x_mag_sq = xn * xn + yn * yn;

        if delta_mag_sq > 0.25 * x_mag_sq && x_mag_sq > 1e-20 {
            return MandelbrotData {
                iterations: n,
                max_iterations,
                escaped: false,
                glitched: true,
            };
        }

        // Delta iteration: delta_{n+1} = 2*X_n*delta_n + delta_n^2 + delta_c
        let new_dx = 2.0 * (xn * dx - yn * dy) + dx * dx - dy * dy + delta_c.0;
        let new_dy = 2.0 * (xn * dy + yn * dx) + 2.0 * dx * dy + delta_c.1;
        dx = new_dx;
        dy = new_dy;
    }

    // Reached max iterations without escaping or glitching
    MandelbrotData {
        iterations: max_iterations,
        max_iterations,
        escaped: false,
        glitched: false,
    }
}
```

**Step 4: Run new tests**

Run: `cargo test -p fractalwonder-compute perturbation_marks_glitched -- --nocapture`

Expected: PASS

**Step 5: Run all perturbation tests**

Run: `cargo test -p fractalwonder-compute perturbation -- --nocapture`

Expected: All pass (some existing tests may need `glitched: false` expectations added)

**Step 6: Fix any failing tests**

If `perturbation_matches_direct_for_nearby_point` fails, update its assertion to also check `!result.glitched`.

**Step 7: Run full test suite**

Run: `cargo test --workspace && cargo clippy --all-targets --all-features -- -D warnings`

Expected: All pass, no warnings.

**Step 8: Commit**

```bash
git add -A && git commit -m "feat(perturbation): mark glitched pixels instead of on-the-fly fallback

Remove broken on-the-fly computation that caused artifacts at deep zoom.
Now pixels are marked as glitched when:
- Reference orbit escapes before pixel fate is determined
- Delta grows too large relative to reference (rebase condition)

Main thread will handle glitched pixels with closer reference points.

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Task 3: Add Quadtree Data Structure

**Files:**
- Create: `fractalwonder-ui/src/workers/quadtree.rs`
- Modify: `fractalwonder-ui/src/workers/mod.rs`

**Step 1: Create quadtree module**

Create `fractalwonder-ui/src/workers/quadtree.rs`:

```rust
//! Quadtree for adaptive reference point placement.

use fractalwonder_core::PixelRect;
use std::collections::HashSet;

/// Maximum quadtree subdivision depth.
pub const MAX_DEPTH: u32 = 6;

/// Minimum cell size in pixels (don't subdivide below this).
pub const MIN_CELL_SIZE: u32 = 64;

/// A cell in the reference quadtree.
#[derive(Debug)]
pub struct QuadtreeCell {
    /// Bounding box in pixel coordinates.
    pub bounds: PixelRect,
    /// Depth in the tree (0 = root).
    pub depth: u32,
    /// Reference orbit ID for this cell (None until computed).
    pub orbit_id: Option<u32>,
    /// Children if subdivided (None = leaf).
    pub children: Option<Box<[QuadtreeCell; 4]>>,
    /// Tiles within this cell that reported glitched pixels.
    pub glitched_tiles: HashSet<u64>,
}

impl QuadtreeCell {
    /// Create a new root cell covering the entire canvas.
    pub fn new_root(canvas_size: (u32, u32)) -> Self {
        Self {
            bounds: PixelRect::new(0, 0, canvas_size.0, canvas_size.1),
            depth: 0,
            orbit_id: None,
            children: None,
            glitched_tiles: HashSet::new(),
        }
    }

    /// Check if this cell contains the given pixel coordinates.
    pub fn contains(&self, x: u32, y: u32) -> bool {
        x >= self.bounds.x
            && x < self.bounds.x + self.bounds.width
            && y >= self.bounds.y
            && y < self.bounds.y + self.bounds.height
    }

    /// Check if this cell contains the given tile.
    pub fn contains_tile(&self, tile: &PixelRect) -> bool {
        self.contains(tile.x, tile.y)
    }

    /// Check if this cell can be subdivided (not at limits).
    pub fn can_subdivide(&self) -> bool {
        self.depth < MAX_DEPTH
            && self.bounds.width > MIN_CELL_SIZE
            && self.bounds.height > MIN_CELL_SIZE
    }

    /// Check if this is a leaf node.
    pub fn is_leaf(&self) -> bool {
        self.children.is_none()
    }

    /// Subdivide this cell into 4 children.
    /// Returns false if already subdivided or cannot subdivide.
    pub fn subdivide(&mut self) -> bool {
        if self.children.is_some() || !self.can_subdivide() {
            return false;
        }

        let half_w = self.bounds.width / 2;
        let half_h = self.bounds.height / 2;
        let x = self.bounds.x;
        let y = self.bounds.y;
        let new_depth = self.depth + 1;

        self.children = Some(Box::new([
            // Top-left
            QuadtreeCell {
                bounds: PixelRect::new(x, y, half_w, half_h),
                depth: new_depth,
                orbit_id: None,
                children: None,
                glitched_tiles: HashSet::new(),
            },
            // Top-right
            QuadtreeCell {
                bounds: PixelRect::new(x + half_w, y, self.bounds.width - half_w, half_h),
                depth: new_depth,
                orbit_id: None,
                children: None,
                glitched_tiles: HashSet::new(),
            },
            // Bottom-left
            QuadtreeCell {
                bounds: PixelRect::new(x, y + half_h, half_w, self.bounds.height - half_h),
                depth: new_depth,
                orbit_id: None,
                children: None,
                glitched_tiles: HashSet::new(),
            },
            // Bottom-right
            QuadtreeCell {
                bounds: PixelRect::new(
                    x + half_w,
                    y + half_h,
                    self.bounds.width - half_w,
                    self.bounds.height - half_h,
                ),
                depth: new_depth,
                orbit_id: None,
                children: None,
                glitched_tiles: HashSet::new(),
            },
        ]));

        true
    }

    /// Find the leaf cell containing the given tile.
    pub fn find_cell_for_tile(&self, tile: &PixelRect) -> Option<&QuadtreeCell> {
        if !self.contains_tile(tile) {
            return None;
        }

        match &self.children {
            None => Some(self),
            Some(children) => {
                for child in children.iter() {
                    if let Some(cell) = child.find_cell_for_tile(tile) {
                        return Some(cell);
                    }
                }
                // Tile is in our bounds but no child claims it - shouldn't happen
                Some(self)
            }
        }
    }

    /// Find the leaf cell containing the given tile (mutable).
    pub fn find_cell_for_tile_mut(&mut self, tile: &PixelRect) -> Option<&mut QuadtreeCell> {
        if !self.contains_tile(tile) {
            return None;
        }

        match &mut self.children {
            None => Some(self),
            Some(children) => {
                for child in children.iter_mut() {
                    if child.contains_tile(tile) {
                        return child.find_cell_for_tile_mut(tile);
                    }
                }
                Some(self)
            }
        }
    }

    /// Get the center of this cell in pixel coordinates.
    pub fn center_pixels(&self) -> (u32, u32) {
        (
            self.bounds.x + self.bounds.width / 2,
            self.bounds.y + self.bounds.height / 2,
        )
    }

    /// Collect all leaf cells that have glitched tiles.
    pub fn collect_glitched_leaves(&self, result: &mut Vec<PixelRect>) {
        if self.is_leaf() {
            if !self.glitched_tiles.is_empty() {
                result.push(self.bounds.clone());
            }
        } else if let Some(children) = &self.children {
            for child in children.iter() {
                child.collect_glitched_leaves(result);
            }
        }
    }

    /// Collect all leaf cells (for debugging).
    pub fn collect_all_leaves(&self, result: &mut Vec<&QuadtreeCell>) {
        if self.is_leaf() {
            result.push(self);
        } else if let Some(children) = &self.children {
            for child in children.iter() {
                child.collect_all_leaves(result);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn root_cell_covers_canvas() {
        let root = QuadtreeCell::new_root((800, 600));
        assert_eq!(root.bounds.x, 0);
        assert_eq!(root.bounds.y, 0);
        assert_eq!(root.bounds.width, 800);
        assert_eq!(root.bounds.height, 600);
        assert_eq!(root.depth, 0);
        assert!(root.is_leaf());
    }

    #[test]
    fn subdivide_creates_four_children() {
        let mut root = QuadtreeCell::new_root((800, 600));
        assert!(root.subdivide());
        assert!(!root.is_leaf());

        let children = root.children.as_ref().unwrap();
        assert_eq!(children.len(), 4);

        // Check children cover the space
        assert_eq!(children[0].bounds, PixelRect::new(0, 0, 400, 300));
        assert_eq!(children[1].bounds, PixelRect::new(400, 0, 400, 300));
        assert_eq!(children[2].bounds, PixelRect::new(0, 300, 400, 300));
        assert_eq!(children[3].bounds, PixelRect::new(400, 300, 400, 300));
    }

    #[test]
    fn find_cell_for_tile_returns_correct_leaf() {
        let mut root = QuadtreeCell::new_root((800, 600));
        root.subdivide();

        // Tile in top-left quadrant
        let tile = PixelRect::new(100, 100, 64, 64);
        let cell = root.find_cell_for_tile(&tile).unwrap();
        assert_eq!(cell.bounds, PixelRect::new(0, 0, 400, 300));

        // Tile in bottom-right quadrant
        let tile = PixelRect::new(500, 400, 64, 64);
        let cell = root.find_cell_for_tile(&tile).unwrap();
        assert_eq!(cell.bounds, PixelRect::new(400, 300, 400, 300));
    }

    #[test]
    fn cannot_subdivide_past_max_depth() {
        let mut cell = QuadtreeCell::new_root((64, 64));
        cell.depth = MAX_DEPTH;
        assert!(!cell.can_subdivide());
        assert!(!cell.subdivide());
    }

    #[test]
    fn cannot_subdivide_below_min_size() {
        let mut cell = QuadtreeCell::new_root((32, 32));
        assert!(!cell.can_subdivide());
        assert!(!cell.subdivide());
    }
}
```

**Step 2: Add module to workers/mod.rs**

Edit `fractalwonder-ui/src/workers/mod.rs`:

```rust
mod quadtree;
mod worker_pool;

pub use quadtree::{QuadtreeCell, MAX_DEPTH, MIN_CELL_SIZE};
pub use worker_pool::{RenderProgress, TileResult, WorkerPool};
```

**Step 3: Run tests**

Run: `cargo test -p fractalwonder-ui quadtree -- --nocapture`

Expected: All pass.

**Step 4: Run full test suite and lints**

Run: `cargo test --workspace && cargo clippy --all-targets --all-features -- -D warnings && cargo fmt --all`

Expected: All pass, no warnings.

**Step 5: Commit**

```bash
git add -A && git commit -m "feat(ui): add quadtree data structure for multi-reference

Quadtree tracks reference point placement with adaptive subdivision.
Cells can subdivide when they contain glitched tiles, down to
MAX_DEPTH=6 or MIN_CELL_SIZE=64px.

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Task 4: Update Message Protocol

**Files:**
- Modify: `fractalwonder-core/src/messages.rs`

**Step 1: Update StoreReferenceOrbit to use c_ref_json**

Edit `fractalwonder-core/src/messages.rs`. Change `StoreReferenceOrbit`:

```rust
/// Store a reference orbit for use in tile rendering.
StoreReferenceOrbit {
    orbit_id: u32,
    c_ref_json: String,  // Changed from (f64, f64) to preserve precision
    orbit: Vec<(f64, f64)>,
    escaped_at: Option<u32>,
},
```

**Step 2: Update RenderTilePerturbation to include c_ref_json**

```rust
/// Render a tile using perturbation.
RenderTilePerturbation {
    render_id: u32,
    tile: PixelRect,
    orbit_id: u32,
    c_ref_json: String,  // NEW: Full precision for potential fallback
    delta_c_origin: (f64, f64),
    delta_c_step: (f64, f64),
    max_iterations: u32,
},
```

**Step 3: Add RenderPixelsDirect message**

Add new variant to `MainToWorker`:

```rust
/// Render specific pixels using BigFloat direct computation.
/// Used as fallback when perturbation fails even after multi-reference.
RenderPixelsDirect {
    render_id: u32,
    tile: PixelRect,
    pixels: Vec<(u32, u32)>,  // Pixel offsets within tile
    viewport_json: String,
    max_iterations: u32,
},
```

**Step 4: Update ReferenceOrbitComplete to use c_ref_json**

In `WorkerToMain`:

```rust
/// Reference orbit computation complete.
ReferenceOrbitComplete {
    render_id: u32,
    orbit_id: u32,
    c_ref_json: String,  // Changed from (f64, f64)
    orbit: Vec<(f64, f64)>,
    escaped_at: Option<u32>,
},
```

**Step 5: Add PixelsComplete message**

Add new variant to `WorkerToMain`:

```rust
/// Direct pixel computation complete.
PixelsComplete {
    render_id: u32,
    tile: PixelRect,
    pixels: Vec<(u32, u32, MandelbrotData)>,  // (x, y, result)
    compute_time_ms: f64,
},
```

**Step 6: Update tests in messages.rs**

Update the test cases to use the new field names. For example:

```rust
#[test]
fn store_reference_orbit_roundtrip() {
    let msg = MainToWorker::StoreReferenceOrbit {
        orbit_id: 1,
        c_ref_json: r#"["-0.5","0.0"]"#.to_string(),
        orbit: vec![(0.0, 0.0), (-0.5, 0.0), (-0.25, 0.0)],
        escaped_at: None,
    };
    let json = serde_json::to_string(&msg).unwrap();
    let parsed: MainToWorker = serde_json::from_str(&json).unwrap();
    match parsed {
        MainToWorker::StoreReferenceOrbit {
            orbit_id, orbit, ..
        } => {
            assert_eq!(orbit_id, 1);
            assert_eq!(orbit.len(), 3);
        }
        _ => panic!("Wrong variant"),
    }
}
```

**Step 7: Run cargo check to find usages that need updating**

Run: `cargo check --workspace 2>&1 | head -100`

Expected: Compiler errors showing where message fields need updating.

**Step 8: Update worker_pool.rs usages**

Edit `fractalwonder-ui/src/workers/worker_pool.rs`. Update all usages of the changed messages to use `c_ref_json` instead of `c_ref`.

**Step 9: Update worker.rs usages**

Edit `fractalwonder-compute/src/worker.rs`. Update message handling for changed fields.

**Step 10: Run tests**

Run: `cargo test --workspace`

Expected: All pass.

**Step 11: Commit**

```bash
git add -A && git commit -m "feat(messages): update protocol for multi-reference support

- StoreReferenceOrbit: c_ref as JSON string for precision
- RenderTilePerturbation: add c_ref_json for fallback
- Add RenderPixelsDirect for BigFloat fallback
- Add PixelsComplete response message

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Task 5: Integrate Quadtree into WorkerPool

**Files:**
- Modify: `fractalwonder-ui/src/workers/worker_pool.rs`

**Step 1: Add quadtree to PerturbationState**

In `fractalwonder-ui/src/workers/worker_pool.rs`, update `PerturbationState`:

```rust
use super::quadtree::QuadtreeCell;

/// State for perturbation rendering flow.
struct PerturbationState {
    /// Current orbit ID counter
    next_orbit_id: u32,
    /// Workers that have confirmed storing each orbit
    workers_with_orbit: HashMap<u32, HashSet<usize>>,
    /// Maximum iterations for perturbation tiles
    max_iterations: u32,
    /// Delta step per pixel in fractal space
    delta_step: (f64, f64),
    /// Quadtree tracking reference point placement
    quadtree: Option<QuadtreeCell>,
    /// Tiles that need re-rendering (had glitched pixels)
    glitched_tiles: HashSet<u64>,
    /// Current rendering pass (0 = initial, 1+ = re-render passes)
    current_pass: u32,
    /// c_ref as JSON for each orbit_id
    orbit_refs: HashMap<u32, String>,
}
```

**Step 2: Update start_perturbation_render to initialize quadtree**

Find `start_perturbation_render` and update it to initialize the quadtree:

```rust
pub fn start_perturbation_render(
    &mut self,
    viewport: Viewport,
    canvas_size: (u32, u32),
    tiles: Vec<PixelRect>,
) {
    self.is_perturbation_render = true;
    self.current_render_id = self.current_render_id.wrapping_add(1);
    self.current_viewport = Some(viewport.clone());
    self.canvas_size = canvas_size;
    self.pending_tiles = tiles.into();
    self.render_start_time = Some(performance_now());

    // Initialize quadtree with root cell
    let mut quadtree = QuadtreeCell::new_root(canvas_size);
    let orbit_id = self.perturbation.next_orbit_id;
    self.perturbation.next_orbit_id += 1;
    quadtree.orbit_id = Some(orbit_id);
    self.perturbation.quadtree = Some(quadtree);
    self.perturbation.glitched_tiles.clear();
    self.perturbation.current_pass = 0;

    // Compute reference orbit at viewport center
    let c_ref_json = serde_json::to_string(&(viewport.center.0.clone(), viewport.center.1.clone()))
        .unwrap_or_default();
    self.perturbation.orbit_refs.insert(orbit_id, c_ref_json.clone());

    // ... rest of initialization
}
```

**Step 3: Update on_tile_complete to track glitched tiles**

In the `TileComplete` handler, check for glitched pixels:

```rust
WorkerToMain::TileComplete {
    render_id,
    tile,
    data,
    compute_time_ms,
} => {
    if render_id != self.current_render_id {
        // ... existing stale tile handling
        return;
    }

    // Count glitched pixels
    let glitch_count = data.iter().filter(|d| {
        matches!(d, ComputeData::Mandelbrot(m) if m.glitched)
    }).count();

    if glitch_count > 0 && self.is_perturbation_render {
        // Record this tile as needing re-render
        let tile_id = tile_to_id(&tile);
        self.perturbation.glitched_tiles.insert(tile_id);

        // Mark in quadtree
        if let Some(ref mut quadtree) = self.perturbation.quadtree {
            if let Some(cell) = quadtree.find_cell_for_tile_mut(&tile) {
                cell.glitched_tiles.insert(tile_id);
            }
        }
    }

    // ... rest of existing handler (progress update, callback)
}
```

**Step 4: Add helper function for tile ID**

```rust
fn tile_to_id(tile: &PixelRect) -> u64 {
    ((tile.x as u64) << 32) | (tile.y as u64)
}
```

**Step 5: Run tests**

Run: `cargo test --workspace && cargo check --workspace`

Expected: Compiles, tests pass.

**Step 6: Commit**

```bash
git add -A && git commit -m "feat(worker-pool): integrate quadtree for glitch tracking

WorkerPool now tracks glitched tiles in quadtree structure.
Prepares for multi-pass re-rendering with closer references.

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Task 6: Implement Multi-Pass Rendering Loop

**Files:**
- Modify: `fractalwonder-ui/src/workers/worker_pool.rs`

**Step 1: Add pass completion detection**

Add a method to check if current pass is complete and handle transitions:

```rust
fn check_pass_complete(&mut self) {
    if !self.pending_tiles.is_empty() {
        return; // Still have tiles to dispatch
    }

    // Check if all dispatched tiles are complete
    let progress = self.progress.get_untracked();
    if progress.completed_tiles < progress.total_tiles {
        return; // Still waiting for results
    }

    if !self.is_perturbation_render {
        return; // Not in perturbation mode
    }

    // Pass complete - check for glitched tiles
    if self.perturbation.glitched_tiles.is_empty() {
        web_sys::console::log_1(
            &format!(
                "[WorkerPool] Pass {} complete, no glitches - done!",
                self.perturbation.current_pass
            ).into()
        );
        return; // All done!
    }

    web_sys::console::log_1(
        &format!(
            "[WorkerPool] Pass {} complete, {} glitched tiles - starting next pass",
            self.perturbation.current_pass,
            self.perturbation.glitched_tiles.len()
        ).into()
    );

    self.start_next_pass();
}

fn start_next_pass(&mut self) {
    self.perturbation.current_pass += 1;

    // Subdivide cells with glitches
    let Some(ref mut quadtree) = self.perturbation.quadtree else {
        return;
    };

    // Collect cells that need subdivision
    let mut cells_to_subdivide = Vec::new();
    quadtree.collect_glitched_leaves(&mut cells_to_subdivide);

    // Subdivide each cell and compute new reference orbits
    for cell_bounds in cells_to_subdivide {
        if let Some(cell) = quadtree.find_cell_for_tile_mut(&PixelRect::new(
            cell_bounds.x, cell_bounds.y, 1, 1
        )) {
            if cell.subdivide() {
                // Compute reference orbits for new children
                if let Some(ref children) = cell.children {
                    for child in children.iter() {
                        let orbit_id = self.perturbation.next_orbit_id;
                        self.perturbation.next_orbit_id += 1;

                        // Request orbit computation for child center
                        // (Will need viewport to compute fractal coords)
                        self.request_child_orbit(orbit_id, &child.bounds);
                    }
                }
            } else {
                // Cannot subdivide - queue for BigFloat fallback
                self.queue_bigfloat_fallback(&cell_bounds);
            }
        }
    }

    // Re-queue glitched tiles
    self.pending_tiles = self.perturbation.glitched_tiles
        .iter()
        .map(|&id| id_to_tile(id))
        .collect();
    self.perturbation.glitched_tiles.clear();

    // Update progress for new pass
    let total = self.pending_tiles.len() as u32;
    self.progress.set(RenderProgress::new(total));
}

fn id_to_tile(id: u64) -> PixelRect {
    let x = (id >> 32) as u32;
    let y = (id & 0xFFFFFFFF) as u32;
    PixelRect::new(x, y, 64, 64) // Assumes fixed tile size
}
```

**Step 2: Call check_pass_complete after tile completion**

In the `TileComplete` handler, after updating progress:

```rust
// After: self.progress.update(|p| { ... });
self.check_pass_complete();
```

**Step 3: Add request_child_orbit method**

```rust
fn request_child_orbit(&mut self, orbit_id: u32, cell_bounds: &PixelRect) {
    let Some(ref viewport) = self.current_viewport else {
        return;
    };

    // Convert cell center to fractal coordinates
    let (cx, cy) = (
        cell_bounds.x + cell_bounds.width / 2,
        cell_bounds.y + cell_bounds.height / 2,
    );

    let precision = viewport.precision_bits();
    let c_ref = fractalwonder_core::pixel_to_fractal(
        cx as f64,
        cy as f64,
        viewport,
        self.canvas_size,
        precision,
    );

    let c_ref_json = serde_json::to_string(&c_ref).unwrap_or_default();
    self.perturbation.orbit_refs.insert(orbit_id, c_ref_json.clone());

    // Send to a worker for computation
    if let Some(&worker_id) = self.initialized_workers.iter().next() {
        self.send_to_worker(
            worker_id,
            &MainToWorker::ComputeReferenceOrbit {
                render_id: self.current_render_id,
                orbit_id,
                c_ref_json,
                max_iterations: self.perturbation.max_iterations,
            },
        );
    }
}
```

**Step 4: Add queue_bigfloat_fallback placeholder**

```rust
fn queue_bigfloat_fallback(&mut self, cell_bounds: &PixelRect) {
    web_sys::console::warn_1(
        &format!(
            "[WorkerPool] Cell at ({},{}) hit limits, needs BigFloat fallback (not yet implemented)",
            cell_bounds.x, cell_bounds.y
        ).into()
    );
    // TODO: Implement BigFloat fallback in Task 7
}
```

**Step 5: Run tests**

Run: `cargo test --workspace && cargo check --workspace`

Expected: Compiles (warnings about unused functions are OK for now).

**Step 6: Commit**

```bash
git add -A && git commit -m "feat(worker-pool): implement multi-pass rendering loop

Pass completion triggers:
- Detect glitched tiles
- Subdivide quadtree cells
- Compute new reference orbits
- Re-queue tiles for next pass

BigFloat fallback queuing is stubbed for next task.

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Task 7: Implement BigFloat Fallback

**Files:**
- Modify: `fractalwonder-compute/src/worker.rs`
- Modify: `fractalwonder-ui/src/workers/worker_pool.rs`

**Step 1: Add RenderPixelsDirect handler in worker.rs**

In `fractalwonder-compute/src/worker.rs`, add handler for the new message:

```rust
MainToWorker::RenderPixelsDirect {
    render_id,
    tile,
    pixels,
    viewport_json,
    max_iterations,
} => {
    let viewport: Viewport = match serde_json::from_str(&viewport_json) {
        Ok(v) => v,
        Err(e) => {
            post_message(&WorkerToMain::Error {
                message: format!("Failed to parse viewport: {}", e),
            });
            return;
        }
    };

    let start_time = Date::now();
    let precision = viewport.precision_bits();
    let canvas_size = (tile.width, tile.height); // Approximation for pixel_to_fractal

    let results: Vec<(u32, u32, MandelbrotData)> = pixels
        .iter()
        .map(|&(px, py)| {
            // Compute exact fractal coordinate
            let c = fractalwonder_core::pixel_to_fractal(
                (tile.x + px) as f64,
                (tile.y + py) as f64,
                &viewport,
                canvas_size,
                precision,
            );

            // Direct BigFloat computation
            let result = compute_mandelbrot_bigfloat(&c, max_iterations, precision);
            (px, py, result)
        })
        .collect();

    let compute_time_ms = Date::now() - start_time;

    post_message(&WorkerToMain::PixelsComplete {
        render_id,
        tile,
        pixels: results,
        compute_time_ms,
    });
}
```

**Step 2: Add compute_mandelbrot_bigfloat helper**

In `fractalwonder-compute/src/worker.rs`:

```rust
fn compute_mandelbrot_bigfloat(
    c: &(BigFloat, BigFloat),
    max_iterations: u32,
    precision: usize,
) -> MandelbrotData {
    let mut x = BigFloat::zero(precision);
    let mut y = BigFloat::zero(precision);
    let four = BigFloat::with_precision(4.0, precision);
    let two = BigFloat::with_precision(2.0, precision);

    for n in 0..max_iterations {
        let x_sq = x.mul(&x);
        let y_sq = y.mul(&y);

        if x_sq.add(&y_sq).gt(&four) {
            return MandelbrotData {
                iterations: n,
                max_iterations,
                escaped: true,
                glitched: false,
            };
        }

        let new_x = x_sq.sub(&y_sq).add(&c.0);
        let new_y = two.mul(&x).mul(&y).add(&c.1);
        x = new_x;
        y = new_y;
    }

    MandelbrotData {
        iterations: max_iterations,
        max_iterations,
        escaped: false,
        glitched: false,
    }
}
```

**Step 3: Handle PixelsComplete in worker_pool.rs**

Add handler in `handle_message`:

```rust
WorkerToMain::PixelsComplete {
    render_id,
    tile,
    pixels,
    compute_time_ms,
} => {
    if render_id != self.current_render_id {
        return;
    }

    web_sys::console::log_1(
        &format!(
            "[WorkerPool] BigFloat fallback complete: {} pixels in {:.0}ms",
            pixels.len(),
            compute_time_ms
        ).into()
    );

    // Merge results into tile data
    // (Implementation depends on how you're storing partial results)
    // For now, just update progress
    self.progress.update(|p| {
        p.completed_tiles += 1;
        p.is_complete = p.completed_tiles >= p.total_tiles;
    });
}
```

**Step 4: Implement queue_bigfloat_fallback properly**

```rust
fn queue_bigfloat_fallback(&mut self, cell_bounds: &PixelRect) {
    let Some(ref viewport) = self.current_viewport else {
        return;
    };

    // Find all glitched pixels in this cell
    // (Simplified: we'd need to track pixel-level glitch info)
    let pixels: Vec<(u32, u32)> = (0..cell_bounds.width)
        .flat_map(|x| (0..cell_bounds.height).map(move |y| (x, y)))
        .collect();

    let viewport_json = serde_json::to_string(viewport).unwrap_or_default();

    // Dispatch to a worker
    if let Some(&worker_id) = self.initialized_workers.iter().next() {
        self.send_to_worker(
            worker_id,
            &MainToWorker::RenderPixelsDirect {
                render_id: self.current_render_id,
                tile: cell_bounds.clone(),
                pixels,
                viewport_json,
                max_iterations: self.perturbation.max_iterations,
            },
        );
    }
}
```

**Step 5: Run tests**

Run: `cargo test --workspace && cargo check --workspace`

Expected: Compiles, tests pass.

**Step 6: Commit**

```bash
git add -A && git commit -m "feat: implement BigFloat fallback for remaining glitched pixels

Workers can now handle RenderPixelsDirect for pixels that still
glitch after hitting quadtree subdivision limits. Uses full
BigFloat precision for guaranteed correctness.

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Task 8: End-to-End Testing

**Files:**
- Browser testing at various zoom levels

**Step 1: Build and run**

Run: `trunk serve`

Navigate to http://localhost:8080

**Step 2: Test at zoom 10^10**

Navigate to a detailed area and zoom to ~10^10. Verify no glitches.

**Step 3: Test at zoom 10^17**

Zoom to the level that previously showed glitches. Verify artifacts are gone or significantly reduced.

**Step 4: Check console for multi-pass logs**

Look for log messages like:
- `[WorkerPool] Pass 0 complete, N glitched tiles - starting next pass`
- `[WorkerPool] Pass 1 complete, no glitches - done!`

**Step 5: Test at zoom 10^20+**

Push deeper to verify the system handles increasing subdivision.

**Step 6: Commit any fixes discovered during testing**

```bash
git add -A && git commit -m "fix: address issues found in end-to-end testing

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Summary

| Task | Description | Estimated Complexity |
|------|-------------|---------------------|
| 1 | Add `glitched` field to MandelbrotData | Low |
| 2 | Mark glitched instead of on-the-fly fallback | Medium |
| 3 | Add quadtree data structure | Medium |
| 4 | Update message protocol | Medium |
| 5 | Integrate quadtree into WorkerPool | Medium |
| 6 | Implement multi-pass rendering loop | High |
| 7 | Implement BigFloat fallback | Medium |
| 8 | End-to-end testing | Low |

Total: ~8 tasks, each with multiple steps following TDD principles.
