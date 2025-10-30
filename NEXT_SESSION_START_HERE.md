# Start Here - Arbitrary Precision Implementation

## Quick Context

We're adding arbitrary precision support to the Mandelbrot renderer so it can zoom to depths like 10^100 and beyond without losing detail.

## What's Done ✓

**Phase 1 (Foundation) - COMPLETE**
- Added `dashu` library (pure Rust, WASM-compatible arbitrary precision)
- Created `BigFloat` type and `ImageFloat` trait ⚠️ **BUT ImageFloat needs to be removed!**
- Made `MandelbrotComputer<T>` generic (works with f64 or BigFloat)
- Renamed `type Coord` → `type Scalar` throughout codebase for clarity
- All tests passing (89/89), clippy clean, WASM builds

## ⚠️ CRITICAL ISSUE DISCOVERED

**ImageFloat trait is REDUNDANT and should not exist!**

**Why:**
- `Point<T>` already defines required operations via trait bounds
- Standard Rust traits (`Add`, `Sub`, `Mul`, `Div`, `From<f64>`) already exist
- We only need ONE simple trait: `ToF64` for display conversion
- ImageFloat was created without understanding the existing `Point<T>` architecture

## What's Next

**MUST START WITH Task 2.0: Remove ImageFloat**

This is now the FIRST task in Phase 2. We need to:

### Immediate Next Steps

1. **Read the full plan**: `/workspace/ARBITRARY_PRECISION_IMPLEMENTATION_PLAN.md`
   - See Task 2.0 for complete details

2. **Task 2.0**: Remove ImageFloat, use standard traits ⚠️ **DO THIS FIRST**
   - Create simple `ToF64` trait in `src/rendering/numeric.rs`
   - Update `MandelbrotComputer` to use standard Rust operators
   - Remove/simplify `ImageFloat`
   - File: `src/rendering/numeric.rs`, `src/rendering/computers/mandelbrot.rs`

3. **Then Task 2.3**: Implement `PrecisionCalculator` (NEW FILE)
   - File: `src/rendering/precision.rs`

4. **Then Task 2.4**: Create `DynamicRenderer` factory (NEW FILE)
   - File: `src/rendering/renderer_factory.rs`

5. **Then Tasks 2.1-2.2**: Make pipeline generic (REFACTORING)

### Key Files to Understand

```
src/rendering/
├── numeric.rs          ✓ DONE - ImageFloat trait, BigFloat type
├── computers/
│   └── mandelbrot.rs   ✓ DONE - Generic MandelbrotComputer<T>
├── precision.rs        ← CREATE THIS FIRST (Task 2.3)
├── renderer_factory.rs ← CREATE THIS SECOND (Task 2.4)
└── ...rest of pipeline needs refactoring for Scalar = T
```

### The Goal

By end of Phase 2:
```rust
// Current: hardcoded f64
let renderer = create_mandelbrot_renderer();  // Always f64

// Target: dynamic based on zoom
let renderer = DynamicRenderer::create_mandelbrot(zoom, colorizer);
// Returns f64 renderer if zoom < 1e10
// Returns BigFloat renderer if zoom >= 1e10
```

### Testing Strategy

- Write tests as you go (TDD)
- Each new file should have comprehensive unit tests
- Don't break existing tests (all 89 must keep passing)
- Run `cargo test` frequently

### Commands to Know

```bash
# Run tests
cargo test --workspace --all-targets --all-features

# Check compilation
cargo check --workspace --all-targets --all-features

# Run clippy
cargo clippy --all-targets --all-features -- -D warnings

# Format code
cargo fmt --all
```

## Important Notes

### About ImageFloat Trait ⚠️ IMPORTANT

**ImageFloat is REDUNDANT and MUST BE REMOVED.**

**The real architecture:**
- `Point<T>` already exists and defines required operations via `where` clauses
- Standard Rust traits (`Add`, `Sub`, `Mul`, `Div`, `From<f64>`, `PartialOrd`) exist
- The ONLY thing Rust doesn't have: a `ToF64` trait for display conversion

**What happened:**
- ImageFloat was created without realizing `Point<T>` already solved the problem
- Phase 1 is complete with it, but it needs to be removed in Phase 2

**The fix (Task 2.0):**
```rust
// Just need this ONE simple trait:
pub trait ToF64 {
    fn to_f64(&self) -> f64;
}

// Then use standard Rust operators everywhere
impl<T> MandelbrotComputer<T>
where
    T: Clone + From<f64> + ToF64 + Add<Output=T> + Mul<Output=T> + ...
{
    let zx = T::from(0.0);           // Standard From
    let zx_sq = zx.clone() * zx;     // Standard Mul
    let display = zx.to_f64();       // Our ToF64
}
```

### About Scalar vs Coord

**Previous name:** `type Coord`
**New name:** `type Scalar` (just renamed!)

**Why renamed:**
- Rust convention: associated types should be descriptive (`Item`, `Output`, `Error`)
- `Coord` was ambiguous (is it the Point or the number inside?)
- `Scalar` is clear: it's the scalar numeric type (`f64`, `BigFloat`)

**Usage:**
```rust
pub trait ImagePointComputer {
    type Scalar;  // The numeric type (f64, BigFloat, etc.)
    fn compute(&self, coord: Point<Self::Scalar>, ...) -> Self::Data;
}
```

## Questions to Ask If Stuck

1. "What is the current state of [file]?" - Read it first
2. "What are similar examples in the codebase?" - Look at existing patterns
3. "What tests would verify this works?" - Write tests first
4. "Does this break any existing functionality?" - Run the test suite

## The Full Plan

See: `/workspace/ARBITRARY_PRECISION_IMPLEMENTATION_PLAN.md`

- **Total:** 6 phases, 26 tasks
- **Done:** Phase 1 (8 tasks) ✓
- **Next:** Phase 2 (6 tasks)
- **Then:** Phases 3-6 (12 tasks)

Good luck! The foundation is solid. Phase 2 is about making the pipeline flexible enough to use it.
