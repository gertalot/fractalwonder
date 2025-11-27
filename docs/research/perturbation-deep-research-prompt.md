# World-Class Perturbation Algorithm: Research & Implementation

> **Purpose:** This document is a prompt for Claude to research, design, and implement a production-quality perturbation algorithm capable of world-record deep Mandelbrot zooms (10^2000+).

---

## Context

You are working on Fractal Wonder, a Rust/WASM Mandelbrot renderer. We have a basic perturbation implementation that works at shallow depths but fails at deep zoom (10^14+) with visual glitches that go undetected.

**The problem:** Our current glitch detection only catches "reference exhaustion" (when the reference orbit escapes before the pixel finishes iterating). It completely misses precision-based glitches where the perturbation math becomes numerically unstable. At 10^16 zoom, we see obvious visual artifacts but only 26/342 tiles are marked as glitched.

**Your mission:** Research and implement perturbation theory correctly and completely, matching the quality of the world's best Mandelbrot renderers (Kalles Fraktaler 2, Mandel, etc.).

---

## Phase 1: Deep Research (NO CODE YET)

Before writing any code, you must thoroughly research and document your understanding. Do not skip this phase. Do not assume you know how perturbation works.

### Research Questions to Answer

1. **Perturbation Theory Fundamentals**
   - What is the exact mathematical derivation of perturbation iteration?
   - What is δ (delta) and how does it relate to z and Z (reference)?
   - What are the precision requirements at each step?
   - Why does perturbation allow f64 arithmetic for deep zooms?

2. **Glitch Types and Detection**
   - What are ALL the ways perturbation can fail? (Not just reference exhaustion)
   - What is the mathematical criterion for detecting precision loss?
   - What is the |δz|/|Z| ratio and what threshold indicates glitching?
   - How do you detect glitches when the reference never escapes?
   - What is "loss of precision near zero" and how is it detected?

3. **Rebasing**
   - What is rebasing and why is it necessary?
   - What triggers a rebase operation?
   - What is the mathematical formula for rebasing?
   - How does rebasing interact with glitch detection?

4. **Series Approximation (if applicable)**
   - What is Series Approximation (SA) / Bivariate Linear Approximation (BLA)?
   - How does it accelerate perturbation?
   - What are the failure modes and how are they detected?
   - Is this necessary for correctness, or only for performance?

5. **Multi-Reference Strategies**
   - How do world-class renderers choose reference points?
   - What spatial data structures are used (quadtree, grid, etc.)?
   - How is the "closest" or "best" reference determined for a pixel?
   - How many references are typically needed at various zoom depths?

6. **Implementation Details from Existing Renderers**
   - How does Kalles Fraktaler 2 detect and handle glitches?
   - How does Mandel (by Wolf Jung) approach this?
   - What does the Fractal Forums community recommend?
   - Are there academic papers on perturbation theory for Mandelbrot?

### Research Deliverable

Create a document `docs/research/perturbation-theory.md` containing:

1. **Mathematical Foundation** - Full derivation of perturbation iteration
2. **Glitch Taxonomy** - Every type of glitch, with mathematical detection criteria
3. **Algorithm Specification** - Pseudocode for correct perturbation with full glitch detection
4. **Test Cases** - Mathematical test cases derived from theory (not guessed)
5. **References** - Links to papers, forum posts, and source code you consulted

**Do not proceed to Phase 2 until this document is complete and reviewed.**

---

## Phase 2: Test-First Implementation

After research is complete, implement using strict TDD. Every mathematical property must have a test BEFORE implementation.

### Core Tests to Write First

```rust
// These tests must FAIL before you implement the feature

#[test]
fn detects_glitch_when_delta_magnitude_exceeds_reference() {
    // When |δz| / |Z| > threshold, pixel is glitched
    // This is the PRIMARY glitch detection mechanism
}

#[test]
fn detects_glitch_near_reference_zero_crossing() {
    // When reference passes near zero, precision loss occurs
    // Even if |δz|/|Z| looks ok, the division is unstable
}

#[test]
fn detects_glitch_when_reference_escapes_before_pixel() {
    // Reference exhaustion - our current (incomplete) detection
}

#[test]
fn no_false_positives_for_correct_perturbation() {
    // Pixels that compute correctly must NOT be marked glitched
}

#[test]
fn glitched_pixel_produces_wrong_result_without_detection() {
    // Prove that undetected glitches cause visible errors
    // Compare perturbation result vs high-precision direct computation
}

#[test]
fn rebasing_resets_delta_accumulation() {
    // After rebase, delta should be small relative to new reference
}

#[test]
fn perturbation_matches_direct_computation_when_no_glitch() {
    // The gold standard: perturbation == direct BigFloat computation
}
```

### Implementation Order

1. **Glitch detection** - Get detection right first, even if we don't fix glitches yet
2. **Verification** - Render known-glitchy regions, confirm cyan matches visible artifacts
3. **Multi-reference infrastructure** - Quadtree, reference orbit storage, assignment
4. **Re-rendering with closer references** - Fix glitches by using better references
5. **Automatic refinement** - Iterate until glitch-free or at precision limits
6. **Fallback to BigFloat** - For pixels that can't be fixed with multi-reference

---

## Phase 3: Validation at Extreme Depths

The algorithm is not complete until validated at:

- 10^14 zoom (current failure point)
- 10^50 zoom
- 10^100 zoom
- 10^500 zoom
- 10^1000+ zoom (stretch goal)

For each depth:
1. Render with perturbation + glitch detection
2. All visual artifacts must be detected (cyan overlay matches visible problems)
3. After multi-reference refinement, image must be artifact-free
4. Compare random pixels against BigFloat direct computation

---

## What We Have

The codebase already contains:

- `fractalwonder-compute/src/perturbation.rs` - Basic perturbation (incomplete glitch detection)
- `fractalwonder-core/src/bigfloat.rs` - Arbitrary precision arithmetic
- `fractalwonder-ui/src/workers/quadtree.rs` - Quadtree for spatial subdivision
- `fractalwonder-ui/src/workers/worker_pool.rs` - Multi-worker tile rendering
- Basic infrastructure for multi-reference (orbit storage, distribution to workers)

**Do not assume any of this is correct.** Audit everything against your research findings.

---

## Critical Mindset

1. **Research before code.** The previous attempt failed because we implemented before fully understanding.

2. **Test against reality.** Every mathematical claim must be tested with actual renders at deep zoom.

3. **No assumptions.** If you think "this should work," prove it with a test. If you can't prove it, you don't understand it.

4. **Match the best.** Kalles Fraktaler 2 can zoom to 10^100000. Our algorithm should be based on the same principles.

5. **Detect ALL glitches.** The hardest part is not fixing glitches—it's knowing they exist. Detection must be comprehensive.

6. **Verify visually.** Mathematical correctness means nothing if the image looks wrong. Cyan overlay must match visible artifacts exactly.

---

## Success Criteria

The implementation is complete when:

- [ ] All glitch types are detected (not just reference exhaustion)
- [ ] Cyan overlay exactly matches visible artifacts at any zoom depth
- [ ] Multi-reference refinement eliminates all detected glitches
- [ ] Renders at 10^100+ zoom produce correct images
- [ ] Performance is acceptable (not necessarily optimal, but usable)
- [ ] Every mathematical property is covered by tests
- [ ] Research document explains the theory completely

---

## Starting Point

Begin with:

```
I need to research perturbation theory for Mandelbrot set rendering.
My goal is to understand ALL types of glitches and their mathematical
detection criteria, not just reference exhaustion.

Start by searching for:
- Kalles Fraktaler glitch detection algorithm
- Perturbation theory Mandelbrot mathematical derivation
- |δz|/|Z| glitch detection threshold
- Series approximation BLA Mandelbrot

Then create the research document before any implementation.
```

Do not skip the research phase. The previous implementation failed because we didn't understand the problem fully. This time, understand first, then implement.
