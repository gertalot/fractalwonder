# GPU HDRFloat Precision Fix Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Fix GPU HDRFloat magnitude comparisons to match CPU precision, eliminating ~1% glitched pixels caused by incorrect rebase decisions.

**Architecture:** Replace f32 magnitude comparisons with HDRFloat comparisons in the progressive GPU shader. Add `hdr_less_than()` and `hdr_complex_norm_sq_hdr()` functions to compare magnitudes without losing extended exponent range.

**Tech Stack:** WGSL shaders, wgpu, Rust tests

**Design Doc:** `docs/plans/2025-01-09-gpu-hdrfloat-precision-fix-design.md`

---

## Task 1: Add HDRFloat Comparison Functions

**Files:**
- Modify: `fractalwonder-gpu/src/shaders/progressive_iteration.wgsl:144` (after `hdr_complex_norm_sq`)

**Step 1: Add `hdr_complex_norm_sq_hdr` function**

Add after line 144 (after existing `hdr_complex_norm_sq` function):

```wgsl
// Return norm_sq as HDRFloat (preserves extended exponent range)
fn hdr_complex_norm_sq_hdr(a: HDRComplex) -> HDRFloat {
    let re_sq = hdr_square(a.re);
    let im_sq = hdr_square(a.im);
    return hdr_add(re_sq, im_sq);
}
```

**Step 2: Add `hdr_less_than` function**

Add after `hdr_complex_norm_sq_hdr`:

```wgsl
// Compare two HDRFloat values: a < b
// For magnitude comparisons, both values are non-negative
fn hdr_less_than(a: HDRFloat, b: HDRFloat) -> bool {
    // Handle zeros
    let a_zero = a.head == 0.0 && a.tail == 0.0;
    let b_zero = b.head == 0.0 && b.tail == 0.0;
    if a_zero { return !b_zero; }
    if b_zero { return false; }

    // Compare exponents first (both positive for magnitudes)
    if a.exp != b.exp {
        return a.exp < b.exp;
    }

    // Same exponent - compare mantissas
    return (a.head + a.tail) < (b.head + b.tail);
}
```

**Step 3: Add `hdr_greater_than` function**

Add after `hdr_less_than`:

```wgsl
// Compare: a > b
fn hdr_greater_than(a: HDRFloat, b: HDRFloat) -> bool {
    return hdr_less_than(b, a);
}
```

**Step 4: Add `hdr_from_f32_const` function**

Add after `hdr_greater_than`:

```wgsl
// Create HDRFloat from f32 constant (for escape_radius_sq, tau_sq)
fn hdr_from_f32_const(val: f32) -> HDRFloat {
    if val == 0.0 { return HDR_ZERO; }
    return hdr_normalize(HDRFloat(val, 0.0, 0));
}
```

**Step 5: Verify shader compiles**

Run: `cargo build -p fractalwonder-gpu 2>&1 | head -20`

Expected: Build succeeds (no WGSL compile errors)

**Step 6: Commit**

```bash
git add fractalwonder-gpu/src/shaders/progressive_iteration.wgsl
git commit -m "feat(gpu): add HDRFloat comparison functions

Add hdr_complex_norm_sq_hdr, hdr_less_than, hdr_greater_than,
and hdr_from_f32_const for precise magnitude comparisons."
```

---

## Task 2: Update Escape Check to Use HDRFloat

**Files:**
- Modify: `fractalwonder-gpu/src/shaders/progressive_iteration.wgsl:303-318`

**Step 1: Locate the current escape check**

Current code at lines 303-318:
```wgsl
let z_mag_sq = hdr_complex_norm_sq(z);
// ...
if z_mag_sq > uniforms.escape_radius_sq {
```

**Step 2: Update to use HDRFloat comparison**

Replace lines 303-318 with:

```wgsl
// Compute magnitudes as HDRFloat (preserves precision)
let z_mag_sq_hdr = hdr_complex_norm_sq_hdr(z);
let dz_mag_sq_hdr = hdr_complex_norm_sq_hdr(dz);

// For output, convert to f32
let z_mag_sq = hdr_to_f32(z_mag_sq_hdr);

// Escape check - use HDRFloat comparison
let escape_radius_sq_hdr = hdr_from_f32_const(uniforms.escape_radius_sq);
if hdr_greater_than(z_mag_sq_hdr, escape_radius_sq_hdr) {
    escaped_buf[linear_idx] = 1u;
    results[linear_idx] = n;
    glitch_flags[linear_idx] = select(0u, 1u, glitched);
    z_norm_sq[linear_idx] = z_mag_sq;
    store_z_re(linear_idx, dz.re);
    store_z_im(linear_idx, dz.im);
    iter_count[linear_idx] = n;
    orbit_index[linear_idx] = m;
    return;
}
```

**Step 3: Verify shader compiles**

Run: `cargo build -p fractalwonder-gpu 2>&1 | head -20`

Expected: Build succeeds

**Step 4: Commit**

```bash
git add fractalwonder-gpu/src/shaders/progressive_iteration.wgsl
git commit -m "feat(gpu): update escape check to use HDRFloat comparison"
```

---

## Task 3: Update Glitch Detection to Use HDRFloat

**Files:**
- Modify: `fractalwonder-gpu/src/shaders/progressive_iteration.wgsl:320-323`

**Step 1: Locate current glitch detection**

Current code at lines 320-323:
```wgsl
if z_m_mag_sq > 1e-20 && z_mag_sq < uniforms.tau_sq * z_m_mag_sq {
    glitched = true;
}
```

**Step 2: Update to use HDRFloat comparison**

Replace with:

```wgsl
// Glitch detection - use HDRFloat comparison
let z_m_mag_sq_hdr = hdr_from_f32_const(z_m_mag_sq);
let threshold_hdr = hdr_from_f32_const(1e-20);
if hdr_greater_than(z_m_mag_sq_hdr, threshold_hdr) {
    let tau_z_m_sq_hdr = hdr_mul_f32(z_m_mag_sq_hdr, uniforms.tau_sq);
    if hdr_less_than(z_mag_sq_hdr, tau_z_m_sq_hdr) {
        glitched = true;
    }
}
```

**Step 3: Verify shader compiles**

Run: `cargo build -p fractalwonder-gpu 2>&1 | head -20`

Expected: Build succeeds

**Step 4: Commit**

```bash
git add fractalwonder-gpu/src/shaders/progressive_iteration.wgsl
git commit -m "feat(gpu): update glitch detection to use HDRFloat comparison"
```

---

## Task 4: Update Rebase Check to Use HDRFloat (Critical Fix)

**Files:**
- Modify: `fractalwonder-gpu/src/shaders/progressive_iteration.wgsl:325-333`

**Step 1: Locate current rebase check**

Current code at lines 325-333:
```wgsl
if z_mag_sq < dz_mag_sq {
    dz = z;
    m = 0u;
    continue;
}
```

**Step 2: Update to use HDRFloat comparison**

Replace with:

```wgsl
// Rebase check - THE CRITICAL FIX - use HDRFloat comparison
if hdr_less_than(z_mag_sq_hdr, dz_mag_sq_hdr) {
    dz = z;
    m = 0u;
    continue;
}
```

Note: `z_mag_sq_hdr` and `dz_mag_sq_hdr` were already computed in Task 2.

**Step 3: Verify shader compiles**

Run: `cargo build -p fractalwonder-gpu 2>&1 | head -20`

Expected: Build succeeds

**Step 4: Commit**

```bash
git add fractalwonder-gpu/src/shaders/progressive_iteration.wgsl
git commit -m "fix(gpu): use HDRFloat comparison for rebase check

This is the critical fix. The rebase check z_mag_sq < dz_mag_sq
was using f32 comparison, causing incorrect rebase decisions
when both values were very small. Now uses HDRFloat comparison
to preserve extended exponent range."
```

---

## Task 5: Run Diagnostic Test

**Files:**
- Test: `fractalwonder-gpu/src/tests.rs` (existing test)

**Step 1: Delete cached glitch data to force regeneration**

Run: `rm -f target/glitched_pixels_cache.json`

**Step 2: Run the diagnostic test**

Run: `cargo test -p fractalwonder-gpu debug_glitched_pixels_gpu_vs_cpu -- --nocapture 2>&1 | tail -50`

**Step 3: Analyze results**

Expected before fix:
```
GPU vs CPU HDRFloat: avg diff = 1921, max diff = 22,651 iterations
```

Expected after fix:
```
GPU vs CPU HDRFloat: avg diff < 10, max diff < 100 iterations
```

If results are still poor, proceed to Task 6 for debugging. Otherwise skip to Task 7.

---

## Task 6: Debug if Test Still Fails (Conditional)

**Only do this task if Task 5 shows avg diff > 50**

**Step 1: Add debug output for a single divergent pixel**

Modify the test to print step-by-step comparison for one pixel that diverges.

**Step 2: Compare GPU vs CPU rebase decisions**

Check if GPU triggers rebase at the same iterations as CPU.

**Step 3: Check hdr_less_than edge cases**

Verify the comparison function handles:
- Both values very small (exp < -100)
- Values with same exponent but different mantissas
- Zero vs non-zero

---

## Task 7: Visual Verification

**Step 1: Start the app**

Run: `trunk serve` (should already be running)

**Step 2: Navigate to test coordinates**

In browser, go to: center (-1.2627, -0.4084), zoom 10^6.66

Or use URL if app supports it.

**Step 3: Visual inspection**

Look for:
- Scattered 1px noise (should be eliminated)
- Color discontinuities (should be smooth)
- Flat color blobs (should show detail)

**Step 4: Compare GPU vs CPU toggle**

If app has GPU/CPU toggle, compare both renders visually.

---

## Task 8: Final Commit and Cleanup

**Step 1: Squash commits if desired**

Run: `git log --oneline -10` to review commits

If you want a single commit:
```bash
git rebase -i HEAD~4
# Change "pick" to "squash" for commits 2-4
```

**Step 2: Update design doc with results**

Add a "Results" section to `docs/plans/2025-01-09-gpu-hdrfloat-precision-fix-design.md`:

```markdown
## Results

- Before: GPU vs CPU avg diff = 1921 iterations
- After: GPU vs CPU avg diff = X iterations
- Visual verification: [PASS/FAIL]
- Performance impact: [X% change]
```

**Step 3: Final commit**

```bash
git add docs/plans/
git commit -m "docs: update design doc with implementation results"
```
