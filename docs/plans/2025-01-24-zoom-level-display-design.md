# Iteration 5: Zoom Level Display

## Summary

Add zoom level display to the UI panel. Zoom is calculated as `reference_width / current_width` and formatted for human readability at any depth (including 10^2000+).

## Decisions

- **Width-only calculation**: Zoom based on viewport width only (not height or geometric mean)
- **Reference from FractalConfig**: Caller provides reference width from `FractalConfig.default_viewport().width`
- **No dedicated functions**: No `calculate_zoom_level` or `format_zoom_level` taking BigFloat params
- **Log-space math**: Use `log2(ref) - log2(cur)` to avoid BigFloat division
- **Formatter in ui_panel.rs**: `format_zoom_from_log2(f64) -> String` alongside existing formatters

## Implementation

### Formatter Function

Add to `fractalwonder-ui/src/components/ui_panel.rs`:

```rust
/// Format a zoom level for display using log2 approximation.
///
/// Produces: "1×", "150×", "1.50 × 10^3", "10^2000"
fn format_zoom_from_log2(log2_val: f64) -> String {
    use std::f64::consts::LOG2_10;

    if log2_val.is_nan() || log2_val.is_infinite() {
        return "1×".to_string();
    }

    let log10_val = log2_val / LOG2_10;
    let exponent = log10_val.floor() as i64;
    let mantissa = 10.0_f64.powf(log10_val - exponent as f64);

    if exponent < 3 {
        // Simple format: "1×", "150×"
        let zoom = 10.0_f64.powf(log10_val);
        format!("{:.0}×", zoom)
    } else if mantissa < 1.05 {
        // Drop mantissa when ≈1: "10^3", "10^2000"
        format!("10^{}", exponent)
    } else {
        // Scientific: "1.50 × 10^3"
        format!("{:.2} × 10^{}", mantissa, exponent)
    }
}
```

### UI Integration

In `UIPanel` component, inside the info string closure:

```rust
let cfg = config.get();
let vp = viewport.get();

// Calculate zoom via log subtraction (avoids BigFloat division)
let reference_width = cfg.default_viewport(vp.precision_bits()).width;
let zoom_log2 = reference_width.log2_approx() - vp.width.log2_approx();
let zoom_str = format_zoom_from_log2(zoom_log2);

format!(
    "{} | Zoom: {} | Center: ({}, {}) | Size: {} x {} | Canvas: {}x{} | Precision: {} bits",
    cfg.display_name, zoom_str, cx, cy, w, h, canvas_w, canvas_h, bits
)
```

## Formatting Rules

| Condition | Format | Example |
|-----------|--------|---------|
| exponent < 3 | Plain number | "1×", "150×", "999×" |
| exponent ≥ 3, mantissa ≈ 1 | Power of 10 | "10^3", "10^2000" |
| exponent ≥ 3, mantissa > 1.05 | Scientific | "1.50 × 10^3" |

## Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::LOG2_10;

    #[test]
    fn format_zoom_1x() {
        assert_eq!(format_zoom_from_log2(0.0), "1×");
    }

    #[test]
    fn format_zoom_10x() {
        assert_eq!(format_zoom_from_log2(LOG2_10), "10×");
    }

    #[test]
    fn format_zoom_100x() {
        assert_eq!(format_zoom_from_log2(2.0 * LOG2_10), "100×");
    }

    #[test]
    fn format_zoom_1000x_becomes_scientific() {
        assert_eq!(format_zoom_from_log2(3.0 * LOG2_10), "10^3");
    }

    #[test]
    fn format_zoom_with_mantissa() {
        let log2_1500 = (1500.0_f64).log2();
        assert_eq!(format_zoom_from_log2(log2_1500), "1.50 × 10^3");
    }

    #[test]
    fn format_zoom_extreme() {
        assert_eq!(format_zoom_from_log2(2000.0 * LOG2_10), "10^2000");
    }
}
```

## Browser Test

1. Load app - shows "Zoom: 1×" at default viewport
2. Zoom in with scroll wheel - shows "2×", "4×", "10×", etc.
3. Zoom out - shows values < 1× (e.g., "0.5×" - need to handle this case)
4. Deep zoom - shows "10^50", "1.5 × 10^100", etc.

## Files Changed

- `fractalwonder-ui/src/components/ui_panel.rs`: Add formatter, update info string
