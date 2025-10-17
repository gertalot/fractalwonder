# Story 001 - Project Setup and Development Environment

## References

- **requirements**: [REQUIREMENTS.md](./REQUIREMENTS.md) - Requirement 11 (Technology Stack)
- **design**: [DESIGN.md](./DESIGN.md) - Tech Stack, Build and Deployment sections
- **project**: [PROJECT.md](./PROJECT.md) - UI/UX requirements

## Description

Set up the initial Rust/Leptos project structure with a development server that displays a basic UI matching the target
design. The application will show a full-screen canvas with a checkerboard placeholder pattern and a bottom UI bar with
auto-hide behavior.

**Requirements**: *Requirement 11*

## Verification Criteria and Test Cases

- [ ] 1. README.md exists and includes all prerequisites, installation steps, development commands, and test commands
- [ ] 2. Running `trunk serve` starts a development server accessible at `http://localhost:8080`
- [ ] 3. Browser displays a full-screen canvas with a grey/white checkerboard pattern
- [ ] 4. Bottom UI bar is visible with placeholder text "Fractal Wonder - In Development"
- [ ] 5. Moving the mouse causes the UI to fade in (if hidden)
- [ ] 6. When mouse stops moving for 4 seconds, UI fades out
- [ ] 7. When mouse hovers over the UI area, UI remains visible even after 4 seconds of no movement
- [ ] 8. UI fade in/out animation is smooth (300ms duration per DESIGN.md)
- [ ] 9. Canvas maintains full viewport size when browser window is resized
- [ ] 10. Running `cargo test` executes all tests successfully
- [ ] 11. Project uses only Rust/Leptos (no TypeScript, React, or Yarn)
- [ ] 12. All code follows the workspace coding rules (120 char line length, 2 space indentation, type annotations,
  etc.)

## Implementation

### Step 1: Create Cargo project and initial structure

Create a new Rust project with Leptos as the web framework, using Trunk as the build tool. The project should use the
default Leptos template structure with proper WASM configuration.

**Instructions:**

- Initialize a new Cargo project in the repository root named "fractalwonder"
- Add Leptos 0.6+ as a dependency with the `csr` (client-side rendering) feature
- Add `wasm-bindgen` for WebAssembly browser APIs
- Add `web-sys` with features for `Window`, `Document`, `HtmlCanvasElement`, `CanvasRenderingContext2d`, `MouseEvent`,
  `EventTarget`
- Add `console_error_panic_hook` and `console_log` for debugging
- Add `leptos-use` for reactive utilities (mouse position, window events)
- Configure the project as a `cdylib` library type for WASM compilation
- Create a minimal `index.html` file in the project root for Trunk to use
- Create `Trunk.toml` configuration file with proper asset handling and WASM settings
- Ensure the crate-type includes `["cdylib", "rlib"]` to support both WASM and tests

### Step 2: Create README.md with comprehensive instructions

Create a detailed README.md file that provides clear, step-by-step instructions for setting up the development
environment, running the project, and executing tests.

**Instructions:**

- Create `README.md` in the repository root
- Include a "Prerequisites" section listing:
  - Rust 1.80+ installation via rustup
  - wasm32-unknown-unknown target (`rustup target add wasm32-unknown-unknown`)
  - Trunk installation (`cargo install trunk`)
- Include a "Development" section with:
  - Clone instructions
  - `trunk serve` command to start dev server
  - `trunk serve --open` to start and open browser automatically
  - Explanation that the server runs on `http://localhost:8080` with hot-reload
  - `trunk serve --release` for optimized development builds
- Include a "Testing" section with:
  - `cargo test` to run all unit tests
  - `cargo test -- --nocapture` to see console output
  - Note that WASM-specific tests require `wasm-pack test --headless --chrome` (but we don't have these yet)
- Include a "Building" section with:
  - `trunk build --release` for production builds
  - Explanation that output goes to `dist/` directory
- Include a "Project Structure" section explaining the main directories
- Add a brief description of the project at the top
- Keep tone professional but approachable

### Step 3: Implement canvas component with checkerboard pattern

Create a Leptos component that renders a full-screen canvas with a grey/white checkerboard pattern as a placeholder for
future fractal rendering.

**Instructions:**

- Create `src/components/mod.rs` to declare the components module
- Create `src/components/canvas.rs` file
- Add ABOUTME comment explaining the canvas component's purpose
- Implement a `Canvas` component function with `#[component]` attribute
- Use `create_node_ref::<html::Canvas>()` to get a reference to the canvas element
- Use `create_effect` to run canvas initialization after the DOM is mounted
- In the effect:
  - Get the canvas element from the node ref
  - Get the 2D rendering context
  - Set canvas width/height to match window inner dimensions
  - Draw a checkerboard pattern (alternating grey #e0e0e0 and white #ffffff squares)
  - Use 32x32 pixel square size for the checkerboard
- Use `window_event_listener` from leptos-use to listen for window resize events
- On resize, update canvas dimensions and redraw the checkerboard
- Return a `<canvas>` element with:
  - node_ref attached
  - class="block w-full h-full" (using Tailwind classes)
  - style="touch-action: none; cursor: grab;" (for future interactions)

### Step 4: Implement UI bar component with placeholder text

Create a Leptos component for the bottom UI bar that displays placeholder text and has a dark theme matching the design
specifications.

**Instructions:**

- Create `src/components/ui.rs` file
- Add ABOUTME comment explaining the UI bar component's purpose
- Implement a `UI` component function with `#[component]` attribute
- Accept a prop `is_visible: ReadSignal<bool>` to control visibility
- Return a `<div>` element with:
  - Fixed positioning at bottom of screen (class="fixed inset-x-0 bottom-0")
  - Dark semi-transparent background (class="bg-black/50 backdrop-blur-sm")
  - Padding (class="px-4 py-3")
  - Opacity controlled by is_visible prop
  - Transition animation (class="transition-opacity duration-300")
  - When is_visible is true: opacity-100
  - When is_visible is false: opacity-0
- Inside the div, create a centered text element displaying "Fractal Wonder - In Development"
  - Use class="text-center text-white text-sm"
- Keep the structure simple and minimal (no buttons or extra elements yet)

### Step 5: Implement auto-hide/show behavior

Create the reactive logic to automatically show the UI when the mouse moves and hide it after 4 seconds of inactivity,
with special handling for hovering over the UI area.

**Instructions:**

- Create `src/components/ui_visibility.rs` file
- Add ABOUTME comment explaining the UI visibility logic
- Create a function `use_ui_visibility() -> (ReadSignal<bool>, WriteSignal<bool>)` that:
  - Creates a `create_signal(true)` for is_visible (starts visible)
  - Creates a `create_signal(false)` for is_hovering (tracks if mouse is over UI)
  - Uses `use_timeout_fn` from leptos-use to create a 4-second timer
  - Uses `use_event_listener` from leptos-use to listen for `mousemove` on window
  - On mousemove:
    - Set is_visible to true
    - Reset/restart the 4-second timer
  - When timer expires:
    - Check if is_hovering is false
    - If not hovering, set is_visible to false
  - Returns the is_visible signal and is_hovering signal
- In the UI component (ui.rs):
  - Accept an additional prop `set_is_hovering: WriteSignal<bool>`
  - Add `on:mouseenter` event handler that sets is_hovering to true
  - Add `on:mouseleave` event handler that sets is_hovering to false

### Step 6: Create main App component and wire everything together

Create the main App component that combines the Canvas and UI components, sets up the UI visibility logic, and provides
the overall application structure.

**Instructions:**

- Create `src/app.rs` file
- Add ABOUTME comment explaining this is the main app component
- Implement an `App` component function with `#[component]` attribute
- Inside the component:
  - Call `use_ui_visibility()` to get the visibility signals
  - Return a `<div>` with class="h-screen w-screen overflow-hidden"
  - Inside the div:
    - Render `<Canvas />`
    - Render `<UI is_visible={is_visible} set_is_hovering={set_is_hovering} />`
- In `src/lib.rs`:
  - Add ABOUTME comment explaining this is the library entry point
  - Declare module structure (mod app; mod components;)
  - Re-export the App component
  - Add a `#[wasm_bindgen]` pub fn hydrate() that mounts the app to the document body
- In `src/main.rs`:
  - Add ABOUTME comment explaining this is only for test compilation
  - Add `fn main() {}` (empty, as Trunk uses the WASM target)

### Step 7: Set up Tailwind CSS for styling

Configure Tailwind CSS to work with Trunk and Leptos, following modern Tailwind best practices for 2025.

**Instructions:**

- Create `input.css` file in project root with Tailwind directives:

  ```css
  @tailwind base;
  @tailwind components;
  @tailwind utilities;
  ```

- Create `tailwind.config.js` with:
  - Content paths: `["./src/**/*.rs", "./index.html"]`
  - Dark theme configuration (though we may not use it initially)
  - Custom colors matching DESIGN.md (background: #0a0a0a, text: #e0e0e0, accent: #4a9eff)
- Add `<link data-trunk rel="tailwind-css" href="input.css" />` to index.html
- Trunk will automatically process Tailwind CSS during build

### Step 8: Add basic unit tests

Create unit tests for core functionality to establish testing patterns and ensure the build system is correctly
configured.

**Instructions:**

- Create `src/components/canvas.rs` tests module:
  - Test that canvas size calculation works correctly
  - Test that checkerboard pattern parameters are correct (32x32 squares)
  - Use `#[cfg(test)]` and `#[test]` attributes
- Create `src/components/ui_visibility.rs` tests module:
  - Test that visibility signal starts as true
  - Test that hovering signal starts as false
  - Use mock time and event testing if feasible
- Add dev-dependencies in Cargo.toml:
  - `wasm-bindgen-test` for WASM-specific testing (future use)
- Run `cargo test` to verify all tests pass
- Note in README.md that interactive behavior tests will require browser testing in future stories

### Step 9: Verify all acceptance criteria and clean up

Run through all verification criteria, test the application thoroughly, and clean up any issues or lint warnings.

**Instructions:**

- Start the dev server with `trunk serve`
- Open browser to `http://localhost:8080`
- Verify checkerboard pattern displays correctly
- Verify UI bar is visible with correct text
- Test mouse movement → UI fades in
- Test waiting 4 seconds → UI fades out
- Test hovering over UI → UI stays visible
- Test window resize → canvas resizes correctly
- Run `cargo test` and ensure all tests pass
- Run `cargo clippy` and fix any warnings
- Run `cargo fmt` to ensure code formatting is correct
- Verify all code has ABOUTME comments on relevant files
- Verify README.md is complete and accurate
- Verify no TypeScript, React, or Yarn files exist in the project
- Verify line length is ≤ 120 characters
- Verify indentation is 2 spaces
- Commit all changes with message "feat: initial project setup with canvas and UI"

## Notes

- The example code in `docs/example-src/` is TypeScript/React but should be used only as UI/UX reference, not code
  reference
- The checkerboard pattern is temporary and will be replaced with fractal rendering in future stories
- The 4-second timer for UI auto-hide is a UX requirement from PROJECT.md
- The 300ms fade animation duration is specified in DESIGN.md
- We're using Leptos with CSR (client-side rendering) only for now; SSR is not needed
- The `leptos-use` crate provides React-like hooks for Leptos (similar to react-use)
- Tailwind CSS integration with Trunk is seamless via the data-trunk attribute
- Browser testing (beyond unit tests) will be added in future stories when we add more complex interactions

## Dependencies Added

- `leptos = { version = "0.6", features = ["csr"] }`
- `wasm-bindgen = "0.2"`
- `web-sys = { version = "0.3", features = ["Window", "Document", "HtmlCanvasElement", "CanvasRenderingContext2d",
  "MouseEvent", "EventTarget"] }`
- `console_error_panic_hook = "0.1"`
- `console_log = "1.0"`
- `leptos-use = "0.10"` (for reactive utilities)
- Dev dependency: `wasm-bindgen-test = "0.3"`

## Build Tools

- Trunk 0.18+ (installed via `cargo install trunk`)
- Tailwind CSS (via Trunk's built-in support)
- rustfmt and clippy (standard Rust tools)
