# Deep Zoom Center Tile Bug

At ~10^270 zoom, center tiles hang during CPU rendering while GPU works fine.

## Bug Status: PARTIALLY FIXED - INVESTIGATION ONGOING

The dc_max underflow issue has been fixed by using HDRFloat. However, the test still hangs, indicating additional issues remain.

```bash
# Verifies HDRFloat dc_max does NOT underflow - PASSES
cargo test --package fractalwonder-compute dc_max_at_extreme_zoom -- --nocapture

# This test still HANGS (additional issue beyond dc_max underflow)
cargo test --package fractalwonder-compute deep_zoom_full_tile_with_bla -- --ignored --nocapture

# This test is SLOW but progresses (no BLA)
cargo test --package fractalwonder-compute deep_zoom_full_tile_without_bla -- --ignored --nocapture
```

## Original Root Cause (FIXED)

**BLA (Bilinear Approximation) failed when dc_max underflowed to 0.**

The original `calculate_dc_max` function computed viewport diagonal using f64. At ~10^270 zoom, squaring underflowed to 0. This has been fixed by using HDRFloat throughout.

## Test Case Details

**Location:** `fractalwonder-compute/src/perturbation/tests/deep_zoom_center_tile.rs`

**Failing URL (decoded):**
```
http://127.0.0.1:8080/fractalwonder/#v1:7ZvLbhvZFUX_hWMh2CcIMtA8XxEEhCyVbQJqyaBodxpG_3so...
```

**Viewport:**
- Center X: 0.273000307495579097715200094310253922494103490187797182966812629706330340783242
- Center Y: 0.005838718497531293679839354462882728828030188792949767250660666951674130465532
- Width: 3.68629585526668733757870313779318701180348758566795E-270
- Height: 2.12689256332334093913116602106093685402570700118706E-270
- Precision: 1026 bits
- Zoom: ~10^269 (2^895)

**Canvas:** 773x446 pixels

**Tile size:** 32x32 (DEEP_ZOOM_TILE_SIZE at this zoom)

**Center tiles (sorted by distance from canvas center):**
1. (384, 192) - 20.18px from center
2. (384, 224) - 21.71px from center
3. (352, 192) - 23.82px from center
4. (352, 224) - 25.12px from center

**Test output before hang:**
```
=== DEEP ZOOM FULL TILE TEST ===
Canvas: 773x446
Tile: (384, 192, 32, 32)
Viewport width: 3.68629585526668733757870313779318701180348758566795E-270
Viewport height: 2.12689256332334093913116602106093685402570700118706E-270

dc_max = 2.12793615438037e-270 (log2 = -895.8)
dc_max after JSON: 2.12793615438037e-270 (log2 = -895.8)

Computing reference orbit (10000000 iterations)...
Orbit: 30265 points, escaped_at=Some(30264)
Orbit after JSON round-trip: 30265 points

BLA table: 60536 entries, 16 levels

delta_c_origin log2: re=-903.3, im=-899.7
delta_c_step log2: re=-904.6, im=-904.6

Computing 1024 pixels (32 x 32)...
[HANGS HERE]
```

**Key observation:** The reference orbit escapes at iteration 30264 (not 10M). This location is NOT in the Mandelbrot set - it should render with colors, not black.

## Why GPU Works But CPU Doesn't

GPU does not use BLA. It performs raw perturbation iteration without skipping:

```wgsl
// fractalwonder-gpu/src/shaders/progressive_iteration.wgsl
// Delta iteration: dz' = 2*z_m*dz + dz^2 + dc
// Iterates one-by-one, no BLA
```

| Aspect | GPU | CPU |
|--------|-----|-----|
| BLA | Not used | Used for iteration skipping |
| dc_max | Not needed | Required for BLA table |
| At dc_max=0 | Works | **HANGS** |

## Test Verification

The test exactly matches production code:

1. **Viewport parameters** - Decoded from actual failing URL
2. **Canvas size** - 773x446 from user's browser
3. **Tile calculation** - Identical to `coordinator.rs` lines 269-278
4. **Delta step** - Identical to `coordinator.rs` lines 181-187
5. **JSON serialization** - All values round-trip through JSON like production
6. **Pixel loop** - Computes all 1024 pixels (32x32), identical to `worker.rs` lines 436-464
7. **BLA usage** - Uses `compute_pixel_perturbation_hdr_bla` like production

## Implemented Fix: HDRFloat for dc_max

Professional renderers (FractalZoomer, FractalShark, Fraktaler-3) use extended-range floating point throughout BLA calculations.

### Changes Made

**1. `helpers.rs:51-54` - Calculate dc_max with HDRFloat:** ✅
```rust
pub fn calculate_dc_max(viewport: &Viewport) -> HDRFloat {
    let half_width = HDRFloat::from_bigfloat(&viewport.width).div_f64(2.0);
    let half_height = HDRFloat::from_bigfloat(&viewport.height).div_f64(2.0);
    half_width.square().add(&half_height.square()).sqrt()
}
```

**2. `bla.rs:16` - Store r_sq as HDRFloat:** ✅
```rust
pub struct BlaEntry {
    pub a_re: f64,
    pub a_im: f64,
    pub b_re: f64,
    pub b_im: f64,
    pub l: u32,
    pub r_sq: HDRFloat,  // HDRFloat to avoid underflow
}
```

**3. `bla.rs:40` - `BlaEntry::merge()` uses HDRFloat for radius calculation** ✅

**4. `messages.rs:40` - `MainToWorker::StoreReferenceOrbit` passes HDRFloat dc_max** ✅

**5. `coordinator.rs:43,177,259` - Full HDRFloat pipeline through coordinator** ✅

**6. `worker.rs:291` - BlaTable::compute receives HDRFloat dc_max** ✅

### Why Not Just Disable BLA?

BLA provides 10-100x speedup at deep zoom. Disabling it would make deep zoom renders impractically slow.

### Remaining Issue

Despite dc_max no longer underflowing, the BLA test still hangs. Additional investigation needed to identify the remaining cause.

## Running the Tests

```bash
# Quick check - verifies dc_max calculation works with HDRFloat
cargo test --package fractalwonder-compute dc_max_at_extreme_zoom -- --nocapture

# Sanity check - verifies test infrastructure works
cargo test --package fractalwonder-compute deep_zoom_sanity_check -- --nocapture

# Full reproduction WITH BLA - HANGS (bug present)
cargo test --package fractalwonder-compute deep_zoom_full_tile_with_bla -- --ignored --nocapture

# Full reproduction WITHOUT BLA - PASSES (confirms BLA is the issue)
cargo test --package fractalwonder-compute deep_zoom_full_tile_without_bla -- --ignored --nocapture

# Quick check of all 4 center tiles
cargo test --package fractalwonder-compute deep_zoom_all_center_tiles -- --ignored --nocapture
```

## Key Files

| File | Purpose |
|------|---------|
| `fractalwonder-compute/src/perturbation/tests/deep_zoom_center_tile.rs` | Reproduction test |
| `fractalwonder-ui/src/workers/perturbation/helpers.rs` | `calculate_dc_max()` - root cause |
| `fractalwonder-compute/src/bla.rs` | BLA table construction |
| `fractalwonder-compute/src/perturbation.rs` | `compute_pixel_perturbation_hdr_bla()` - hanging code |
| `fractalwonder-compute/src/worker.rs` | Production worker code path |
| `fractalwonder-ui/src/workers/perturbation/coordinator.rs` | Tile parameter calculation |

## References

- [Deep zoom theory and practice](https://mathr.co.uk/blog/2021-05-14_deep_zoom_theory_and_practice.html)
- [Deep zoom theory and practice (again)](https://mathr.co.uk/blog/2022-02-21_deep_zoom_theory_and_practice_again.html)
- [FractalShark GitHub](https://github.com/mattsaccount364/FractalShark)
- [Fraktaler-3](https://mathr.co.uk/fraktaler-3/)
