# Requirements Document - Extreme Zoom Mandelbrot Explorer

## Description

A high-performance, browser-based Mandelbrot set explorer capable of rendering at extreme zoom levels (up to 10^100 and
beyond) with interactive real-time exploration. The application uses Rust compiled to WebAssembly for computation,
WebGPU for GPU acceleration, and a TypeScript/React frontend for the user interface.

### Value

This tool enables deep exploration of the Mandelbrot set at zoom levels previously requiring specialized desktop
software, making extreme-precision fractal visualization accessible through a web browser. It serves as a personal
exploration tool and potential open-source project for the fractal enthusiast community.

### Context

The application leverages modern web technologies (WebGPU, WebAssembly) combined with advanced mathematical techniques
(perturbation theory, arbitrary precision arithmetic) to overcome the traditional limitations of browser-based fractal
rendering. The architecture is designed to support future distributed rendering capabilities without requiring
mathematics code rewrites.

### Scope

**In Scope:**

- Interactive Mandelbrot set exploration with pan and zoom
- Extreme zoom support (target: 10^100, architecture supports higher)
- GPU and multi-core CPU acceleration
- Progressive tile-based rendering
- URL-based bookmark/sharing system
- Predefined color schemes (minimum 3)
- PNG image export with embedded metadata
- Precision handling up to 250+ decimal places
- Automatic iteration count scaling based on zoom level
- Modern, minimal, dark-themed UI

**Out of Scope (Future Enhancements):**

- Distributed/backend rendering
- Custom color scheme editor
- Animation/zoom sequences
- Other fractal types (Julia, Lyapunov, etc.)
- Keyboard shortcuts and accessibility features
- Tutorial/onboarding experience

---

## Requirements

## Requirement 1 - Core Rendering Engine

**User story**: As a fractal explorer, I want to compute Mandelbrot set visualizations at extreme zoom levels, so that I
can discover intricate fractal structures invisible at standard precision.

### Acceptance Criteria

1. **The system SHALL** compute Mandelbrot set iterations using arbitrary precision arithmetic in Rust/WebAssembly

2. **WHEN** zoom level exceeds 10^14 **THEN** the system **SHALL** use perturbation theory to optimize computation
   performance

3. **The system SHALL** automatically calculate required decimal precision using the formula: `max(30,
   ceil(log10(zoom_level) × 2.5 + 20))`

4. **The system SHALL** separate computation (iteration data) from coloring (visual output) to enable re-coloring
   without recomputation

5. **The system SHALL** store per-pixel computation data including: iteration count, escape flag, final z value,
   smoothed iteration (mu), and derivative magnitude

6. **The system SHALL** support zoom levels up to 10^100 with no artificial mathematical limits

7. **The system SHALL** utilize all available CPU cores for computation

8. **WHEN** WebGPU is available **THEN** the system **SHALL** utilize GPU acceleration for rendering

9. **WHEN** WebGPU is unavailable **THEN** the system **SHALL** fall back to CPU rendering

### Research Questions

1. Can emulated f64 or f128 precision arithmetic in WGSL be practical for perturbation delta orbit calculations?
2. At what zoom levels does f32 become insufficient for delta orbit calculations?
3. What is the performance tradeoff between GPU with emulated precision vs CPU with native f64?
4. How to efficiently implement higher precision (64-bit or 128-bit) arithmetic in WGSL when not natively supported?
5. What is the optimal data pipeline architecture for passing computation results from Rust/WASM to WebGPU?

---

## Requirement 2 - Progressive Tile-Based Rendering

**User story**: As a fractal explorer, I want to see rendering progress in real-time with the most interesting areas
first, so that I can quickly assess if a location is worth exploring further.

### Acceptance Criteria

1. **The system SHALL** render the viewport using tile-based rendering

2. **The system SHALL** determine tile size dynamically based on zoom level and complexity

3. **The system SHALL** render tiles in a spiral pattern starting from the viewport center outward

4. **WHEN** rendering is in progress **THEN** the system **SHALL** display completed tiles immediately as they finish

5. **The system SHALL** display a circular progress indicator in the bottom-left corner when UI is hidden

6. **WHEN** UI is visible **THEN** the system **SHALL** display a linear progress bar showing render progress

7. **WHEN** UI is visible during rendering **THEN** the system **SHALL** display the number of active compute units
   (cores/threads/workers)

8. **WHEN** render completes **THEN** the system **SHALL** hide all progress indicators

9. **WHEN** UI is visible and render completes **THEN** the system **SHALL** display the total render time

---

## Requirement 3 - High-Precision Coordinate System

**User story**: As a fractal explorer, I want coordinate transformations to maintain perfect accuracy at extreme zoom
levels, so that interactions remain smooth and precise even at 10^100 zoom.

### Acceptance Criteria

1. **The system SHALL** store viewport center coordinates using arbitrary precision types in Rust

2. **The system SHALL** maintain coordinate precision up to 250+ decimal places

3. **The system SHALL** explicitly distinguish pixel-space values from fractal-space values in all code and variable
   names

4. **The system SHALL** implement pixel-to-fractal coordinate conversion that maintains full precision

5. **The system SHALL** implement fractal-to-pixel coordinate conversion that maintains full precision

6. **The system SHALL** pass coordinates between Rust/WASM and JavaScript without precision loss using string
   serialization

7. **WHEN** precision becomes insufficient for current zoom level **THEN** the system **SHALL** detect this condition
   and warn the user

8. **The system SHALL** support round-trip coordinate conversion (pixel → fractal → pixel) that is invariant at all zoom
   levels

### Testing Requirements

1. **The system SHALL** include round-trip validation tests verifying pixel→fractal→pixel conversions are lossless at
   zoom levels: 10^15, 10^30, 10^50, 10^100

2. **The system SHALL** include mathematical derivation tests that calculate expected coordinate changes for known
   transformations and verify code produces exact values

3. **The system SHALL** include property-based tests validating mathematical invariants (e.g., "fractal point under
   mouse stays under mouse during zoom")

---

## Requirement 4 - Interactive Exploration

**User story**: As a fractal explorer, I want to pan and zoom with smooth, accurate previews, so that I can intuitively
navigate the fractal at any zoom level.

### Acceptance Criteria

1. **WHEN** user drags the mouse **THEN** the system **SHALL** terminate any active render immediately

2. **WHEN** user drags the mouse **THEN** the system **SHALL** display a fast preview by transforming current imageData
   to match mouse movement exactly

3. **WHEN** user scrolls mouse wheel **THEN** the system **SHALL** zoom centered on the mouse cursor position in fractal
   space

4. **WHEN** user scrolls mouse wheel **THEN** the system **SHALL** maintain constant visual zoom rate (same scroll
   distance = same perceived zoom change) regardless of current zoom level

5. **WHEN** user resizes browser window AND canvas size actually changes **THEN** the system **SHALL** terminate active
   render and maintain fractal center in viewport center

6. **WHEN** user resizes browser window WITHOUT canvas size change **THEN** the system **SHALL NOT** terminate the
   active render

7. **WHEN** user clicks window resize handle WITHOUT changing size **THEN** the system **SHALL NOT** terminate the
   active render

8. **WHEN** user stops interacting for 1.5 seconds **THEN** the system **SHALL** start a new render

9. **The system SHALL** consider interaction stopped when no mouse movement, drag, scroll, or resize occurs for 1.5
   seconds

10. **The system SHALL NOT** consider mouse movement without button press as interaction for render cancellation
    purposes

11. **WHEN** user performs zoom centered on mouse at position (x,y) **THEN** the fractal point at (x,y) **SHALL** remain
    at pixel position (x,y) after zoom completes

12. **The system SHALL** support extreme precision coordinate changes (250+ decimal places) when user drags at zoom
    levels 10^100+

### Testing Requirements

1. **The system SHALL** include invariant tests verifying mouse position stays fixed in fractal space during zoom
   operations at zoom levels: 10^50, 10^100

2. **The system SHALL** include mathematical derivation tests for drag operations: at zoom 10^100, dragging N pixels
   produces mathematically correct coordinate delta

3. **The system SHALL** include edge case tests for viewport transforms at extreme zoom, minimum/maximum viewport sizes,
   and corner mouse positions

---

## Requirement 5 - User Interface

**User story**: As a fractal explorer, I want a minimal, distraction-free interface that appears only when needed, so
that I can focus on the fractal visualization.

### Acceptance Criteria

1. **The system SHALL** display canvas at full screen size by default

2. **The system SHALL** hide the UI by default

3. **WHEN** user moves mouse or taps screen **THEN** the system **SHALL** fade in a full-width UI area at the bottom of
   the screen

4. **WHEN** user stops moving mouse or after tap **THEN** the system **SHALL** fade out the UI after 4 seconds

5. **WHEN** UI is visible **THEN** the system **SHALL** display current zoom level, fractal coordinates, and iteration
   count

6. **WHEN** UI is visible **THEN** the system **SHALL** display a color scheme selector with adjustable settings for the
   selected scheme

7. **WHEN** UI is visible **THEN** the system **SHALL** display an "i" icon (bottom-left) that opens an information menu
   with GitHub link

8. **WHEN** UI is visible **THEN** the system **SHALL** display a "home" icon that resets to default parameters

9. **WHEN** UI is visible **THEN** the system **SHALL** display a fullscreen toggle icon (bottom-right)

10. **The system SHALL** use a dark theme with sleek, minimal aesthetics following modern UX guidelines

11. **WHEN** UI is visible **THEN** the system **SHALL** display metrics showing active CPU cores/threads and GPU
    compute units

12. **The system SHALL** match the exact UI design shown in the reference screenshots in PROJECT.md

---

## Requirement 6 - Color Schemes and Rendering Pipeline

**User story**: As a fractal explorer, I want to apply different color schemes to the same computation, so that I can
visualize the fractal in different ways without re-rendering.

### Acceptance Criteria

1. **The system SHALL** implement at least 3 predefined color algorithms

2. **The system SHALL** store computation results separately from color mapping

3. **The system SHALL** store per-pixel data in f32 format: iteration count, escape flag, final z magnitude, smoothed
   mu, derivative magnitude

4. **The system SHALL** support color algorithms including: smooth coloring, orbit trap, potential-based, distance
   estimation, and field-line methods

5. **WHEN** user changes color scheme **THEN** the system **SHALL** re-color without recomputing iteration data

6. **The system SHALL** allow per-scheme settings adjustments (offset, scale, cycling speed, etc.)

7. **The system SHALL** include color scheme selection in URL parameters for bookmarking

---

## Requirement 7 - State Persistence and Bookmarking

**User story**: As a fractal explorer, I want to bookmark and share specific locations, so that I can return to
interesting discoveries and share them with others.

### Acceptance Criteria

1. **The system SHALL** encode all fractal parameters in URL query parameters: center coordinates, zoom level, iteration
   settings, color scheme, and color scheme settings

2. **The system SHALL** use compressed encoding for URL parameters to minimize URL length

3. **The system SHALL** store current parameters in browser localStorage

4. **WHEN** application loads WITH URL parameters **THEN** the system **SHALL** use URL parameters

5. **WHEN** application loads WITHOUT URL parameters **THEN** the system **SHALL** load parameters from localStorage

6. **WHEN** application loads WITHOUT URL parameters AND localStorage is empty **THEN** the system **SHALL** use default
   parameters

7. **WHEN** URL parameters are invalid or corrupted **THEN** the system **SHALL** ignore them and fall back to
   localStorage or defaults

8. **The system SHALL** update URL parameters as user explores (on render start after interaction stops)

9. **The system SHALL** serialize/deserialize arbitrary precision coordinates without precision loss

---

## Requirement 8 - Default Configuration

**User story**: As a new user, I want to see the classic Mandelbrot set on first load, so that I have a clear starting
point for exploration.

### Acceptance Criteria

1. **The system SHALL** use default viewport showing the complete Mandelbrot set (real: -2 to 1, imaginary: -1.5 to 1.5)

2. **The system SHALL** use 500 iterations as the default base iteration count

3. **The system SHALL** scale iterations based on zoom using formula: `scaledIterations = baseIterations *
   iterationScalingFactor * Math.pow(log10(zoom), 1.5)`

4. **The system SHALL** define a default color scheme (to be determined during implementation)

5. **The system SHALL** allow user to adjust iteration scaling factor

6. **WHEN** user clicks home/reset button **THEN** the system **SHALL** restore these default parameters

---

## Requirement 9 - Image Export

**User story**: As a fractal explorer, I want to save high-quality images of discoveries, so that I can preserve and
share interesting fractal locations.

### Acceptance Criteria

1. **WHEN** UI is visible **THEN** the system **SHALL** display a save/export button

2. **WHEN** user clicks save button **THEN** the system **SHALL** prompt for filename

3. **The system SHALL** export current viewport as rendered at current canvas resolution

4. **The system SHALL** save images in PNG format

5. **The system SHALL** embed fractal parameters in PNG metadata (coordinates, zoom, iterations, color scheme)

6. **IF** PNG metadata embedding is not possible **THEN** the system **SHALL** generate a companion text file with
   parameters

---

## Requirement 10 - Error Handling and Resilience

**User story**: As a fractal explorer, I want the system to gracefully handle errors and limitations, so that I have a
reliable exploration experience.

### Acceptance Criteria

1. **WHEN** WebGPU is unavailable **THEN** the system **SHALL** render using CPU with visible indication of CPU-only
   mode

2. **WHEN** computation buffer exceeds available memory **THEN** the system **SHALL** display error message suggesting
   window resize

3. **WHEN** rendering takes longer than 30 seconds **THEN** the system **SHALL** continue rendering and update progress
   indicators

4. **The system SHALL** support cancellation of long-running renders via user interaction

5. **WHEN** browser tab is not visible **THEN** the system **MAY** pause rendering to conserve resources

6. **The system SHALL** validate all user inputs and handle invalid values gracefully

---

## Requirement 11 - Technology Stack

**User story**: As a developer, I want a technology stack that supports current browser-based rendering and future
backend expansion, so that the codebase doesn't require rewrites for distributed rendering.

### Acceptance Criteria

1. **The system SHALL** use Rust for all mathematical computation code

2. **The system SHALL** compile Rust to WebAssembly for browser execution

3. **The system SHALL** use the `rug` crate (or equivalent) for arbitrary precision arithmetic

4. **The system SHALL** use TypeScript for frontend code

5. **The system SHALL** use React for UI framework

6. **The system SHALL** use Yarn for package management

7. **The system SHALL** use WebGPU for GPU acceleration when available

8. **The system SHALL** use Web Workers for multi-threaded computation

9. **The system SHALL** structure code to support future native compilation of Rust math code for backend rendering

10. **The system SHALL** isolate the computation engine as a reusable module with no browser-specific dependencies, so that it can be used directly in backend/distributed rendering systems without modification

---

## Requirement 12 - Testing and Quality Assurance

**User story**: As a developer, I want comprehensive test coverage validating mathematical correctness, so that I can
trust the system produces accurate results at extreme zoom levels.

### Acceptance Criteria

1. **The system SHALL** include unit tests for all coordinate transformation functions

2. **The system SHALL** include mathematical derivation tests that calculate expected values from first principles and
   validate against code output

3. **The system SHALL** include round-trip validation tests for pixel↔fractal conversions at zoom levels: 10^15, 10^30,
   10^50, 10^100

4. **The system SHALL** include property-based tests validating mathematical invariants

5. **The system SHALL** include integration tests for pan, zoom, and resize operations

6. **The system SHALL** include precision boundary tests using extreme coordinate values (250+ decimal places)

7. **The system SHALL** include edge case matrix tests covering extreme zoom/viewport/mouse position combinations

8. **The system SHALL** validate that mathematical invariants hold (e.g., "mouse position invariant under zoom")

9. **The system SHALL** design tests to FAIL until implementation is mathematically correct, not to easily pass

10. **The system SHALL** include tests with known reference coordinates from the fractal community

---

## Technical Constraints

1. **WebGPU/WGSL does not support native f64** - emulation or CPU fallback required for f64 precision
2. **Browser memory limits** - maximum practical buffer size ~2GB per canvas
3. **JavaScript number precision** - zoom level stored as f64 limits zoom representation to ~10^308
4. **Arbitrary precision library limits** - bounded only by available memory, not mathematical constraints
5. **Computation time** - extreme zooms (10^1000+) may require hours/days per frame

## Performance Targets

1. **The system SHOULD** render standard zoom levels (< 10^15) in under 5 seconds on modern hardware
2. **The system SHOULD** render extreme zoom levels (10^50 - 10^100) in under 5 minutes on modern hardware
3. **The system SHOULD** utilize 100% of available CPU cores during computation
4. **The system SHOULD** maintain 60fps for interaction previews (imageData transforms) at all zoom levels
5. **The system SHOULD** minimize memory usage through efficient tile streaming where possible

## Future Enhancements (Out of Scope for V1)

1. Bivariate Linear Approximation (BLA) optimization
2. Series approximation optimization
3. Distributed rendering with backend compute nodes
4. Custom color scheme editor with gradient designer
5. Animation and zoom sequence recording
6. Other fractal types (Julia sets, Lyapunov fractals, etc.)
7. Keyboard shortcuts and full accessibility support
8. Tutorial and onboarding experience
9. Advanced glitch detection and automatic reference point adjustment
10. Export at higher resolution than viewport (super-sampling)

