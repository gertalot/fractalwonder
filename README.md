# Fractal Wonder

A high-performance, browser-based Mandelbrot set explorer capable of rendering at extreme zoom levels (up to 10^100 and
beyond) with interactive real-time exploration. Built entirely in Rust using Leptos and WebAssembly.

## Prerequisites

Before you begin, ensure you have the following installed:

- **Rust 1.80 or later**: Install via [rustup](https://rustup.rs/)

  ```bash
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  ```

- **wasm32-unknown-unknown target**: Required for WebAssembly compilation

  ```bash
  rustup target add wasm32-unknown-unknown
  ```

- **Trunk**: Build tool for Rust WASM applications

  ```bash
  cargo install trunk
  ```

- **Node.js and npm** (optional): Only needed if you want to modify Tailwind CSS configuration
  - Trunk has built-in Tailwind CSS support via the standalone CLI

## Development

### Clone and Run

```bash
# Clone the repository
git clone <repository-url>
cd fractalwonder

# Start the development server with hot-reload
trunk serve

# Or start and automatically open in browser
trunk serve --open
```

The development server runs on `http://localhost:8080` with automatic hot-reload enabled. Changes to Rust files will
trigger automatic recompilation and browser refresh.

### Development Modes

```bash
# Development mode (default, with debug info)
trunk serve

# Development with optimized builds (faster runtime, slower compilation)
trunk serve --release
```

## Testing

```bash
# Run all unit tests
cargo test

# Run tests with console output visible
cargo test -- --nocapture

# Run tests for a specific module
cargo test components::

# WASM-specific browser tests (when available)
wasm-pack test --headless --chrome
```

## Building

### Production Build

```bash
# Build optimized release version
trunk build --release
```

Output files are generated in the `dist/` directory and are ready to deploy to any static hosting service.

### Serving Production Build Locally

The production build requires specific HTTP headers for SharedArrayBuffer support (needed for multi-threading):

```bash
# Example using Python's http.server with custom headers
cd dist
python3 -m http.server 8080
```

**Note**: For proper SharedArrayBuffer support in production, your server must send:

- `Cross-Origin-Opener-Policy: same-origin`
- `Cross-Origin-Embedder-Policy: require-corp`

Trunk's dev server includes these headers automatically.

## Project Structure

```txt
fractalwonder/
├── src/
│   ├── lib.rs              # WASM entry point
│   ├── main.rs             # Empty main (for test compatibility)
│   ├── app.rs              # Main App component
│   └── components/         # UI components
│       ├── mod.rs
│       ├── canvas.rs       # Full-screen canvas rendering
│       ├── ui.rs           # Bottom UI bar
│       └── ui_visibility.rs # Auto-hide/show logic
├── index.html              # HTML entry point for Trunk
├── input.css               # Tailwind CSS source
├── tailwind.config.js      # Tailwind configuration
├── Trunk.toml              # Trunk build configuration
├── Cargo.toml              # Rust dependencies
└── README.md               # This file
```

## Technology Stack

- **Language**: Rust (100% Rust, no TypeScript/JavaScript)
- **Frontend Framework**: Leptos 0.6+ (compiled to WebAssembly)
- **Build Tool**: Trunk (bundling and dev server)
- **Styling**: Tailwind CSS (processed by Trunk)
- **Testing**: cargo test, wasm-bindgen-test

## Development Notes

- The checkerboard pattern on canvas is a placeholder for future fractal rendering
- UI auto-hides after 4 seconds of mouse inactivity (UX requirement)
- The 300ms fade animation duration follows the design specification
- Browser testing will be expanded in future development iterations

## Contributing

This is a personal project, but suggestions and bug reports are welcome via GitHub issues.

## License

MIT
