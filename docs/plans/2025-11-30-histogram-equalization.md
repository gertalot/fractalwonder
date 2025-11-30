# Histogram Equalization Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add histogram equalization as a toggleable effect that distributes colors evenly based on iteration count frequency.

**Architecture:** Extend `SmoothIterationColorizer` with a new `SmoothIterationContext` struct that holds both smooth values and an optional CDF. Build histogram/CDF in `preprocess` when enabled. Use CDF for normalization in `colorize`.

**Tech Stack:** Rust, Leptos, WebAssembly

---

## Task 1: Add `histogram_enabled` to ColorOptions

**Files:**
- Modify: `fractalwonder-ui/src/rendering/colorizers/settings.rs:89-110`

**Step 1: Write the failing test**

Add to the existing `mod tests` section at line 152:

```rust
#[test]
fn color_options_default_histogram_disabled() {
    let options = ColorOptions::default();
    assert!(!options.histogram_enabled);
}

#[test]
fn color_options_to_color_settings_histogram() {
    let options = ColorOptions {
        histogram_enabled: true,
        ..Default::default()
    };
    let settings = options.to_color_settings();
    assert!(settings.histogram_enabled);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test --package fractalwonder-ui color_options_default_histogram`
Expected: FAIL with "no field `histogram_enabled`"

**Step 3: Write minimal implementation**

Add field to `ColorOptions` struct at line 98 (after `smooth_enabled`):

```rust
/// Whether histogram equalization is enabled.
pub histogram_enabled: bool,
```

Add field to `Default` impl at line 107 (after `smooth_enabled: true`):

```rust
histogram_enabled: false,
```

Add field to `ColorSettings` struct at line 54 (after `shading`):

```rust
/// Whether histogram equalization is enabled.
pub histogram_enabled: bool,
```

Update `ColorSettings::default()` at line 63 (after `shading`):

```rust
histogram_enabled: false,
```

Update `ColorSettings::with_palette()` at line 74 (after `shading`):

```rust
histogram_enabled: false,
```

Update `ColorSettings::with_shading()` at line 83 (after `shading`):

```rust
histogram_enabled: false,
```

Update `ColorOptions::to_color_settings()` at line 148 (after `shading`):

```rust
histogram_enabled: self.histogram_enabled,
```

**Step 4: Run test to verify it passes**

Run: `cargo test --package fractalwonder-ui color_options`
Expected: PASS

**Step 5: Commit**

```bash
git add fractalwonder-ui/src/rendering/colorizers/settings.rs
git commit -m "feat(colorizers): add histogram_enabled to ColorOptions and ColorSettings"
```

---

## Task 2: Create SmoothIterationContext struct

**Files:**
- Modify: `fractalwonder-ui/src/rendering/colorizers/smooth_iteration.rs:1-40`

**Step 1: Write the failing test**

Add after the existing tests (around line 244):

```rust
#[test]
fn smooth_iteration_context_default_has_no_cdf() {
    let ctx = SmoothIterationContext::default();
    assert!(ctx.smooth_values.is_empty());
    assert!(ctx.cdf.is_none());
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test --package fractalwonder-ui smooth_iteration_context_default`
Expected: FAIL with "cannot find type `SmoothIterationContext`"

**Step 3: Write minimal implementation**

Add after line 10 (after `SmoothIterationColorizer` struct):

```rust
/// Context data computed during preprocessing.
/// Holds smooth iteration values and optional histogram CDF.
#[derive(Clone, Debug, Default)]
pub struct SmoothIterationContext {
    /// Smooth iteration values per pixel.
    pub smooth_values: Vec<f64>,
    /// CDF for histogram equalization. None if disabled.
    /// Index = iteration count, value = cumulative probability [0,1].
    pub cdf: Option<Vec<f64>>,
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test --package fractalwonder-ui smooth_iteration_context`
Expected: PASS

**Step 5: Commit**

```bash
git add fractalwonder-ui/src/rendering/colorizers/smooth_iteration.rs
git commit -m "feat(colorizers): add SmoothIterationContext struct"
```

---

## Task 3: Add histogram CDF computation function

**Files:**
- Modify: `fractalwonder-ui/src/rendering/colorizers/smooth_iteration.rs`

**Step 1: Write the failing test**

Add to tests section:

```rust
#[test]
fn build_histogram_cdf_uniform_distribution() {
    // 10 pixels with iterations 0-9, max_iter=10
    let data: Vec<ComputeData> = (0..10)
        .map(|i| {
            ComputeData::Mandelbrot(MandelbrotData {
                iterations: i,
                max_iterations: 10,
                escaped: true,
                glitched: false,
                final_z_norm_sq: 100000.0,
            })
        })
        .collect();

    let cdf = build_histogram_cdf(&data, 10);

    // Uniform distribution: CDF should be [0.1, 0.2, 0.3, ..., 1.0]
    assert_eq!(cdf.len(), 11); // max_iter + 1
    assert!((cdf[0] - 0.1).abs() < 0.001);
    assert!((cdf[4] - 0.5).abs() < 0.001);
    assert!((cdf[9] - 1.0).abs() < 0.001);
}

#[test]
fn build_histogram_cdf_skewed_distribution() {
    // Most pixels at iteration 5
    let mut data = Vec::new();
    for _ in 0..90 {
        data.push(ComputeData::Mandelbrot(MandelbrotData {
            iterations: 5,
            max_iterations: 10,
            escaped: true,
            glitched: false,
            final_z_norm_sq: 100000.0,
        }));
    }
    for _ in 0..10 {
        data.push(ComputeData::Mandelbrot(MandelbrotData {
            iterations: 9,
            max_iterations: 10,
            escaped: true,
            glitched: false,
            final_z_norm_sq: 100000.0,
        }));
    }

    let cdf = build_histogram_cdf(&data, 10);

    // Iterations 0-4 have 0 pixels, so CDF stays at 0
    assert_eq!(cdf[0], 0.0);
    assert_eq!(cdf[4], 0.0);
    // Iteration 5 has 90% of pixels
    assert!((cdf[5] - 0.9).abs() < 0.001);
    // Iteration 9 brings it to 100%
    assert!((cdf[9] - 1.0).abs() < 0.001);
}

#[test]
fn build_histogram_cdf_excludes_interior() {
    let data = vec![
        ComputeData::Mandelbrot(MandelbrotData {
            iterations: 5,
            max_iterations: 10,
            escaped: true,
            glitched: false,
            final_z_norm_sq: 100000.0,
        }),
        ComputeData::Mandelbrot(MandelbrotData {
            iterations: 10,
            max_iterations: 10,
            escaped: false, // Interior point
            glitched: false,
            final_z_norm_sq: 0.0,
        }),
    ];

    let cdf = build_histogram_cdf(&data, 10);

    // Only 1 exterior pixel at iteration 5
    assert_eq!(cdf[5], 1.0);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test --package fractalwonder-ui build_histogram_cdf`
Expected: FAIL with "cannot find function `build_histogram_cdf`"

**Step 3: Write minimal implementation**

Add after `compute_smooth_iteration` function (around line 27):

```rust
/// Build histogram CDF from iteration counts.
/// Returns a Vec where cdf[i] = cumulative probability for iteration i.
/// Interior points (escaped=false) are excluded from the histogram.
pub fn build_histogram_cdf(data: &[ComputeData], max_iterations: u32) -> Vec<f64> {
    let len = max_iterations as usize + 1;
    let mut histogram = vec![0u64; len];
    let mut total_exterior = 0u64;

    // Count iterations for exterior points only
    for d in data {
        if let ComputeData::Mandelbrot(m) = d {
            if m.escaped && m.iterations < max_iterations {
                histogram[m.iterations as usize] += 1;
                total_exterior += 1;
            }
        }
    }

    // Build CDF
    let mut cdf = vec![0.0; len];
    if total_exterior > 0 {
        let mut cumulative = 0u64;
        for i in 0..len {
            cumulative += histogram[i];
            cdf[i] = cumulative as f64 / total_exterior as f64;
        }
    }

    cdf
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test --package fractalwonder-ui build_histogram_cdf`
Expected: PASS

**Step 5: Commit**

```bash
git add fractalwonder-ui/src/rendering/colorizers/smooth_iteration.rs
git commit -m "feat(colorizers): add build_histogram_cdf function"
```

---

## Task 4: Update Colorizer impl to use SmoothIterationContext

**Files:**
- Modify: `fractalwonder-ui/src/rendering/colorizers/smooth_iteration.rs:29-83`
- Modify: `fractalwonder-ui/src/rendering/colorizers/colorizer.rs:85-89`

**Step 1: Write the failing test**

Add to tests section:

```rust
#[test]
fn preprocess_builds_cdf_when_histogram_enabled() {
    let colorizer = SmoothIterationColorizer;
    let settings = ColorSettings {
        histogram_enabled: true,
        ..ColorSettings::with_palette(Palette::grayscale())
    };

    let data: Vec<ComputeData> = (0..10)
        .map(|i| {
            ComputeData::Mandelbrot(MandelbrotData {
                iterations: i,
                max_iterations: 10,
                escaped: true,
                glitched: false,
                final_z_norm_sq: 100000.0,
            })
        })
        .collect();

    let ctx = colorizer.preprocess(&data, &settings);

    assert!(ctx.cdf.is_some());
    let cdf = ctx.cdf.unwrap();
    assert_eq!(cdf.len(), 11);
}

#[test]
fn preprocess_no_cdf_when_histogram_disabled() {
    let colorizer = SmoothIterationColorizer;
    let settings = ColorSettings::with_palette(Palette::grayscale());

    let data = vec![make_escaped(5, 10)];
    let ctx = colorizer.preprocess(&data, &settings);

    assert!(ctx.cdf.is_none());
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test --package fractalwonder-ui preprocess_builds_cdf`
Expected: FAIL (signature mismatch or wrong Context type)

**Step 3: Write minimal implementation**

Update the `Colorizer` trait impl (replace lines 29-83):

```rust
impl Colorizer for SmoothIterationColorizer {
    type Context = SmoothIterationContext;

    fn preprocess(&self, data: &[ComputeData], settings: &ColorSettings) -> Self::Context {
        let smooth_values: Vec<f64> = data
            .iter()
            .map(|d| match d {
                ComputeData::Mandelbrot(m) => compute_smooth_iteration(m),
                ComputeData::TestImage(_) => 0.0,
            })
            .collect();

        let cdf = if settings.histogram_enabled {
            // Find max_iterations from first Mandelbrot data point
            let max_iter = data
                .iter()
                .find_map(|d| {
                    if let ComputeData::Mandelbrot(m) = d {
                        Some(m.max_iterations)
                    } else {
                        None
                    }
                })
                .unwrap_or(1000);

            Some(build_histogram_cdf(data, max_iter))
        } else {
            None
        };

        SmoothIterationContext { smooth_values, cdf }
    }

    fn colorize(
        &self,
        data: &ComputeData,
        context: &Self::Context,
        settings: &ColorSettings,
        index: usize,
    ) -> [u8; 4] {
        match data {
            ComputeData::Mandelbrot(m) => {
                let smooth = if index < context.smooth_values.len() {
                    context.smooth_values[index]
                } else {
                    compute_smooth_iteration(m)
                };
                self.colorize_mandelbrot(m, smooth, context, settings)
            }
            ComputeData::TestImage(_) => [128, 128, 128, 255],
        }
    }

    fn postprocess(
        &self,
        pixels: &mut [[u8; 4]],
        data: &[ComputeData],
        context: &Self::Context,
        settings: &ColorSettings,
        width: usize,
        height: usize,
        zoom_level: f64,
    ) {
        apply_slope_shading(
            pixels,
            data,
            &context.smooth_values,
            &settings.shading,
            width,
            height,
            zoom_level,
        );
    }
}
```

Update the `Colorizer` trait in `colorizer.rs` to pass settings to preprocess (line 18):

```rust
fn preprocess(&self, _data: &[ComputeData], _settings: &ColorSettings) -> Self::Context {
    Self::Context::default()
}
```

Update `ColorizerKind::run_pipeline` (line 72):

```rust
let ctx = c.preprocess(data, settings);
```

Update `ColorizerKind::colorize_quick` (line 87):

```rust
Self::SmoothIteration(c) => c.colorize(data, &SmoothIterationContext::default(), settings, 0),
```

Add import at top of colorizer.rs:

```rust
use super::smooth_iteration::SmoothIterationContext;
```

**Step 4: Run test to verify it passes**

Run: `cargo test --package fractalwonder-ui preprocess_`
Expected: PASS

**Step 5: Commit**

```bash
git add fractalwonder-ui/src/rendering/colorizers/smooth_iteration.rs
git add fractalwonder-ui/src/rendering/colorizers/colorizer.rs
git commit -m "feat(colorizers): integrate SmoothIterationContext into Colorizer pipeline"
```

---

## Task 5: Update colorize to use CDF when available

**Files:**
- Modify: `fractalwonder-ui/src/rendering/colorizers/smooth_iteration.rs:86-110`

**Step 1: Write the failing test**

Add to tests section:

```rust
#[test]
fn colorize_uses_cdf_when_available() {
    let colorizer = SmoothIterationColorizer;
    let settings = ColorSettings {
        histogram_enabled: true,
        cycle_count: 1.0, // No cycling for predictable results
        ..ColorSettings::with_palette(Palette::grayscale())
    };

    // Create skewed data: 90 pixels at iter 1, 10 at iter 9
    let mut data = Vec::new();
    for _ in 0..90 {
        data.push(ComputeData::Mandelbrot(MandelbrotData {
            iterations: 1,
            max_iterations: 10,
            escaped: true,
            glitched: false,
            final_z_norm_sq: 100000.0,
        }));
    }
    for _ in 0..10 {
        data.push(ComputeData::Mandelbrot(MandelbrotData {
            iterations: 9,
            max_iterations: 10,
            escaped: true,
            glitched: false,
            final_z_norm_sq: 100000.0,
        }));
    }

    let ctx = colorizer.preprocess(&data, &settings);

    // First pixel (iter 1) should map to CDF[1] = 0.9 (90% of pixels)
    let color1 = colorizer.colorize(&data[0], &ctx, &settings, 0);
    // Last pixel (iter 9) should map to CDF[9] = 1.0
    let color2 = colorizer.colorize(&data[90], &ctx, &settings, 90);

    // With grayscale and CDF, iter 1 gets bright (0.9), iter 9 gets white (1.0)
    // Both should be quite bright since CDF values are high
    assert!(color1[0] > 200, "iter 1 with CDF 0.9 should be bright: {:?}", color1);
    assert!(color2[0] > 250, "iter 9 with CDF 1.0 should be near white: {:?}", color2);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test --package fractalwonder-ui colorize_uses_cdf`
Expected: FAIL (CDF not being used yet)

**Step 3: Write minimal implementation**

Replace the `colorize_mandelbrot_smooth` method with `colorize_mandelbrot` (around line 86):

```rust
impl SmoothIterationColorizer {
    fn colorize_mandelbrot(
        &self,
        data: &MandelbrotData,
        smooth: f64,
        context: &SmoothIterationContext,
        settings: &ColorSettings,
    ) -> [u8; 4] {
        // Interior points are black
        if !data.escaped {
            return [0, 0, 0, 255];
        }

        // Avoid division by zero
        if data.max_iterations == 0 {
            return [0, 0, 0, 255];
        }

        // Normalize: use CDF if available, otherwise linear
        let normalized = if let Some(cdf) = &context.cdf {
            // Use integer iteration for CDF lookup
            let idx = (data.iterations as usize).min(cdf.len().saturating_sub(1));
            cdf[idx]
        } else {
            smooth / data.max_iterations as f64
        };

        // Apply cycling for better color variation
        let t = (normalized * settings.cycle_count).fract();
        let [r, g, b] = settings.palette.sample(t);
        [r, g, b, 255]
    }
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test --package fractalwonder-ui colorize_uses_cdf`
Expected: PASS

**Step 5: Commit**

```bash
git add fractalwonder-ui/src/rendering/colorizers/smooth_iteration.rs
git commit -m "feat(colorizers): use histogram CDF for normalization when enabled"
```

---

## Task 6: Export SmoothIterationContext from mod.rs

**Files:**
- Modify: `fractalwonder-ui/src/rendering/colorizers/mod.rs:14`

**Step 1: Update export**

Change line 14 from:

```rust
pub use smooth_iteration::SmoothIterationColorizer;
```

To:

```rust
pub use smooth_iteration::{SmoothIterationColorizer, SmoothIterationContext};
```

**Step 2: Run all colorizer tests**

Run: `cargo test --package fractalwonder-ui colorizers`
Expected: PASS

**Step 3: Commit**

```bash
git add fractalwonder-ui/src/rendering/colorizers/mod.rs
git commit -m "refactor(colorizers): export SmoothIterationContext"
```

---

## Task 7: Add histogram toggle to OptionsMenu

**Files:**
- Modify: `fractalwonder-ui/src/components/options_menu.rs`

**Step 1: Add props to component**

Update component signature (lines 11-26) to add histogram props:

```rust
#[component]
pub fn OptionsMenu(
    /// 3D shading enabled state
    shading_enabled: Signal<bool>,
    /// Callback when 3D is toggled
    on_shading_toggle: Callback<()>,
    /// Smooth iteration enabled state
    smooth_enabled: Signal<bool>,
    /// Callback when smooth is toggled
    on_smooth_toggle: Callback<()>,
    /// Histogram equalization enabled state
    histogram_enabled: Signal<bool>,
    /// Callback when histogram is toggled
    on_histogram_toggle: Callback<()>,
    /// Current cycle count
    cycle_count: Signal<u32>,
    /// Callback to increase cycles
    on_cycle_up: Callback<()>,
    /// Callback to decrease cycles
    on_cycle_down: Callback<()>,
) -> impl IntoView {
```

**Step 2: Add histogram toggle button**

Add after the Smooth button (after line 72):

```rust
<button
    class="w-full text-left px-4 py-2 text-sm text-gray-300 hover:bg-white/10 hover:text-white flex items-center justify-between"
    on:click=move |_| {
        on_histogram_toggle.call(());
    }
>
    <span class="flex items-center gap-2">
        <span class=move || if histogram_enabled.get() { "opacity-100" } else { "opacity-30" }>
            {move || if histogram_enabled.get() { "☑" } else { "☐" }}
        </span>
        "Histogram"
    </span>
    <span class="text-xs text-gray-500">"[H]"</span>
</button>
```

**Step 3: Verify compilation**

Run: `cargo check --package fractalwonder-ui`
Expected: Error about missing props in ui_panel.rs (expected, fixed in next task)

**Step 4: Commit**

```bash
git add fractalwonder-ui/src/components/options_menu.rs
git commit -m "feat(components): add histogram toggle to OptionsMenu"
```

---

## Task 8: Wire histogram through UIPanel

**Files:**
- Modify: `fractalwonder-ui/src/components/ui_panel.rs:8-50,99-107`

**Step 1: Add props to UIPanel**

Add after `on_smooth_toggle` (around line 31):

```rust
/// Histogram equalization enabled
histogram_enabled: Signal<bool>,
/// Callback to toggle histogram
on_histogram_toggle: Callback<()>,
```

**Step 2: Pass props to OptionsMenu**

Update OptionsMenu usage (around line 99-107):

```rust
<OptionsMenu
    shading_enabled=shading_enabled
    on_shading_toggle=on_shading_toggle
    smooth_enabled=smooth_enabled
    on_smooth_toggle=on_smooth_toggle
    histogram_enabled=histogram_enabled
    on_histogram_toggle=on_histogram_toggle
    cycle_count=cycle_count
    on_cycle_up=on_cycle_up
    on_cycle_down=on_cycle_down
/>
```

**Step 3: Verify compilation**

Run: `cargo check --package fractalwonder-ui`
Expected: Error about missing props in app.rs (expected, fixed in next task)

**Step 4: Commit**

```bash
git add fractalwonder-ui/src/components/ui_panel.rs
git commit -m "feat(components): wire histogram props through UIPanel"
```

---

## Task 9: Wire histogram in App and add keyboard shortcut

**Files:**
- Modify: `fractalwonder-ui/src/app.rs`

**Step 1: Find UIPanel usage and add props**

Search for `<UIPanel` in app.rs and add after `on_smooth_toggle`:

```rust
histogram_enabled=Signal::derive(move || color_options.get().histogram_enabled)
on_histogram_toggle=Callback::new(move |_| {
    set_color_options.update(|opts| {
        opts.histogram_enabled = !opts.histogram_enabled;
        let msg = if opts.histogram_enabled {
            "Histogram: On"
        } else {
            "Histogram: Off"
        };
        set_toast_message.set(Some(msg.to_string()));
    });
})
```

**Step 2: Add keyboard shortcut**

Add after the "s" | "S" case (around line 285):

```rust
"h" | "H" => {
    // Toggle histogram equalization
    set_color_options.update(|opts| {
        opts.histogram_enabled = !opts.histogram_enabled;
        let msg = if opts.histogram_enabled {
            "Histogram: On"
        } else {
            "Histogram: Off"
        };
        set_toast_message.set(Some(msg.to_string()));
    });
}
```

**Step 3: Verify compilation**

Run: `cargo check --package fractalwonder-ui`
Expected: PASS

**Step 4: Commit**

```bash
git add fractalwonder-ui/src/app.rs
git commit -m "feat(app): add histogram toggle and 'h' keyboard shortcut"
```

---

## Task 10: Run full test suite and fix any issues

**Step 1: Run all tests**

```bash
cargo test --workspace --all-targets --all-features -- --nocapture
```

**Step 2: Run clippy**

```bash
cargo clippy --all-targets --all-features -- -D warnings -W clippy::all
```

**Step 3: Run format check**

```bash
cargo fmt --all --check
```

**Step 4: Fix any issues found**

**Step 5: Final commit**

```bash
git add -A
git commit -m "test: ensure histogram equalization tests pass"
```

---

## Task 11: Manual browser testing

**Step 1: Start dev server**

Ensure `trunk serve` is running on http://localhost:8080

**Step 2: Test histogram toggle**

1. Load the app
2. Press 'h' - verify toast shows "Histogram: On"
3. Press 'h' again - verify toast shows "Histogram: Off"
4. Open Options menu - verify Histogram checkbox appears with [H] hint
5. Click Histogram toggle - verify it toggles

**Step 3: Test visual effect**

1. Navigate to a zoom level with varied iteration counts
2. Toggle histogram on/off - observe color distribution changes
3. Verify colors spread more evenly with histogram enabled

**Step 4: Test persistence**

1. Enable histogram
2. Refresh page
3. Verify histogram is still enabled

---

## Summary

| Task | Description | Files |
|------|-------------|-------|
| 1 | Add histogram_enabled to settings | settings.rs |
| 2 | Create SmoothIterationContext struct | smooth_iteration.rs |
| 3 | Add build_histogram_cdf function | smooth_iteration.rs |
| 4 | Update Colorizer to use new Context | smooth_iteration.rs, colorizer.rs |
| 5 | Use CDF in colorize when available | smooth_iteration.rs |
| 6 | Export SmoothIterationContext | mod.rs |
| 7 | Add histogram toggle to OptionsMenu | options_menu.rs |
| 8 | Wire histogram through UIPanel | ui_panel.rs |
| 9 | Wire in App + keyboard shortcut | app.rs |
| 10 | Run full test suite | - |
| 11 | Manual browser testing | - |
