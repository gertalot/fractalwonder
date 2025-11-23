# Iteration 3: Config, Precision & Viewport Fitting

## Summary

Implement fractal configuration, precision calculation, and viewport aspect ratio fitting.

## Decisions

| Topic | Decision |
|-------|----------|
| FractalConfig location | UI layer only |
| Precision calculation location | Core layer (`precision.rs`) |
| Precision approach | Exponent extraction (avoids needing `ln()` in dashu) |
| FractalConfig fields | Minimal: id, display_name, center, width, height |
| Config registry | TestImage + Mandelbrot from start |
| Viewport fitting | Always expand to fill canvas (never letterbox) |
| Zoom functions | Defer to Iteration 5 |
| UI panel display | Full technical detail |

## Core Layer Changes

### `fractalwonder-core/src/bigfloat.rs` (additions)

```rust
impl BigFloat {
    /// Approximate log2 using exponent extraction.
    /// Accurate to ~1 bit, sufficient for precision calculation.
    pub fn log2_approx(&self) -> f64;

    /// Absolute value.
    pub fn abs(&self) -> Self;
}
```

### `fractalwonder-core/src/precision.rs` (new file)

```rust
const SAFETY_BITS: u64 = 64;
const DEFAULT_MAX_ITERATIONS: u64 = 10_000;

/// Calculate required precision bits for fractal computation.
pub fn calculate_precision_bits(
    viewport: &Viewport,
    canvas_size: (u32, u32),
) -> usize;

/// Calculate precision bits with custom iteration count.
pub fn calculate_precision_bits_with_iterations(
    viewport: &Viewport,
    canvas_size: (u32, u32),
    max_iterations: u64,
) -> usize;
```

**Algorithm:**
1. Compute `log2(min_delta)` where `delta = dimension / pixels`
2. Compute `log2(M)` where `M = max(|cx| + width/2, |cy| + height/2)`
3. `bits_from_ratio = ceil(log2(M) - log2(min_delta))`
4. Add `log2(iterations)` for error amplification
5. Add safety margin (64 bits)
6. Round to power of 2

### `fractalwonder-core/src/transforms.rs` (addition)

```rust
/// Expand viewport to match canvas aspect ratio.
/// Center stays fixed; dimensions expand (never shrink).
pub fn fit_viewport_to_canvas(
    natural_viewport: &Viewport,
    canvas_size: (u32, u32),
) -> Viewport;
```

## UI Layer Changes

### `fractalwonder-ui/src/config.rs` (new file)

```rust
pub struct FractalConfig {
    pub id: &'static str,
    pub display_name: &'static str,
    pub default_center: (&'static str, &'static str),
    pub default_width: &'static str,
    pub default_height: &'static str,
}

impl FractalConfig {
    pub fn default_viewport(&self, precision_bits: usize) -> Viewport;
}

pub static FRACTAL_CONFIGS: &[FractalConfig] = &[
    FractalConfig {
        id: "test_image",
        display_name: "Test Pattern",
        default_center: ("0.0", "0.0"),
        default_width: "4.0",
        default_height: "4.0",
    },
    FractalConfig {
        id: "mandelbrot",
        display_name: "Mandelbrot Set",
        default_center: ("-0.5", "0.0"),
        default_width: "4.0",
        default_height: "4.0",
    },
];

pub fn get_config(id: &str) -> Option<&'static FractalConfig>;
```

### `fractalwonder-ui/src/components/ui_panel.rs` (modified)

Display format:
```
Fractal: Mandelbrot Set
Viewport: (-0.5, 0.0) | 4.0 × 3.0
Precision: 128 bits
Canvas: 1920 × 1080
```

New props:
- `viewport: Signal<Viewport>`
- `config: Signal<&'static FractalConfig>`
- `precision_bits: Signal<usize>`

### `fractalwonder-ui/src/app.rs` (modified)

New signals:
- `config: RwSignal<&'static FractalConfig>`
- `viewport: RwSignal<Viewport>`
- `precision_bits: Memo<usize>` (derived)

State flow:
1. App starts with default config (mandelbrot)
2. On canvas resize: `fit_viewport_to_canvas()` with natural bounds
3. Compute precision via `calculate_precision_bits()`
4. Update viewport signal with computed precision

## Unit Tests

### Core

- `calculate_precision_bits` returns expected values:
  - 1x zoom, 4K canvas → ~128 bits
  - 10^10 zoom → ~256 bits
  - 10^2000 zoom → ~8192 bits
- `fit_viewport_to_canvas`:
  - Square viewport + landscape canvas → wider viewport
  - Square viewport + portrait canvas → taller viewport
  - Center unchanged after fitting

### UI

- `FractalConfig::default_viewport()` parses strings correctly
- `get_config("mandelbrot")` returns correct config
- `get_config("invalid")` returns None

## Browser Tests

- UI panel shows fractal name and precision
- On landscape monitor: viewport wider than tall
- On portrait monitor: viewport taller than wide
- Resize browser window: precision recalculates
