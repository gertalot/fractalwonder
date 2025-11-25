# Fractal Wonder Implementation Roadmap

This roadmap defines incremental iterations to build Fractal Wonder from the current state to a fully-featured
Mandelbrot explorer capable of extreme deep zoom (10^2000+).

## Guiding Principles

- **Small, measurable increments**: Each iteration delivers testable functionality
- **Potentially shippable**: Every iteration produces something visible in the browser
- **Self-contained**: No iteration depends on future iterations
- **Test-driven development**: Write failing tests first, then implement
- **No global steps**: No "implement tests" or "add documentation" as separate iterations
- **Build on the archive**: Reuse patterns and code from `_archive/` where appropriate, but fix precision handling

## Current State

- `fractalwonder-core`: BigFloat, Viewport, PixelRect, transforms (pixel_to_fractal, fractal_to_pixel, apply_pixel_transform_to_viewport, compose_affine_transformations)
- `fractalwonder-ui`: `use_canvas_interaction` hook (complete), App component (just "Hello World")
- No compute layer, no canvas rendering, no UI panel

---

## ✅ Iteration 1: Canvas with Static Pattern

**Goal:** Prove we can render pixels to a canvas

**Status:** DONE

**Steps:**
1. Create `InteractiveCanvas` component with an HTML canvas element (reference `_archive/fractalwonder-ui/src/components/` for patterns)
2. Fill canvas with a simple position-dependent gradient (R = x%, G = y%, B = 50%)
3. Replace "Hello World" with `InteractiveCanvas`

**Deliverable:** A colored gradient fills the browser viewport

**Browser Test:** Open app, see gradient (red increases left-to-right, green increases top-to-bottom)

**Unit Tests:**
- Canvas element mounts with correct dimensions
- Gradient function returns expected RGBA for known (x, y)

---

## ✅ Iteration 2: UI Panel

**Goal:** Adopt the UI infrastructure from archive

**Steps:**
1. Port UI panel component from `_archive/fractalwonder-ui/src/components/`
2. Port autohide functionality as implemented in archive (mouse idle timeout)
3. Port full-screen toggle
4. Panel content: display canvas dimensions

**Deliverable:** Working UI panel with autohide and full-screen

**Browser Test:**
- See UI panel overlay on canvas
- Click full-screen button, app goes full-screen
- Leave mouse idle, UI panel fades out
- Move mouse, UI panel fades back in
- Panel shows "Canvas: 1920 x 1080" (or current size)

**Unit Tests:**
- Autohide signal responds to mouse activity timeout
- Full-screen toggle calls correct browser API

---

## ✅ Iteration 3: Config, Precision & Viewport Fitting

**Goal:** Implement fractal configuration, precision calculation, and viewport aspect ratio fitting

Make sure you read `docs/architecture.md` and the code in `_archive` to understand what to do.

**Steps:**
1. Implement `calculate_required_precision(viewport, canvas_size)` in `precision.rs`
2. Create `FractalConfig` struct with natural bounds (center, width, height) - e.g., Mandelbrot: center (-0.5, 0), width 4.0, height 4.0
3. Implement `fit_viewport_to_canvas(natural_viewport, canvas_size) -> Viewport` that expands viewport to match canvas aspect ratio while keeping center fixed (reference archive code)
4. Use `calculate_required_precision()` when creating any Viewport
5. Display config info and current precision bits in UI panel

**Deliverable:** Config system with precision-aware, aspect-ratio-fitted viewports

**Browser Test:**
- UI panel shows fractal name, natural bounds, and precision (e.g., "128 bits")
- On landscape monitor: viewport is wider than tall
- On portrait monitor: viewport is taller than wide
- Resize browser window, viewport adjusts, precision recalculates

**Unit Tests:**
- `calculate_required_precision` returns expected bits for known viewport/canvas combinations
- `fit_viewport_to_canvas` with square viewport + landscape canvas produces wider viewport
- Center stays the same after fitting
- All created Viewports have correct precision_bits

---

## ✅ Iteration 4: Viewport-Driven Rendering

**Goal:** Prove coordinate transforms work end-to-end

**Steps:**
1. Add `Viewport` signal to `InteractiveCanvas`, initialized via `fit_viewport_to_canvas`
2. Use `pixel_to_fractal()` to convert each pixel to fractal coordinates
3. Pattern reflects fractal coordinates: checkerboard at integer boundaries, circle/crosshairs at origin
4. Wire up `use_canvas_interaction` hook for pan/zoom
5. Display current viewport center and dimensions in UI panel

**Deliverable:** Interactive test pattern showing fractal coordinates

**Browser Test:**
- See checkerboard with visible origin marker
- Drag canvas, pattern moves smoothly (preview)
- Release, wait 1.5s, pattern re-renders at new position
- Zoom with scroll wheel, pattern scales around cursor
- UI panel shows current center (x, y) and width/height updating after each interaction

**Unit Tests:**
- Test pattern function: `pattern_at(0.0, 0.0)` returns origin marker color
- Test pattern function: `pattern_at(1.0, 1.0)` vs `pattern_at(1.5, 1.5)` differ (checkerboard)
- Viewport updates correctly after `apply_pixel_transform_to_viewport`

---

## ✅ Iteration 5: Zoom Level Display

**Goal:** Show current zoom level in UI

**Steps:**
1. Implement `calculate_zoom_level()` in `transforms.rs`
2. Implement `format_zoom_level()` in `transforms.rs`
3. Add zoom display to UI panel (e.g., "Zoom: 1x", "Zoom: 2.5x")

**Deliverable:** See current zoom level update as you interact

**Browser Test:**
- Initially shows "1x" (at natural bounds)
- Zoom in, shows "2x", "4x", "10x", etc.
- Zoom out, shows "0.5x", "0.25x", etc.
- Deep zoom, shows "1.5 x 10^20" format

**Unit Tests:**
- `calculate_zoom_level(width, width)` returns (1.0, 0)
- `calculate_zoom_level(width/2, width)` returns (2.0, 0)
- `format_zoom_level` produces "1x", "10x", "1.5 x 10^50"

---

## ✅ Iteration 6: Compute Crate & Renderer Trait

**Goal:** Create the compute layer foundation following the architecture

**Steps:**
1. Create `fractalwonder-compute` crate
2. Define data types: `TestImageData` in a  `ComputeData` enum (start with just TestImage variant)
3. Define `Renderer` trait: `render(region: &Viewport, resolution: (u32, u32)) -> Vec<Self::Data>`
4. Implement `TestImageRenderer` (formalizes the test pattern from Iteration 4)
5. Create basic `Colorizer` type in UI: `fn(&ComputeData) -> (u8, u8, u8, u8)`
6. Wire up: `InteractiveCanvas` uses `TestImageRenderer` -> `Colorizer` -> canvas pixels

**Deliverable:** Test pattern rendered through the proper Renderer -> Colorizer pipeline (still main thread)

**Browser Test:**
- Same visual result as Iteration 4 (checkerboard with origin marker)
- Pan/zoom still works
- No user-visible change, but architecture is now correct

**Unit Tests:**
- `TestImageRenderer::render()` returns correct `Vec<TestImageData>` for known viewport
- `TestImageData` serializes/deserializes correctly
- Colorizer produces expected RGBA for known `ComputeData`

---

## ✅ Iteration 7: MandelbrotRenderer

**Goal:** Add Mandelbrot computation following the Renderer trait

**Steps:**
1. Add `MandelbrotData { iterations: u32, escaped: bool }` to compute crate
2. Add `Mandelbrot` variant to `ComputeData` enum
3. Implement `MandelbrotRenderer` with `Renderer` trait (simple escape-time algorithm)
4. Add grayscale colorizer for `MandelbrotData`
5. Add renderer selection to UI (TestImage vs Mandelbrot)
6. Update `FractalConfig` to support Mandelbrot with its natural bounds (center -0.5, 0)

**Deliverable:** The Mandelbrot set rendered through the proper pipeline

**Browser Test:**
- Switch to Mandelbrot in UI, see the iconic Mandelbrot shape
- Black interior, gradient exterior
- Pan/zoom works (slowly - still main thread)
- Switch back to TestImage, see checkerboard

**Unit Tests:**
- `MandelbrotRenderer::render()` at origin region returns expected iteration counts
- Point (0, 0): max iterations (in set)
- Point (2, 0): 1 iteration (escapes immediately)
- Point (-0.75, 0): high iterations (on boundary)

---

## ✅ Iteration 8: Tiled Rendering

**Goal:** Progressive tile-by-tile display (still main thread)

**Steps:**
1. Implement tile grid calculation: divide canvas into tiles (64x64 or 128x128)
2. Order tiles center-out for better UX
3. Render each tile sequentially on main thread
4. Display each tile on canvas as it completes
5. Show progress in UI panel (e.g., "12/64 tiles")

**Deliverable:** See image build up tile by tile from center outward

**Browser Test:**
- Start render, see tiles appear progressively
- Center of image appears first, edges last
- Each tile aligns correctly (no seams or gaps)
- Progress updates in UI panel
- (Still slow - main thread, but now tiled)

**Unit Tests:**
- Tile grid covers canvas exactly (no overlap, no gaps)
- Center-out ordering: tiles closer to center have lower indices
- Tile pixel bounds convert to correct fractal-space regions

---

## ✅ Iteration 9: Worker Infrastructure

**Goal:** Distribute tiles across workers for parallel computation

**Steps:**
1. Define worker message types in compute crate: `MainToWorker`, `WorkerToMain` per architecture
2. Create worker entry point that receives tile region, runs renderer, sends results
3. Create WorkerPool managing N workers (based on `navigator.hardwareConcurrency`)
4. Dispatch tiles to available workers in parallel
5. Display tiles as workers complete them

**Deliverable:** Parallel tile rendering, UI stays responsive

**Browser Test:**
- Start render, multiple tiles appear simultaneously
- UI doesn't freeze
- Render completes noticeably faster than Iteration 9
- All tiles still correct

**Unit Tests:**
- `MainToWorker` / `WorkerToMain` serialize/deserialize correctly
- WorkerPool spawns expected number of workers
- Viewport precision preserved through JSON serialization

---

## ✅ Iteration 10: Render Cancellation

**Goal:** Cancel in-progress renders on new interaction

**Steps:**
1. When interaction starts, send cancel message to all workers
2. Workers stop current work and acknowledge
3. Clear pending tile queue
4. On interaction end, start fresh render

**Deliverable:** Responsive interaction even during slow renders

**Browser Test:**
- Start deep zoom render (takes several seconds)
- While rendering, drag the canvas
- Tiles stop appearing, preview shows immediately
- Release, fresh render starts from new viewport

**Unit Tests:**
- Cancel message received and acknowledged by workers
- Pending tile queue cleared on cancel
- No stale tile results displayed after cancellation

---

## Iteration 11: Progress Indication

**Goal:** Rich progress feedback during rendering

**Steps:**
1. Implement `RenderProgress` struct per architecture (completed_tiles, total_tiles, elapsed_ms, is_complete)
2. Add visual progress bar to UI panel
3. Show elapsed time during render
4. Show render complete confirmation (briefly)

**Deliverable:** Clear visual feedback of rendering progress

**Browser Test:**
- During render, see progress bar fill up
- Shows "32/64 tiles" and "1.2s elapsed"
- Progress bar animates smoothly
- On completion, brief "Complete" indicator then hides

**Unit Tests:**
- RenderProgress updates correctly as tiles complete
- Progress percentage calculation correct
- elapsed_ms tracks actual time

---

## Iteration 12: Perturbation Theory Rendering

**Goal:** Enable extreme deep zoom (10^1000+) with reasonable performance

**Steps:**
1. Implement `PerturbationRenderer` following the architecture's `OrchestrationType::Perturbation`
2. Compute high-precision reference orbit at viewport center
3. Use low-precision deltas from reference for all other pixels
4. Implement glitch detection (where perturbation approximation fails)
5. Subdivide and compute additional reference points for glitched regions
6. Add renderer selection in config: `SimpleTiling` vs `Perturbation`

**Deliverable:** Deep zoom to 10^1000+ at practical speeds

**Browser Test:**
- Zoom to 10^100, renders in reasonable time (not hours)
- Zoom to 10^500, still renders
- No visible glitches or artifacts in final image
- Compare known deep locations to reference images

**Unit Tests:**
- Reference orbit computation produces correct values
- Delta iteration matches full-precision result (within tolerance)
- Glitch detection identifies problematic pixels
- Subdivision produces correct sub-regions

---

## Iteration 13: Colorizer System

**Goal:** Multiple color schemes with UI selection

**Steps:**
1. Create `ColorizerInfo` struct and colorizer registry per the architecture
2. Implement multiple Mandelbrot colorizers (grayscale, fire, ocean, rainbow)
3. Add colorizer dropdown to UI panel
4. Cache computed `MandelbrotData`, re-colorize without recomputing on colorizer change

**Deliverable:** Switch between color schemes instantly

**Browser Test:**
- Dropdown shows available colorizers for current renderer
- Select "Fire", colors change instantly (no re-render delay)
- Select "Ocean", colors change instantly
- Same fractal structure, different appearance
- Pan/zoom recomputes, then applies current colorizer

**Unit Tests:**
- Each colorizer produces distinct colors for same iteration count
- Colorizer registry returns correct colorizers for "mandelbrot" renderer_id
- Cache hit: changing colorizer doesn't trigger recompute

---


## Summary

| # | Iteration | Key Deliverable |
|---|-----------|-----------------|
| 1 | Canvas with Static Pattern | See gradient on canvas |
| 2 | UI Panel | Autohide, full-screen, canvas size display |
| 3 | Config, Precision & Viewport Fitting | Precision calculation, aspect ratio fitting |
| 4 | Viewport-Driven Rendering | Interactive test pattern with pan/zoom |
| 5 | Zoom Level Display | See "1x", "10x", "10^50" in UI |
| 6 | Compute Crate & Renderer Trait | Proper Renderer -> Colorizer pipeline |
| 7 | MandelbrotRenderer | See the Mandelbrot set |
| 8 | Colorizer System | Multiple color schemes, instant switching |
| 9 | Tiled Rendering | Progressive tile-by-tile display |
| 10 | Worker Infrastructure | Parallel rendering, responsive UI |
| 11 | Render Cancellation | Interrupt renders on interaction |
| 12 | Progress Indication | Visual progress bar, elapsed time |
| 13 | Perturbation Theory | Extreme deep zoom (10^1000+) |

---

## Key Milestones

- **After Iteration 4:** Working interactive canvas with coordinate transforms
- **After Iteration 7:** See the actual Mandelbrot set
- **After Iteration 10:** Production-quality parallel rendering
- **After Iteration 13:** World-class deep zoom capability
