# Iteration 7: MandelbrotRenderer Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add Mandelbrot computation with UI selection between TestImage and Mandelbrot renderers.

**Architecture:** MandelbrotRenderer implements the Renderer trait using BigFloat for all fractal-space math. UI dropdowns (ported from archive) allow switching renderers. InteractiveCanvas dispatches to the correct renderer based on FractalConfig.id.

**Tech Stack:** Rust, Leptos, BigFloat (dashu), WASM

**Reference Design:** `docs/plans/2025-11-25-iteration-7-mandelbrot-design.md`

---

## Task 1: Add MandelbrotData to Core

**Files:**
- Modify: `fractalwonder-core/src/compute_data.rs`
- Modify: `fractalwonder-core/src/lib.rs`

**Step 1: Add MandelbrotData struct**

In `fractalwonder-core/src/compute_data.rs`, add after `TestImageData`:

```rust
/// Data computed for a Mandelbrot pixel.
#[derive(Clone, Debug, PartialEq)]
pub struct MandelbrotData {
    /// Number of iterations before escape (or max_iterations if didn't escape)
    pub iterations: u32,
    /// Maximum iterations used for this computation (for colorizer normalization)
    pub max_iterations: u32,
    /// Whether the point escaped the set
    pub escaped: bool,
}
```

**Step 2: Add Mandelbrot variant to ComputeData enum**

Update the `ComputeData` enum in the same file:

```rust
/// Unified enum for all compute results.
#[derive(Clone, Debug)]
pub enum ComputeData {
    TestImage(TestImageData),
    Mandelbrot(MandelbrotData),
}
```

**Step 3: Export MandelbrotData from lib.rs**

In `fractalwonder-core/src/lib.rs`, update the compute_data export:

```rust
pub use compute_data::{ComputeData, MandelbrotData, TestImageData};
```

**Step 4: Run tests and checks**

```bash
cargo check --workspace --all-targets --all-features
cargo test --workspace -- --nocapture
```

Expected: All pass, no errors.

**Step 5: Commit**

```bash
git add fractalwonder-core/src/compute_data.rs fractalwonder-core/src/lib.rs
git commit -m "feat(core): add MandelbrotData struct"
```

---

## Task 2: Add calculate_max_iterations to Core

**Files:**
- Modify: `fractalwonder-core/src/transforms.rs`
- Modify: `fractalwonder-core/src/lib.rs`

**Step 1: Write failing tests**

Add at the end of the `tests` module in `fractalwonder-core/src/transforms.rs`:

```rust
    // ============================================================================
    // calculate_max_iterations tests
    // ============================================================================

    #[test]
    fn max_iterations_at_1x_zoom() {
        // At 1x zoom, viewport_width == reference_width
        let width = BigFloat::with_precision(4.0, 128);
        let result = calculate_max_iterations(&width, &width);
        // base = 50, log10(1) = 0, so result = 50
        assert_eq!(result, 50);
    }

    #[test]
    fn max_iterations_at_10x_zoom() {
        let viewport_width = BigFloat::with_precision(0.4, 128);
        let reference_width = BigFloat::with_precision(4.0, 128);
        let result = calculate_max_iterations(&viewport_width, &reference_width);
        // zoom = 10, log10(10) = 1, iterations = 50 + 100 * 1^1.5 = 150
        assert_eq!(result, 150);
    }

    #[test]
    fn max_iterations_at_1000x_zoom() {
        let viewport_width = BigFloat::with_precision(0.004, 128);
        let reference_width = BigFloat::with_precision(4.0, 128);
        let result = calculate_max_iterations(&viewport_width, &reference_width);
        // zoom = 1000, log10(1000) = 3, iterations = 50 + 100 * 3^1.5 ≈ 570
        assert!(result > 500 && result < 600, "Expected ~570, got {}", result);
    }

    #[test]
    fn max_iterations_clamped_to_minimum() {
        // When zoomed out (viewport > reference), iterations should be clamped to 50
        let viewport_width = BigFloat::with_precision(40.0, 128);
        let reference_width = BigFloat::with_precision(4.0, 128);
        let result = calculate_max_iterations(&viewport_width, &reference_width);
        assert_eq!(result, 50);
    }

    #[test]
    fn max_iterations_clamped_to_maximum() {
        // At extreme zoom, should clamp to 10000
        let viewport_width = BigFloat::from_string("4e-100", 256).unwrap();
        let reference_width = BigFloat::with_precision(4.0, 256);
        let result = calculate_max_iterations(&viewport_width, &reference_width);
        assert_eq!(result, 10000);
    }
```

**Step 2: Run tests to verify they fail**

```bash
cargo test --package fractalwonder-core max_iterations -- --nocapture
```

Expected: FAIL - `calculate_max_iterations` not found.

**Step 3: Implement calculate_max_iterations**

Add before the `#[cfg(test)]` section in `fractalwonder-core/src/transforms.rs`:

```rust
/// Calculate maximum iterations based on viewport width relative to reference width.
///
/// Takes the current viewport width and reference (default) width as BigFloat.
/// Internally computes zoom level and derives appropriate iteration count.
/// Deeper zoom requires more iterations to resolve fine detail at the boundary.
pub fn calculate_max_iterations(viewport_width: &BigFloat, reference_width: &BigFloat) -> u32 {
    // zoom = reference_width / viewport_width
    // log2(zoom) = log2(reference_width) - log2(viewport_width)
    let log2_zoom = reference_width.log2_approx() - viewport_width.log2_approx();

    // Convert log2 to log10 for the iteration formula
    let log10_zoom = log2_zoom / std::f64::consts::LOG2_10;

    let base = 50.0;
    let k = 100.0;
    let power = 1.5;

    let iterations = base + k * log10_zoom.max(0.0).powf(power);
    iterations.clamp(50.0, 10000.0) as u32
}
```

**Step 4: Run tests to verify they pass**

```bash
cargo test --package fractalwonder-core max_iterations -- --nocapture
```

Expected: All 5 tests PASS.

**Step 5: Export from lib.rs**

Update `fractalwonder-core/src/lib.rs`:

```rust
pub use transforms::{
    apply_pixel_transform_to_viewport, calculate_aspect_ratio, calculate_max_iterations,
    compose_affine_transformations, fit_viewport_to_canvas, fractal_to_pixel, pixel_to_fractal,
    AffinePrimitive, PixelMat3, PixelTransform,
};
```

**Step 6: Run all checks**

```bash
cargo check --workspace --all-targets --all-features
cargo test --workspace -- --nocapture
```

Expected: All pass.

**Step 7: Commit**

```bash
git add fractalwonder-core/src/transforms.rs fractalwonder-core/src/lib.rs
git commit -m "feat(core): add calculate_max_iterations function"
```

---

## Task 3: Create MandelbrotRenderer

**Files:**
- Create: `fractalwonder-compute/src/mandelbrot.rs`
- Modify: `fractalwonder-compute/src/lib.rs`

**Step 1: Create mandelbrot.rs with tests first**

Create `fractalwonder-compute/src/mandelbrot.rs`:

```rust
use crate::Renderer;
use fractalwonder_core::{pixel_to_fractal, BigFloat, MandelbrotData, Viewport};

/// Mandelbrot set renderer using escape-time algorithm.
///
/// All fractal-space math uses BigFloat for arbitrary precision.
pub struct MandelbrotRenderer {
    max_iterations: u32,
}

impl MandelbrotRenderer {
    pub fn new(max_iterations: u32) -> Self {
        Self { max_iterations }
    }
}

impl Renderer for MandelbrotRenderer {
    type Data = MandelbrotData;

    fn render(&self, viewport: &Viewport, canvas_size: (u32, u32)) -> Vec<MandelbrotData> {
        let (width, height) = canvas_size;
        let precision = viewport.precision_bits();

        (0..height)
            .flat_map(|py| {
                (0..width).map(move |px| {
                    let (cx, cy) = pixel_to_fractal(
                        px as f64,
                        py as f64,
                        viewport,
                        canvas_size,
                        precision,
                    );
                    self.compute_point(cx, cy, precision)
                })
            })
            .collect()
    }
}

impl MandelbrotRenderer {
    /// Compute Mandelbrot iteration for a single point using BigFloat arithmetic.
    fn compute_point(&self, cx: BigFloat, cy: BigFloat, precision: usize) -> MandelbrotData {
        let mut zx = BigFloat::zero(precision);
        let mut zy = BigFloat::zero(precision);
        let four = BigFloat::with_precision(4.0, precision);
        let two = BigFloat::with_precision(2.0, precision);

        for i in 0..self.max_iterations {
            let zx_sq = zx.mul(&zx);
            let zy_sq = zy.mul(&zy);

            // Escape check: |z|^2 > 4
            if zx_sq.add(&zy_sq).gt(&four) {
                return MandelbrotData {
                    iterations: i,
                    max_iterations: self.max_iterations,
                    escaped: true,
                };
            }

            // z = z^2 + c
            // new_zx = zx^2 - zy^2 + cx
            // new_zy = 2*zx*zy + cy
            let new_zx = zx_sq.sub(&zy_sq).add(&cx);
            let new_zy = two.mul(&zx).mul(&zy).add(&cy);
            zx = new_zx;
            zy = new_zy;
        }

        MandelbrotData {
            iterations: self.max_iterations,
            max_iterations: self.max_iterations,
            escaped: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_produces_correct_size() {
        let renderer = MandelbrotRenderer::new(100);
        let vp = Viewport::from_f64(-0.5, 0.0, 4.0, 4.0, 128);
        let result = renderer.render(&vp, (100, 50));
        assert_eq!(result.len(), 100 * 50);
    }

    #[test]
    fn origin_is_in_set() {
        // Point (0, 0) is in the Mandelbrot set
        let renderer = MandelbrotRenderer::new(100);
        let precision = 128;
        let cx = BigFloat::zero(precision);
        let cy = BigFloat::zero(precision);
        let result = renderer.compute_point(cx, cy, precision);
        assert!(!result.escaped, "Origin should be in set");
        assert_eq!(result.iterations, 100);
        assert_eq!(result.max_iterations, 100);
    }

    #[test]
    fn point_outside_escapes_immediately() {
        // Point (2, 0) escapes immediately: z1 = 4 + 0i, |z1|^2 = 16 > 4
        let renderer = MandelbrotRenderer::new(100);
        let precision = 128;
        let cx = BigFloat::with_precision(2.0, precision);
        let cy = BigFloat::zero(precision);
        let result = renderer.compute_point(cx, cy, precision);
        assert!(result.escaped, "Point (2,0) should escape");
        assert_eq!(result.iterations, 1, "Should escape after 1 iteration");
    }

    #[test]
    fn point_far_outside_escapes_at_zero() {
        // Point (10, 0): |c|^2 = 100 > 4, escapes at iteration 0
        let renderer = MandelbrotRenderer::new(100);
        let precision = 128;
        let cx = BigFloat::with_precision(10.0, precision);
        let cy = BigFloat::zero(precision);
        let result = renderer.compute_point(cx, cy, precision);
        assert!(result.escaped);
        // First iteration: z = 0, then check |z|^2 = 0 < 4, then z = c
        // Second check: |c|^2 = 100 > 4 -> escape at i=1
        assert_eq!(result.iterations, 1);
    }

    #[test]
    fn point_on_boundary_high_iterations() {
        // Point (-0.75, 0.1) is near the boundary, should take many iterations
        let renderer = MandelbrotRenderer::new(1000);
        let precision = 128;
        let cx = BigFloat::with_precision(-0.75, precision);
        let cy = BigFloat::with_precision(0.1, precision);
        let result = renderer.compute_point(cx, cy, precision);
        // This point eventually escapes but takes many iterations
        assert!(result.escaped);
        assert!(result.iterations > 10, "Boundary point should take many iterations");
    }

    #[test]
    fn main_cardioid_point_in_set() {
        // Point (-0.5, 0) is in the main cardioid
        let renderer = MandelbrotRenderer::new(500);
        let precision = 128;
        let cx = BigFloat::with_precision(-0.5, precision);
        let cy = BigFloat::zero(precision);
        let result = renderer.compute_point(cx, cy, precision);
        assert!(!result.escaped, "Point (-0.5, 0) should be in set");
    }

    #[test]
    fn max_iterations_stored_in_result() {
        let renderer = MandelbrotRenderer::new(500);
        let precision = 128;
        let cx = BigFloat::zero(precision);
        let cy = BigFloat::zero(precision);
        let result = renderer.compute_point(cx, cy, precision);
        assert_eq!(result.max_iterations, 500);
    }
}
```

**Step 2: Update lib.rs to include the module**

In `fractalwonder-compute/src/lib.rs`, add:

```rust
mod mandelbrot;
mod test_image;

use fractalwonder_core::Viewport;

pub use mandelbrot::MandelbrotRenderer;
pub use test_image::TestImageRenderer;

/// Renders a viewport to a grid of computed data.
pub trait Renderer {
    type Data;

    /// Render the given viewport at the specified canvas resolution.
    /// Returns a row-major Vec of pixel data (width * height elements).
    fn render(&self, viewport: &Viewport, canvas_size: (u32, u32)) -> Vec<Self::Data>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use fractalwonder_core::Viewport;

    #[test]
    fn test_image_renderer_produces_correct_size() {
        let renderer = TestImageRenderer;
        let vp = Viewport::from_f64(0.0, 0.0, 4.0, 4.0, 128);
        let result = renderer.render(&vp, (100, 50));
        assert_eq!(result.len(), 100 * 50);
    }

    #[test]
    fn test_image_renderer_origin_detected() {
        let renderer = TestImageRenderer;
        let vp = Viewport::from_f64(0.0, 0.0, 4.0, 4.0, 128);
        let result = renderer.render(&vp, (100, 100));
        let center_idx = 50 * 100 + 50;
        assert!(result[center_idx].is_on_origin);
    }
}
```

**Step 3: Run tests**

```bash
cargo test --package fractalwonder-compute -- --nocapture
```

Expected: All tests PASS.

**Step 4: Run all workspace checks**

```bash
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace -- --nocapture
```

Expected: All pass.

**Step 5: Commit**

```bash
git add fractalwonder-compute/src/mandelbrot.rs fractalwonder-compute/src/lib.rs
git commit -m "feat(compute): add MandelbrotRenderer with BigFloat arithmetic"
```

---

## Task 4: Add Mandelbrot Colorizer

**Files:**
- Create: `fractalwonder-ui/src/rendering/colorizers/mandelbrot.rs`
- Modify: `fractalwonder-ui/src/rendering/colorizers/mod.rs`

**Step 1: Create the colorizer**

Create `fractalwonder-ui/src/rendering/colorizers/mandelbrot.rs`:

```rust
use fractalwonder_core::MandelbrotData;

/// Grayscale colorizer for Mandelbrot data.
///
/// Points in the set (escaped=false) are black.
/// Escaped points get grayscale based on normalized iteration count.
pub fn colorize(data: &MandelbrotData) -> [u8; 4] {
    if !data.escaped {
        // In the set = black
        return [0, 0, 0, 255];
    }

    // Normalize iterations to 0.0..1.0
    let normalized = data.iterations as f64 / data.max_iterations as f64;
    let gray = (normalized * 255.0) as u8;

    [gray, gray, gray, 255]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn in_set_is_black() {
        let data = MandelbrotData {
            iterations: 1000,
            max_iterations: 1000,
            escaped: false,
        };
        assert_eq!(colorize(&data), [0, 0, 0, 255]);
    }

    #[test]
    fn escaped_at_zero_is_black() {
        let data = MandelbrotData {
            iterations: 0,
            max_iterations: 1000,
            escaped: true,
        };
        assert_eq!(colorize(&data), [0, 0, 0, 255]);
    }

    #[test]
    fn escaped_at_max_is_white() {
        let data = MandelbrotData {
            iterations: 1000,
            max_iterations: 1000,
            escaped: true,
        };
        assert_eq!(colorize(&data), [255, 255, 255, 255]);
    }

    #[test]
    fn escaped_halfway_is_gray() {
        let data = MandelbrotData {
            iterations: 500,
            max_iterations: 1000,
            escaped: true,
        };
        let result = colorize(&data);
        // 500/1000 * 255 = 127.5 -> 127
        assert_eq!(result, [127, 127, 127, 255]);
    }
}
```

**Step 2: Update mod.rs**

Update `fractalwonder-ui/src/rendering/colorizers/mod.rs`:

```rust
pub mod mandelbrot;
pub mod test_image;

pub use mandelbrot::colorize as colorize_mandelbrot;
pub use test_image::colorize as colorize_test_image;
```

**Step 3: Run tests**

```bash
cargo test --package fractalwonder-ui colorizers -- --nocapture
```

Expected: All tests PASS.

**Step 4: Commit**

```bash
git add fractalwonder-ui/src/rendering/colorizers/mandelbrot.rs fractalwonder-ui/src/rendering/colorizers/mod.rs
git commit -m "feat(ui): add grayscale Mandelbrot colorizer"
```

---

## Task 5: Port DropdownMenu Component

**Files:**
- Create: `fractalwonder-ui/src/components/dropdown_menu.rs`
- Modify: `fractalwonder-ui/src/components/mod.rs`

**Step 1: Create dropdown_menu.rs**

Create `fractalwonder-ui/src/components/dropdown_menu.rs` (port from archive):

```rust
use leptos::*;

#[component]
pub fn DropdownMenu<F>(
    label: String,
    options: Signal<Vec<(String, String)>>, // (id, display_name)
    selected_id: Signal<String>,
    on_select: F,
) -> impl IntoView
where
    F: Fn(String) + 'static + Copy,
{
    let (is_open, set_is_open) = create_signal(false);

    view! {
        <div class="relative">
            <button
                class="text-white hover:text-gray-200 hover:bg-white/10 rounded-lg px-3 py-2 transition-colors flex items-center gap-2"
                on:click=move |_| set_is_open.update(|v| *v = !*v)
            >
                <span class="text-sm">{label.clone()}</span>
                <span class="text-xs opacity-70">"▾"</span>
            </button>

            {move || is_open.get().then(|| view! {
                <div class="absolute bottom-full mb-2 left-0 min-w-40 bg-black/70 backdrop-blur-sm border border-gray-800 rounded-lg overflow-hidden">
                    <For
                        each=move || options.get()
                        key=|(id, _)| id.clone()
                        children=move |(id, name)| {
                            let id_for_selected = id.clone();
                            let id_for_click = id.clone();
                            let is_selected = move || selected_id.get() == id_for_selected;
                            view! {
                                <button
                                    class=move || format!(
                                        "w-full text-left px-4 py-2 text-sm transition-colors {}",
                                        if is_selected() {
                                            "bg-white/20 text-white"
                                        } else {
                                            "text-gray-300 hover:bg-white/10 hover:text-white"
                                        }
                                    )
                                    on:click=move |_| {
                                        on_select(id_for_click.clone());
                                        set_is_open.set(false);
                                    }
                                >
                                    {name}
                                </button>
                            }
                        }
                    />
                </div>
            })}
        </div>
    }
}
```

**Step 2: Update mod.rs**

Update `fractalwonder-ui/src/components/mod.rs` to include and export:

```rust
mod dropdown_menu;
mod fullscreen_button;
mod home_button;
mod info_button;
mod interactive_canvas;
mod ui_panel;

pub use dropdown_menu::DropdownMenu;
pub use fullscreen_button::FullscreenButton;
pub use home_button::HomeButton;
pub use info_button::InfoButton;
pub use interactive_canvas::InteractiveCanvas;
pub use ui_panel::UIPanel;
```

**Step 3: Run checks**

```bash
cargo check --package fractalwonder-ui --all-targets
```

Expected: Compiles successfully.

**Step 4: Commit**

```bash
git add fractalwonder-ui/src/components/dropdown_menu.rs fractalwonder-ui/src/components/mod.rs
git commit -m "feat(ui): port DropdownMenu component from archive"
```

---

## Task 6: Update UIPanel with Dropdowns

**Files:**
- Modify: `fractalwonder-ui/src/components/ui_panel.rs`

**Step 1: Add new props to UIPanel**

Update the `UIPanel` component signature and add dropdowns. The file is long, so here are the specific changes:

**Add imports at top:**
```rust
use crate::components::DropdownMenu;
```

**Update component signature:**
```rust
#[component]
pub fn UIPanel(
    /// Canvas dimensions (width, height)
    canvas_size: Signal<(u32, u32)>,
    /// Current viewport in fractal space
    viewport: Signal<Viewport>,
    /// Current fractal configuration
    config: Signal<&'static FractalConfig>,
    /// Calculated precision bits
    precision_bits: Signal<usize>,
    /// Callback when home button is clicked
    on_home_click: Callback<()>,
    /// Renderer selection options (id, display_name)
    renderer_options: Signal<Vec<(String, String)>>,
    /// Currently selected renderer ID
    selected_renderer_id: Signal<String>,
    /// Callback when renderer is selected
    on_renderer_select: Callback<String>,
    /// Colorizer selection options (id, display_name)
    colorizer_options: Signal<Vec<(String, String)>>,
    /// Currently selected colorizer ID
    selected_colorizer_id: Signal<String>,
    /// Callback when colorizer is selected
    on_colorizer_select: Callback<String>,
) -> impl IntoView {
```

**Update the left section in the view (around line 62-65):**

Replace:
```rust
                // Left section: info button and home button
                <div class="flex items-center space-x-2">
                    <InfoButton is_open=is_info_open set_is_open=set_is_info_open />
                    <HomeButton on_click=on_home_click />
                </div>
```

With:
```rust
                // Left section: info button, home button, and dropdowns
                <div class="flex items-center space-x-2">
                    <InfoButton is_open=is_info_open set_is_open=set_is_info_open />
                    <HomeButton on_click=on_home_click />
                    <DropdownMenu
                        label="Function".to_string()
                        options=renderer_options
                        selected_id=selected_renderer_id
                        on_select=move |id| on_renderer_select.call(id)
                    />
                    <DropdownMenu
                        label="Colors".to_string()
                        options=colorizer_options
                        selected_id=selected_colorizer_id
                        on_select=move |id| on_colorizer_select.call(id)
                    />
                </div>
```

**Step 2: Run checks**

```bash
cargo check --package fractalwonder-ui --all-targets
```

Expected: Will fail because App.rs doesn't pass the new props yet. That's expected - we'll fix it in Task 8.

**Step 3: Commit (partial - UIPanel updated)**

```bash
git add fractalwonder-ui/src/components/ui_panel.rs
git commit -m "feat(ui): add dropdown props to UIPanel"
```

---

## Task 7: Update InteractiveCanvas with Config-Based Dispatch

**Files:**
- Modify: `fractalwonder-ui/src/components/interactive_canvas.rs`

**Step 1: Update imports**

Add at the top:
```rust
use fractalwonder_compute::{MandelbrotRenderer, Renderer, TestImageRenderer};
use fractalwonder_core::{apply_pixel_transform_to_viewport, calculate_max_iterations, Viewport};
use crate::config::FractalConfig;
use crate::rendering::{colorize_mandelbrot, colorize_test_image};
```

Remove old imports that are now covered:
```rust
// Remove these if present:
// use fractalwonder_compute::{Renderer, TestImageRenderer};
// use crate::rendering::colorize_test_image;
```

**Step 2: Update component signature**

Change the component to accept config:

```rust
#[component]
pub fn InteractiveCanvas(
    /// Current viewport in fractal space (read-only)
    viewport: Signal<Viewport>,
    /// Callback fired when user interaction ends with a new viewport
    on_viewport_change: Callback<Viewport>,
    /// Current fractal configuration
    config: Signal<&'static FractalConfig>,
    /// Callback fired when canvas dimensions change
    #[prop(optional)]
    on_resize: Option<Callback<(u32, u32)>>,
) -> impl IntoView {
```

**Step 3: Update the render effect**

Replace the render effect (the `create_effect` that does rendering, around line 73-115) with:

```rust
    // Render effect - redraws when viewport or config changes
    create_effect(move |_| {
        let vp = viewport.get();
        let cfg = config.get();
        let size = canvas_size.get();

        if size.0 == 0 || size.1 == 0 {
            return;
        }

        let Some(canvas_el) = canvas_ref.get() else {
            return;
        };
        let canvas = canvas_el.unchecked_ref::<HtmlCanvasElement>();

        let ctx = canvas
            .get_context("2d")
            .unwrap()
            .unwrap()
            .unchecked_into::<CanvasRenderingContext2d>();

        let (width, height) = size;

        // Dispatch to renderer based on config.id
        let mut data = vec![0u8; (width * height * 4) as usize];

        match cfg.id {
            "test_image" => {
                let renderer = TestImageRenderer;
                let computed = renderer.render(&vp, size);
                for (i, pixel_data) in computed.iter().enumerate() {
                    let color = colorize_test_image(pixel_data);
                    let idx = i * 4;
                    data[idx] = color[0];
                    data[idx + 1] = color[1];
                    data[idx + 2] = color[2];
                    data[idx + 3] = color[3];
                }
            }
            "mandelbrot" => {
                let reference_width = cfg.default_viewport(vp.precision_bits()).width;
                let max_iters = calculate_max_iterations(&vp.width, &reference_width);
                let renderer = MandelbrotRenderer::new(max_iters);
                let computed = renderer.render(&vp, size);
                for (i, pixel_data) in computed.iter().enumerate() {
                    let color = colorize_mandelbrot(pixel_data);
                    let idx = i * 4;
                    data[idx] = color[0];
                    data[idx + 1] = color[1];
                    data[idx + 2] = color[2];
                    data[idx + 3] = color[3];
                }
            }
            _ => {
                // Unknown renderer - fill with magenta for visibility
                for i in 0..(width * height) as usize {
                    let idx = i * 4;
                    data[idx] = 255;
                    data[idx + 1] = 0;
                    data[idx + 2] = 255;
                    data[idx + 3] = 255;
                }
            }
        }

        // Draw to canvas
        let image_data = ImageData::new_with_u8_clamped_array_and_sh(Clamped(&data), width, height)
            .expect("should create ImageData");
        ctx.put_image_data(&image_data, 0.0, 0.0)
            .expect("should put image data");
    });
```

**Step 4: Run checks**

```bash
cargo check --package fractalwonder-ui --all-targets
```

Expected: Will fail because App.rs doesn't pass config yet. That's expected.

**Step 5: Commit**

```bash
git add fractalwonder-ui/src/components/interactive_canvas.rs
git commit -m "feat(ui): add config-based renderer dispatch to InteractiveCanvas"
```

---

## Task 8: Update App with State Management

**Files:**
- Modify: `fractalwonder-ui/src/app.rs`

**Step 1: Read current app.rs**

First, read the current file to understand its structure, then apply the necessary changes.

**Step 2: Add imports**

Ensure these imports are present:
```rust
use crate::config::{default_config, get_config, FractalConfig, FRACTAL_CONFIGS};
```

**Step 3: Add state signals**

After existing state signals, add:
```rust
    // Selected renderer (fractal type)
    let (selected_config_id, set_selected_config_id) = create_signal("mandelbrot".to_string());

    // Derive config from selected ID
    let config = create_memo(move |_| {
        get_config(&selected_config_id.get()).unwrap_or_else(default_config)
    });

    // Build renderer options from FRACTAL_CONFIGS
    let renderer_options = Signal::derive(move || {
        FRACTAL_CONFIGS
            .iter()
            .map(|c| (c.id.to_string(), c.display_name.to_string()))
            .collect::<Vec<_>>()
    });

    // Colorizer options (single default for now)
    let colorizer_options = Signal::derive(move || {
        vec![("default".to_string(), "Default".to_string())]
    });
    let selected_colorizer_id = Signal::derive(move || "default".to_string());
```

**Step 4: Add effect to reset viewport on config change**

```rust
    // Reset viewport when config changes
    create_effect(move |prev_id: Option<String>| {
        let current_id = selected_config_id.get();
        if let Some(prev) = prev_id {
            if prev != current_id {
                // Config changed - reset to default viewport
                if let Some(cfg) = get_config(&current_id) {
                    let size = canvas_size.get();
                    if size.0 > 0 && size.1 > 0 {
                        let precision = calculate_precision_bits(
                            &cfg.default_viewport(128),
                            size,
                        );
                        let natural_vp = cfg.default_viewport(precision);
                        let fitted_vp = fit_viewport_to_canvas(&natural_vp, size);
                        set_viewport.set(fitted_vp);
                    }
                }
            }
        }
        current_id
    });
```

**Step 5: Update InteractiveCanvas usage**

Pass the config prop:
```rust
<InteractiveCanvas
    viewport=viewport.into()
    on_viewport_change=Callback::new(move |vp| set_viewport.set(vp))
    config=config.into()
    on_resize=Callback::new(move |size| {
        set_canvas_size.set(size);
        // ... existing resize logic ...
    })
/>
```

**Step 6: Update UIPanel usage**

Pass all the new props:
```rust
<UIPanel
    canvas_size=canvas_size.into()
    viewport=viewport.into()
    config=config.into()
    precision_bits=precision_bits.into()
    on_home_click=Callback::new(move |_| {
        // ... existing home logic ...
    })
    renderer_options=renderer_options
    selected_renderer_id=Signal::derive(move || selected_config_id.get())
    on_renderer_select=Callback::new(move |id: String| set_selected_config_id.set(id))
    colorizer_options=colorizer_options
    selected_colorizer_id=selected_colorizer_id
    on_colorizer_select=Callback::new(move |_: String| {
        // No-op for now - colorizer selection in Iteration 8
    })
/>
```

**Step 7: Run all checks**

```bash
cargo fmt --all
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo check --workspace --all-targets --all-features
cargo test --workspace -- --nocapture
```

Expected: All pass.

**Step 8: Commit**

```bash
git add fractalwonder-ui/src/app.rs
git commit -m "feat(ui): add renderer selection state management to App"
```

---

## Task 9: Browser Testing

**Step 1: Ensure trunk is running**

Verify `trunk serve` is running on http://localhost:8080

**Step 2: Manual browser tests**

1. Open http://localhost:8080
2. Should see Mandelbrot set (black interior, gradient exterior)
3. Click "Function" dropdown, select "Test Pattern"
4. Should see checkerboard with axes
5. Select "Mandelbrot Set" again
6. Pan and zoom - should work (slowly, main thread)
7. Verify viewport resets when switching fractal types

**Step 3: Fix any issues found**

If issues are found, debug and fix before final commit.

---

## Task 10: Final Verification and Commit

**Step 1: Run all quality checks**

```bash
cargo fmt --all
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo check --workspace --all-targets --all-features
cargo test --workspace -- --nocapture
```

**Step 2: Review changes**

```bash
git status
git diff --stat HEAD~8
```

**Step 3: Tag iteration complete (optional)**

```bash
git tag -a iteration-7-complete -m "Iteration 7: MandelbrotRenderer complete"
```

---

## Summary

| Task | Description | Files |
|------|-------------|-------|
| 1 | Add MandelbrotData | core/compute_data.rs, core/lib.rs |
| 2 | Add calculate_max_iterations | core/transforms.rs, core/lib.rs |
| 3 | Create MandelbrotRenderer | compute/mandelbrot.rs, compute/lib.rs |
| 4 | Add Mandelbrot colorizer | ui/colorizers/mandelbrot.rs, ui/colorizers/mod.rs |
| 5 | Port DropdownMenu | ui/components/dropdown_menu.rs, ui/components/mod.rs |
| 6 | Update UIPanel | ui/components/ui_panel.rs |
| 7 | Update InteractiveCanvas | ui/components/interactive_canvas.rs |
| 8 | Update App | ui/app.rs |
| 9 | Browser testing | Manual verification |
| 10 | Final verification | Quality checks |
