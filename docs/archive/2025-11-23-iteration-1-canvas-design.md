# Iteration 1: Canvas with Static Pattern

## Goal

Prove we can render pixels to a canvas using ImageData pixel manipulation.

## Files

**Create:**
- `fractalwonder-ui/src/components/mod.rs`
- `fractalwonder-ui/src/components/interactive_canvas.rs`

**Modify:**
- `fractalwonder-ui/src/lib.rs` - add `mod components`
- `fractalwonder-ui/src/app.rs` - replace "Hello World" with `<InteractiveCanvas />`
- `fractalwonder-ui/Cargo.toml` - verify `web-sys` features

## Component Design

### `InteractiveCanvas`

```rust
#[component]
pub fn InteractiveCanvas() -> impl IntoView
```

No props for Iteration 1. Viewport and interaction come in Iteration 4.

**Internal structure:**
- `canvas_ref: NodeRef<Canvas>`
- Single `create_effect` for dimensions + rendering on mount
- Returns `<canvas class="block" />`

### Rendering Logic

The effect:
1. Gets canvas element and 2D context
2. Sets dimensions from `window.innerWidth/innerHeight`
3. Creates ImageData buffer
4. Fills with position-dependent gradient (R=x%, G=y%, B=50%)
5. Draws to canvas via `putImageData`

## Key Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Component structure | Forward-compatible | Prepares for viewport/interaction in Iteration 4 |
| Resize handling | Skip (mount only) | Comes free with `use_canvas_interaction` in Iteration 4 |
| Rendering method | ImageData pixels | Proves the exact pipeline used for fractals |
| Gradient code | Minimal inline | Disposable; replaced in Iteration 4 |
| Effects | Single combined | No resize = no need to separate dimensions from rendering |

## Dependencies

`web-sys` features required:
- `Window`
- `HtmlCanvasElement`
- `CanvasRenderingContext2d`
- `ImageData`

## Verification

**Compile:**
```bash
cargo clippy --all-targets --all-features -- -D warnings
cargo check --workspace
```

**Browser test:**
- Open `http://127.0.0.1:8000/fractalwonder`
- Gradient fills entire viewport
- Red increases left to right
- Green increases top to bottom
- Blue constant (purple tint)

## What This Proves

- Canvas element mounts correctly
- Dimensions set to full viewport
- ImageData pixel manipulation works
- Foundation ready for Iteration 2 (UI panel)
