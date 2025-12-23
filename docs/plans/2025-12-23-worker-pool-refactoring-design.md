# Worker Pool Refactoring Design

## Problem

`fractalwonder-ui/src/workers/worker_pool.rs` is 1291 lines with 5 intertwined responsibilities:

| Responsibility | Lines | Issue |
|---------------|-------|-------|
| Worker Lifecycle | ~100 | Mixed with render logic |
| Message Routing | ~290 | Single 290-line function |
| Render Coordination | ~200 | Shared setup duplicated |
| Perturbation State | ~200 | Tightly coupled to WorkerPool |
| Glitch/Cell Orbits | ~300 | Phase 7-8 experimental feature inline |

Additionally:
- No unit tests (unlike quadtree.rs with 30+ tests)
- Duplicated logic: viewport validation, max_iterations calculation appear 3 times
- 23 fields covering multiple concerns

## Solution

Extract concerns into focused modules with clear APIs. WorkerPool becomes a thin orchestrator.

## New File Structure

```
fractalwonder-ui/src/workers/
├── mod.rs                    # Re-exports public API
├── worker_pool.rs            # Slimmed: lifecycle + routing + standard render
├── quadtree.rs               # Unchanged
└── perturbation/
    ├── mod.rs                # Re-exports coordinator + glitch_resolution
    ├── coordinator.rs        # PerturbationState + render orchestration
    ├── glitch_resolution.rs  # Phase 7-8: quadtree-based orbit computation
    └── helpers.rs            # Pure functions: validation, calculations
```

## Line Count Estimates

| File | Lines | Contents |
|------|-------|----------|
| worker_pool.rs | ~500 | Worker lifecycle, message handlers (as methods), standard render, delegation |
| perturbation/coordinator.rs | ~350 | PerturbationState, start_perturbation_render, compute_orbit_for_gpu |
| perturbation/glitch_resolution.rs | ~250 | Cell orbit computation, broadcasting, subdivision |
| perturbation/helpers.rs | ~80 | validate_viewport(), calculate_render_max_iterations(), calculate_dc_max() |

**Total: ~1180 lines** (down from 1291, but more importantly, separated by concern)

## Module APIs

### PerturbationCoordinator

```rust
// perturbation/coordinator.rs

pub struct PerturbationCoordinator {
    state: PerturbationState,
    glitch_resolver: GlitchResolver,
}

impl PerturbationCoordinator {
    pub fn new() -> Self;

    /// Prepare for a new perturbation render, returns orbit request to send
    pub fn start_render(&mut self, viewport: &Viewport, canvas_size: (u32, u32))
        -> OrbitRequest;

    /// Prepare for GPU orbit computation (no tiles)
    pub fn start_gpu_render(&mut self, viewport: &Viewport, canvas_size: (u32, u32))
        -> OrbitRequest;

    /// Handle orbit completion, returns messages to broadcast
    pub fn on_orbit_complete(&mut self, orbit_data: OrbitData)
        -> Vec<MainToWorker>;

    /// Check if worker is ready for tile dispatch
    pub fn worker_ready_for_tiles(&self, worker_id: usize) -> bool;

    /// Build tile message with delta parameters
    pub fn build_tile_message(&self, tile: PixelRect, orbit_id: u32)
        -> MainToWorker;
}
```

**Key insight:** WorkerPool handles *when* to send messages (worker lifecycle, message routing). PerturbationCoordinator handles *what* messages contain (delta calculations, orbit state).

### GlitchResolver

```rust
// perturbation/glitch_resolution.rs

pub struct GlitchResolver {
    quadtree: Option<QuadtreeCell>,
    glitched_tiles: Vec<PixelRect>,
    cell_orbits: HashMap<CellKey, ReferenceOrbit>,
    cell_orbit_ids: HashMap<CellKey, u32>,
    cell_orbit_confirmations: HashMap<u32, HashSet<usize>>,
    orbit_id_counter: u32,
}

type CellKey = (u32, u32, u32, u32);  // (x, y, width, height)

impl GlitchResolver {
    pub fn new() -> Self;

    /// Initialize quadtree for a new render
    pub fn init_for_render(&mut self, canvas_size: (u32, u32));

    /// Record a glitched tile
    pub fn record_glitched_tile(&mut self, tile: PixelRect);

    /// Subdivide cells containing glitches, returns subdivision count
    pub fn subdivide_glitched_cells(&mut self) -> u32;

    /// Compute orbits for glitched cell centers
    pub fn compute_cell_orbits(&mut self, viewport: &Viewport, canvas_size: (u32, u32));

    /// Get orbits needing broadcast
    pub fn orbits_to_broadcast(&mut self) -> Vec<(u32, OrbitBroadcast)>;

    /// Record worker confirmation for an orbit
    pub fn confirm_orbit_stored(&mut self, orbit_id: u32, worker_id: usize);
}
```

### Helpers (Pure Functions)

```rust
// perturbation/helpers.rs

/// Validate viewport dimensions, returns Err with message if invalid
pub fn validate_viewport(viewport: &Viewport) -> Result<(), String>;

/// Calculate max iterations from zoom level
pub fn calculate_render_max_iterations(viewport: &Viewport, config: &FractalConfig) -> u32;

/// Calculate dc_max (maximum delta magnitude) for BLA
pub fn calculate_dc_max(viewport: &Viewport) -> f64;
```

### Slimmed WorkerPool

```rust
// worker_pool.rs

pub struct WorkerPool {
    // Worker lifecycle
    workers: Vec<Worker>,
    renderer_id: String,
    initialized_workers: HashSet<usize>,
    self_ref: Weak<RefCell<Self>>,

    // Render coordination (shared between modes)
    pending_tiles: VecDeque<PixelRect>,
    current_render_id: u32,
    current_viewport: Option<Viewport>,
    canvas_size: (u32, u32),
    progress: RwSignal<RenderProgress>,
    render_start_time: Option<f64>,

    // Callbacks
    on_tile_complete: Rc<dyn Fn(TileResult)>,
    on_render_complete: Rc<RefCell<Option<Rc<dyn Fn()>>>>,
    on_orbit_complete: OrbitCompleteCallback,

    // Mode flags
    is_perturbation_render: bool,
    gpu_mode: bool,

    // Delegated concern
    perturbation: PerturbationCoordinator,
}

impl WorkerPool {
    // Message handlers become focused methods
    fn handle_message(&mut self, worker_id: usize, msg: WorkerToMain) {
        match msg {
            WorkerToMain::Ready => self.handle_ready(worker_id),
            WorkerToMain::RequestWork { render_id } => self.handle_request_work(worker_id, render_id),
            WorkerToMain::TileComplete { .. } => self.handle_tile_complete(...),
            WorkerToMain::Error { message } => self.handle_error(worker_id, message),
            WorkerToMain::ReferenceOrbitComplete { .. } => self.handle_orbit_complete(...),
            WorkerToMain::OrbitStored { orbit_id } => self.handle_orbit_stored(worker_id, orbit_id),
        }
    }
}
```

## Testing Strategy

**What gets tested:**

| Module | Testable Units |
|--------|----------------|
| `helpers.rs` | Pure functions with clear inputs/outputs |
| `glitch_resolution.rs` | Subdivision logic, orbit computation with mock data |
| `coordinator.rs` | Delta calculations, message building |

**What stays untested (integration only):**

- WorkerPool message routing (requires Web Workers)
- Actual worker communication
- Browser-specific callbacks

## Migration Strategy

Incremental, non-breaking steps:

1. **Create module structure** - Add empty files with module declarations
2. **Extract helpers first** - Pure functions, no dependencies, easy to test
3. **Extract GlitchResolver** - Self-contained, move state + methods together
4. **Extract PerturbationCoordinator** - Depends on GlitchResolver, move together
5. **Refactor WorkerPool** - Update to use extracted modules, split handle_message
6. **Add tests** - For helpers and glitch_resolution logic
7. **Clean up** - Remove duplication, verify all tests pass

Each step compiles and tests pass before proceeding.

## Decisions Made

| Question | Decision | Rationale |
|----------|----------|-----------|
| Glitch resolution handling? | Extract to separate module | Distinct feature, ~300 lines, Phase 7-8 experimental |
| Perturbation state? | Extract to coordinator module | Separates "what" from "when" in message handling |
| Message handlers? | Inline methods, not separate file | Need access to WorkerPool state, keeps logic close |
| Testing approach? | Test extracted pure logic only | Web Workers untestable in unit tests |
| File organization? | Nested `perturbation/` submodule | Groups related concerns, signals relationship |
