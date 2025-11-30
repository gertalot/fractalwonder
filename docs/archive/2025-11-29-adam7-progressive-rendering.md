# Adam7 Progressive Rendering

Replace the current resolution-based pass system (Preview16/8/4/Full) with Adam7 interlacing for meaningful visual progress during GPU rendering.

## Problem

The current 4-pass system (1/16, 1/8, 1/4, full resolution) completes the first three passes in ~300ms combined, providing little useful feedback. The final full-resolution pass takes the majority of time with no progress indication.

## Solution

Replace with Adam7 interlacing: 7 passes that compute different pixel subsets at full resolution. Each pass doubles the pixel count, providing 7 visual updates instead of 1 during the expensive full-resolution computation.

## Adam7 Pattern

```
Pass 1: 1/64 pixels (1.6%)   - every 8th pixel in 8x8 grid
Pass 2: +1/64 (3.1% total)
Pass 3: +2/64 (6.3% total)
Pass 4: +4/64 (12.5% total)
Pass 5: +8/64 (25% total)
Pass 6: +16/64 (50% total)
Pass 7: +32/64 (100% total)
```

The 8x8 matrix determines which pass each pixel belongs to:
```
1 6 4 6 2 6 4 6
7 7 7 7 7 7 7 7
5 6 5 6 5 6 5 6
7 7 7 7 7 7 7 7
3 6 4 6 3 6 4 6
7 7 7 7 7 7 7 7
5 6 5 6 5 6 5 6
7 7 7 7 7 7 7 7
```

## Architecture

### Render Flow

```
For each Adam7Pass 1..7:
    GPU dispatch (all pixels, shader skips non-matching)
    ↓
    Vec<ComputeData> with sentinel for skipped pixels
    ↓
    Merge valid results into Adam7Accumulator
    ↓
    Stretch: fill None gaps from left/top neighbor
    ↓
    Colorize & display
    ↓
    Browser repaint (double-rAF)
```

### GPU Shader

Add `adam7_step` uniform (0 = compute all, 1-7 = specific pass).

Early exit for non-matching pixels:
```wgsl
fn get_adam7_pass(x: u32, y: u32) -> u32 {
    let adam7_matrix = array<array<u32, 8>, 8>(
        array(1u, 6u, 4u, 6u, 2u, 6u, 4u, 6u),
        array(7u, 7u, 7u, 7u, 7u, 7u, 7u, 7u),
        array(5u, 6u, 5u, 6u, 5u, 6u, 5u, 6u),
        array(7u, 7u, 7u, 7u, 7u, 7u, 7u, 7u),
        array(3u, 6u, 4u, 6u, 3u, 6u, 4u, 6u),
        array(7u, 7u, 7u, 7u, 7u, 7u, 7u, 7u),
        array(5u, 6u, 5u, 6u, 5u, 6u, 5u, 6u),
        array(7u, 7u, 7u, 7u, 7u, 7u, 7u, 7u),
    );
    return adam7_matrix[y % 8u][x % 8u];
}

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    if id.x >= uniforms.width || id.y >= uniforms.height {
        return;
    }

    // Adam7 early exit (step 0 = compute all)
    if uniforms.adam7_step > 0u && get_adam7_pass(id.x, id.y) != uniforms.adam7_step {
        return;  // Don't write anything for skipped pixels
    }

    // ... existing perturbation iteration code ...
}
```

### Adam7Pass Type

Replace `Pass` enum entirely:

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Adam7Pass(u8);  // 1-7

impl Adam7Pass {
    pub fn all() -> [Adam7Pass; 7] {
        [1, 2, 3, 4, 5, 6, 7].map(Adam7Pass)
    }

    pub fn step(&self) -> u8 { self.0 }
    pub fn is_final(&self) -> bool { self.0 == 7 }

    pub fn cumulative_percent(&self) -> f32 {
        match self.0 {
            1 => 1.56, 2 => 3.13, 3 => 6.25, 4 => 12.5,
            5 => 25.0, 6 => 50.0, 7 => 100.0, _ => 0.0,
        }
    }
}
```

### Adam7Accumulator

Accumulates results across passes:

```rust
struct Adam7Accumulator {
    data: Vec<Option<ComputeData>>,
    width: u32,
    height: u32,
}

impl Adam7Accumulator {
    fn new(width: u32, height: u32) -> Self {
        Self {
            data: vec![None; (width * height) as usize],
            width,
            height,
        }
    }

    fn merge(&mut self, gpu_result: &[ComputeData]) {
        for (i, computed) in gpu_result.iter().enumerate() {
            if let ComputeData::Mandelbrot(m) = computed {
                // Sentinel value indicates "not computed this pass"
                if m.iterations != 0xFFFFFFFF {
                    self.data[i] = Some(computed.clone());
                }
            }
        }
    }

    fn to_stretched(&self) -> Vec<ComputeData> {
        // Fill None gaps from left/top neighbor
        let mut result = Vec::with_capacity(self.data.len());
        for (i, pixel) in self.data.iter().enumerate() {
            match pixel {
                Some(d) => result.push(d.clone()),
                None => {
                    let fallback_idx = if i % (self.width as usize) > 0 {
                        i - 1
                    } else {
                        i.saturating_sub(self.width as usize)
                    };
                    result.push(result.get(fallback_idx).cloned().unwrap_or_else(|| {
                        ComputeData::Mandelbrot(MandelbrotData {
                            iterations: 0,
                            max_iterations: 1,
                            escaped: false,
                            glitched: false,
                        })
                    }));
                }
            }
        }
        result
    }
}
```

## Files to Change

| File | Change |
|------|--------|
| `fractalwonder-gpu/src/pass.rs` | Replace `Pass` enum with `Adam7Pass` |
| `fractalwonder-gpu/src/lib.rs` | Update exports |
| `fractalwonder-gpu/src/renderer.rs` | Update `render()` to accept `Adam7Pass`, add `adam7_step` to uniforms |
| `fractalwonder-gpu/src/buffers.rs` | Add `adam7_step` to `Uniforms` struct |
| `fractalwonder-gpu/src/pipeline.rs` | Update shader if embedded |
| Shader file (`.wgsl`) | Add Adam7 matrix + early exit logic |
| `fractalwonder-gpu/src/stretch.rs` | Replace with Adam7 gap-filling stretch |
| `fractalwonder-ui/src/rendering/parallel_renderer.rs` | Replace pass scheduling, add `Adam7Accumulator` |
| `fractalwonder-ui/src/rendering/render_progress.rs` | Update for 7 passes |

## Deleted Code

- `Pass::Preview16`, `Preview8`, `Preview4`, `Full` variants
- `Pass::scale()`, `Pass::dimensions()`, `Pass::max_iterations()`
- Old `stretch_compute_data()` function
- Resolution-based dimension calculations

## Unchanged

- `ComputeData` structure
- Colorization pipeline
- Recolorize from cache
- Orbit computation
- Reference orbit calculation
