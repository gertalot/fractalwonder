# Iteration 7: MandelbrotRenderer Design

## Goal

Add Mandelbrot computation following the Renderer trait, with UI selection between TestImage and Mandelbrot.

## Key Design Decisions

1. **All fractal math uses BigFloat** - No f64 anywhere except final colorization
2. **MandelbrotData is self-describing** - Includes max_iterations for colorizer normalization
3. **calculate_max_iterations hides internals** - Takes BigFloats, handles log2 conversion internally
4. **Simple match dispatch** - InteractiveCanvas matches on config.id (no trait objects)
5. **Port dropdown infrastructure from archive** - Reuse existing DropdownMenu component

---

## Data Types

### MandelbrotData (fractalwonder-core/src/compute_data.rs)

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

Add `Mandelbrot(MandelbrotData)` variant to `ComputeData` enum.

### calculate_max_iterations (fractalwonder-core/src/transforms.rs)

```rust
/// Calculate maximum iterations based on viewport width relative to reference width.
///
/// Takes the current viewport width and reference (default) width as BigFloat.
/// Internally computes zoom level and derives appropriate iteration count.
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

---

## MandelbrotRenderer (fractalwonder-compute/src/mandelbrot.rs)

```rust
use crate::Renderer;
use fractalwonder_core::{pixel_to_fractal, BigFloat, MandelbrotData, Viewport};

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
                    // Convert pixel to fractal coordinates (BigFloat)
                    let (cx, cy) = pixel_to_fractal(
                        px as f64,
                        py as f64,
                        viewport,
                        canvas_size,
                        precision,
                    );

                    // Run escape-time iteration with BigFloat arithmetic
                    self.compute_point(cx, cy, precision)
                })
            })
            .collect()
    }
}

impl MandelbrotRenderer {
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
```

---

## Colorizer (fractalwonder-ui/src/rendering/colorizers/mandelbrot.rs)

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
```

Update `colorizers/mod.rs`:
```rust
pub mod test_image;
pub mod mandelbrot;

pub use test_image::colorize as colorize_test_image;
pub use mandelbrot::colorize as colorize_mandelbrot;
```

---

## UI Components

### DropdownMenu (fractalwonder-ui/src/components/dropdown_menu.rs)

Port from `_archive/fractalwonder-ui/src/components/dropdown_menu.rs`:

```rust
#[component]
pub fn DropdownMenu<F>(
    label: String,
    options: Signal<Vec<(String, String)>>, // (id, display_name)
    selected_id: Signal<String>,
    on_select: F,
) -> impl IntoView
where
    F: Fn(String) + 'static + Copy,
{ /* ... port from archive ... */ }
```

### UIPanel Updates

Add props for renderer and colorizer selection:

```rust
#[component]
pub fn UIPanel(
    // ... existing props ...

    // Renderer selection
    renderer_options: Signal<Vec<(String, String)>>,
    selected_renderer_id: Signal<String>,
    on_renderer_select: Callback<String>,

    // Colorizer selection
    colorizer_options: Signal<Vec<(String, String)>>,
    selected_colorizer_id: Signal<String>,
    on_colorizer_select: Callback<String>,
) -> impl IntoView {
    // Add dropdowns to left section:
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
}
```

---

## InteractiveCanvas - Renderer Dispatch

Update to accept config and dispatch based on `config.id`:

```rust
#[component]
pub fn InteractiveCanvas(
    viewport: Signal<Viewport>,
    on_viewport_change: Callback<Viewport>,
    config: Signal<&'static FractalConfig>,
    #[prop(optional)]
    on_resize: Option<Callback<(u32, u32)>>,
) -> impl IntoView {
    // In render effect:
    let pixel_data: Vec<u8> = match cfg.id {
        "test_image" => {
            let renderer = TestImageRenderer;
            let data = renderer.render(&vp, size);
            data.iter().flat_map(|d| colorize_test_image(d)).collect()
        }
        "mandelbrot" => {
            let reference_width = cfg.default_viewport(vp.precision_bits()).width;
            let max_iters = calculate_max_iterations(&vp.width, &reference_width);
            let renderer = MandelbrotRenderer::new(max_iters);
            let data = renderer.render(&vp, size);
            data.iter().flat_map(|d| colorize_mandelbrot(d)).collect()
        }
        _ => panic!("Unknown renderer: {}", cfg.id),
    };
}
```

---

## App - State Management

```rust
#[component]
pub fn App() -> impl IntoView {
    // Selected config (renderer) - default to Mandelbrot
    let (selected_config_id, set_selected_config_id) = create_signal("mandelbrot".to_string());

    // Derive config from ID
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

    // Colorizer options (single "default" for now - expand in Iteration 8)
    let colorizer_options = Signal::derive(move || {
        vec![("default".to_string(), "Default".to_string())]
    });
    let selected_colorizer_id = Signal::derive(|| "default".to_string());

    // Reset viewport when config changes
    create_effect(move |prev_id: Option<String>| {
        let current_id = selected_config_id.get();
        if prev_id.is_some() && prev_id.as_ref() != Some(&current_id) {
            let cfg = get_config(&current_id).unwrap();
            let precision = calculate_precision_bits(/* canvas_size, viewport */);
            set_viewport.set(cfg.default_viewport(precision));
        }
        current_id
    });

    // Wire everything together in view...
}
```

---

## File Changes Summary

| File | Action |
|------|--------|
| `fractalwonder-core/src/compute_data.rs` | Add `MandelbrotData`, add `Mandelbrot` variant to `ComputeData` |
| `fractalwonder-core/src/transforms.rs` | Add `calculate_max_iterations` |
| `fractalwonder-core/src/lib.rs` | Export new types/functions |
| `fractalwonder-compute/src/mandelbrot.rs` | NEW: `MandelbrotRenderer` |
| `fractalwonder-compute/src/lib.rs` | Add `mod mandelbrot`, export `MandelbrotRenderer` |
| `fractalwonder-ui/src/rendering/colorizers/mandelbrot.rs` | NEW: grayscale colorizer |
| `fractalwonder-ui/src/rendering/colorizers/mod.rs` | Export `colorize_mandelbrot` |
| `fractalwonder-ui/src/components/dropdown_menu.rs` | NEW: port from archive |
| `fractalwonder-ui/src/components/mod.rs` | Export `DropdownMenu` |
| `fractalwonder-ui/src/components/ui_panel.rs` | Add dropdown props and components |
| `fractalwonder-ui/src/components/interactive_canvas.rs` | Add config prop, match dispatch |
| `fractalwonder-ui/src/app.rs` | State management for renderer/colorizer selection |

---

## Tests

### Unit Tests

- `MandelbrotRenderer::render()` returns correct size
- Point (0, 0): max iterations, escaped=false (in set)
- Point (2, 0): few iterations, escaped=true (outside set)
- Point (-0.75, 0): high iterations (boundary)
- `calculate_max_iterations` returns expected values for known zoom levels
- Colorizer produces black for escaped=false, grayscale gradient for escaped=true

### Browser Tests

- Switch to Mandelbrot in UI, see the iconic Mandelbrot shape
- Black interior, gradient exterior
- Pan/zoom works (slowly - still main thread)
- Switch back to TestImage, see checkerboard
- Viewport resets when switching fractal types
