# Histogram Equalization Design

## Overview

Add histogram equalization as a toggle in the Options menu (keyboard shortcut "h"). Orthogonal to palette, cycling, shading, and smooth iteration settings.

Histogram equalization distributes colors evenly across the image based on iteration count frequency, ensuring details are visible at all zoom levels regardless of iteration distribution.

## Architecture

Extends the existing `preprocess → colorize → postprocess` pipeline in `SmoothIterationColorizer`.

### Data Structures

**Context struct** (replaces `Vec<f64>`):

```rust
// colorizers/smooth_iteration.rs
pub struct SmoothIterationContext {
    pub smooth_values: Vec<f64>,
    pub cdf: Option<Vec<f64>>,  // None if histogram disabled
}
```

**Settings** (extend `ColorOptions`):

```rust
// colorizers/settings.rs
pub struct ColorOptions {
    pub palette_id: String,
    pub shading_enabled: bool,
    pub smooth_enabled: bool,
    pub histogram_enabled: bool,  // NEW
    pub cycle_count: u32,
}
```

**ColorSettings** (extend):

```rust
pub struct ColorSettings {
    pub palette: Palette,
    pub cycle_count: f64,
    pub shading: ShadingSettings,
    pub histogram_enabled: bool,  // NEW
}
```

### Algorithm

**Preprocess** (when histogram enabled):

1. Compute smooth values (existing behavior)
2. Build histogram from integer iteration counts:
   ```rust
   let mut histogram = vec![0u64; max_iterations as usize + 1];
   for data in pixels {
       if data.escaped && data.iterations < max_iterations {
           histogram[data.iterations as usize] += 1;
       }
   }
   ```
3. Compute CDF:
   ```rust
   let total: u64 = histogram.iter().sum();
   let mut cdf = vec![0.0; histogram.len()];
   let mut cumulative = 0u64;
   for i in 0..histogram.len() {
       cumulative += histogram[i];
       cdf[i] = cumulative as f64 / total as f64;
   }
   ```

**Colorize**:

```rust
let t = if let Some(cdf) = &context.cdf {
    // Histogram equalization: map through CDF
    cdf[data.iterations as usize]
} else {
    // Linear: normalize by max_iterations
    data.iterations as f64 / data.max_iterations as f64
};

// Apply cycling on top (orthogonal)
let cycled = (t * settings.cycle_count).fract();
let [r, g, b] = settings.palette.sample(cycled);
```

### UI Integration

**OptionsMenu component**:
- Add "Histogram" toggle in Effects section
- Follows same pattern as Shading toggle

**Keyboard shortcut**:
- "h" toggles histogram on/off
- Add to `use_keyboard_shortcuts` hook

**Persistence**:
- Add `histogram_enabled` to `PersistedState`
- Default: `false`

## Files to Modify

| File | Changes |
|------|---------|
| `colorizers/settings.rs` | Add `histogram_enabled` to `ColorOptions` and `ColorSettings` |
| `colorizers/smooth_iteration.rs` | Replace `Vec<f64>` with `SmoothIterationContext`, add CDF computation |
| `colorizers/colorizer.rs` | Update `colorize_quick` to handle new Context type |
| `components/options_menu.rs` | Add Histogram toggle |
| `hooks/use_keyboard_shortcuts.rs` | Add "h" shortcut |
| `persistence.rs` | Add `histogram_enabled` to `PersistedState` |

## Testing

1. **CDF correctness**: Verify CDF is monotonically increasing, ends at 1.0
2. **Edge cases**: All pixels same iteration, single pixel, interior-only image
3. **Toggle behavior**: Switching histogram on/off re-colorizes correctly
4. **Persistence**: Setting survives page reload
5. **Keyboard shortcut**: "h" toggles histogram

## Interaction with Other Features

- **Smooth iteration**: Histogram uses integer `iterations`, smooth uses `final_z_norm_sq`. Independent.
- **Cycling**: CDF output feeds into cycling: `(cdf[i] * cycle_count).fract()`
- **Shading**: Shading runs in postprocess, uses `smooth_values`. Unaffected.
- **Palette**: Palette sampling unchanged, just receives different `t` value.
