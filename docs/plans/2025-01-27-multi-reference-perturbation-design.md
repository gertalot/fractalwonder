# Multi-Reference Perturbation Rendering

## Problem

At zoom levels beyond 10^17, the current single-reference perturbation implementation produces visible "smeared arc" artifacts. Root cause: when perturbation breaks down (delta grows too large relative to reference orbit), the on-the-fly fallback uses a truncated f64 reference point, causing geometric errors.

## Solution

Replace single-reference perturbation with adaptive multi-reference system using quadtree subdivision. When perturbation breaks down for a pixel, mark it as "glitched" and re-render with a closer reference point.

## Design Decisions

| Aspect | Choice |
|--------|--------|
| Glitch detection | Rebase condition triggers `glitched = true` |
| Reference placement | Quadtree cell centers |
| Tile/quadtree relationship | Independent; tiles query containing cell |
| Subdivision | Adaptive quadtree, cells with glitches split into 4 |
| Termination | Zero glitches OR min cell size OR max depth |
| Orbit computation | Lazy/on-demand |
| Re-render granularity | Entire tiles |
| Fallback | BigFloat direct computation |

## Parameters

```rust
const MAX_QUADTREE_DEPTH: u32 = 6;      // Max 4^6 = 4096 cells
const MIN_CELL_SIZE: u32 = 64;          // Don't subdivide below 64x64 pixels
const TILE_SIZE: u32 = 64;              // Existing tile size
```

## Data Structures

### Modified MandelbrotData

```rust
struct MandelbrotData {
    iterations: u32,
    max_iterations: u32,
    escaped: bool,
    glitched: bool,  // NEW: pixel needs different reference
}
```

### Quadtree Cell

```rust
struct QuadtreeCell {
    /// Bounding box in pixel coordinates
    bounds: PixelRect,
    /// Reference orbit for this cell (None until computed)
    orbit_id: Option<u32>,
    /// Children if subdivided (None = leaf)
    children: Option<Box<[QuadtreeCell; 4]>>,
    /// Tiles within this cell that had glitches
    glitched_tiles: HashSet<TileId>,
}
```

### Reference Orbit

```rust
struct ReferenceOrbit {
    orbit_id: u32,
    /// Cell center in fractal space (BigFloat for precision)
    center: (BigFloat, BigFloat),
    /// Precomputed orbit values (f64, bounded by escape radius)
    orbit: Vec<(f64, f64)>,
    /// When reference escaped (None = never)
    escaped_at: Option<u32>,
}
```

## Message Protocol Changes

### Modified Messages

```rust
// Workers cache multiple orbits, receive BigFloat c_ref
MainToWorker::StoreReferenceOrbit {
    orbit_id: u32,
    c_ref_json: String,      // BigFloat JSON, not f64
    orbit: Vec<(f64, f64)>,
    escaped_at: Option<u32>,
}

// Tile specifies which orbit to use
MainToWorker::RenderTilePerturbation {
    render_id: u32,
    tile: PixelRect,
    orbit_id: u32,
    c_ref_json: String,      // Full precision for potential fallback
    delta_c_origin: (f64, f64),
    delta_c_step: (f64, f64),
    max_iterations: u32,
}
```

### New Message

```rust
// BigFloat fallback for remaining glitched pixels
MainToWorker::RenderPixelsDirect {
    render_id: u32,
    tile: PixelRect,
    pixels: Vec<(u32, u32)>,  // Pixel offsets within tile
    viewport_json: String,     // Full BigFloat viewport
    max_iterations: u32,
}
```

## Rendering Flow

```
1. Initialize quadtree with single root cell (full viewport)
   Compute root reference orbit at viewport center

2. Render pass: dispatch all tiles
   Each tile uses orbit from containing quadtree cell
   Workers return: pixel data + glitched flags

3. Collect glitches per cell
   Cells with glitched tiles -> subdivide into 4 children
   Compute reference orbit for each new child cell

4. If glitched tiles remain:
   - If limits not hit: loop back to step 2 (re-render glitched tiles only)
   - If limits hit: BigFloat fallback for remaining pixels

5. Done
```

## Worker Perturbation Logic

Key change: remove on-the-fly fallback, just mark as glitched.

```rust
pub fn compute_pixel_perturbation(
    orbit: &ReferenceOrbit,
    delta_c: (f64, f64),
    max_iterations: u32,
) -> MandelbrotData {
    let mut dx = 0.0;
    let mut dy = 0.0;

    for n in 0..max_iterations {
        let (xn, yn) = if n < orbit.orbit.len() && n < orbit.escaped_at {
            orbit.orbit[n]
        } else {
            // Reference ended - mark as glitched
            return MandelbrotData {
                iterations: n,
                escaped: false,
                glitched: true,
                max_iterations
            };
        };

        // Escape check
        let zx = xn + dx;
        let zy = yn + dy;
        if zx * zx + zy * zy > 4.0 {
            return MandelbrotData {
                iterations: n,
                escaped: true,
                glitched: false,
                max_iterations
            };
        }

        // Rebase check - mark glitched instead of fallback
        let delta_mag_sq = dx * dx + dy * dy;
        let x_mag_sq = xn * xn + yn * yn;
        if delta_mag_sq > 0.25 * x_mag_sq && x_mag_sq > 1e-20 {
            return MandelbrotData {
                iterations: n,
                escaped: false,
                glitched: true,
                max_iterations
            };
        }

        // Delta iteration
        let new_dx = 2.0 * (xn * dx - yn * dy) + dx * dx - dy * dy + delta_c.0;
        let new_dy = 2.0 * (xn * dy + yn * dx) + 2.0 * dx * dy + delta_c.1;
        dx = new_dx;
        dy = new_dy;
    }

    MandelbrotData {
        iterations: max_iterations,
        escaped: false,
        glitched: false,
        max_iterations
    }
}
```

## BigFloat Fallback

For pixels that still glitch after hitting quadtree limits:

```rust
fn handle_render_pixels_direct(
    tile: PixelRect,
    pixels: Vec<(u32, u32)>,
    viewport_json: String,
    max_iterations: u32,
) -> Vec<(u32, u32, MandelbrotData)> {
    let viewport: Viewport = serde_json::from_str(&viewport_json).unwrap();
    let precision = viewport.precision_bits();

    pixels.iter().map(|(px, py)| {
        let c = pixel_to_fractal(
            tile.x + px,
            tile.y + py,
            &viewport,
            canvas_size,
            precision,
        );

        let result = compute_mandelbrot_bigfloat(&c, max_iterations);

        (*px, *py, MandelbrotData {
            iterations: result.iterations,
            max_iterations,
            escaped: result.escaped,
            glitched: false,
        })
    }).collect()
}
```

## Expected Behavior

At zoom 10^1000:
- Root pass: many tiles glitch near fractal boundary
- 2-4 subdivision passes in complex regions
- Handful of pixels hit BigFloat fallback
- Smooth regions stay single-reference

## Implementation Order

1. Add `glitched` field to `MandelbrotData`
2. Modify perturbation worker to mark glitched instead of on-the-fly fallback
3. Add quadtree data structure to worker pool
4. Implement multi-pass rendering loop
5. Add `RenderPixelsDirect` message and BigFloat fallback
6. Test at increasing zoom depths
