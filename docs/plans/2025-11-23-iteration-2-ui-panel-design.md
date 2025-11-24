# Iteration 2: UI Panel Design

## Goal

Add a UI panel overlay with autohide, fullscreen toggle, info button, and canvas dimensions display.

## Components

### UIPanel

Bottom bar container that:
- Fades out after 2s mouse idle
- Fades in on mouse movement
- Stays visible when hovering over the panel itself

Layout:
```
┌─────────────────────────────────────────────────┐
│ [i]                  Canvas: 1920×1080      [⛶] │
└─────────────────────────────────────────────────┘
  Left                    Center               Right
```

### InfoButton

- Circular button with "i" icon
- Click toggles popover above the button
- Popover content: "Fractal Wonder" title, usage hint, GitHub link

### FullscreenButton

- Circular button with maximize/minimize icon
- Icon changes based on fullscreen state
- Click toggles document fullscreen mode

## Hooks

### use_fullscreen (port from archive)

```rust
pub fn use_fullscreen() -> (ReadSignal<bool>, impl Fn())
```

- Returns `(is_fullscreen, toggle_fullscreen)`
- Listens to `fullscreenchange` event to track state
- Toggle function requests/exits fullscreen

### use_ui_visibility (port from archive)

```rust
pub fn use_ui_visibility() -> UiVisibility
```

Returns:
- `is_visible: ReadSignal<bool>` - whether panel should show
- `set_is_hovering: WriteSignal<bool>` - set by panel on mouse enter/leave

Behavior:
- Starts visible
- After 2s idle → sets `is_visible` to false (unless hovering)
- On mouse move → sets `is_visible` to true, resets timer

## Data Flow

```
App
├── InteractiveCanvas
│   └── exposes: canvas_size: Signal<(u32, u32)>
└── UIPanel
    ├── receives: canvas_size
    ├── owns: use_ui_visibility()
    ├── InfoButton
    └── FullscreenButton
        └── uses: use_fullscreen()
```

## Files to Create/Modify

**Create:**
- `fractalwonder-ui/src/hooks/fullscreen.rs`
- `fractalwonder-ui/src/hooks/ui_visibility.rs`
- `fractalwonder-ui/src/components/ui_panel.rs`
- `fractalwonder-ui/src/components/info_button.rs`
- `fractalwonder-ui/src/components/fullscreen_button.rs`

**Modify:**
- `fractalwonder-ui/src/hooks/mod.rs` - export hooks
- `fractalwonder-ui/src/components/mod.rs` - export components
- `fractalwonder-ui/src/components/interactive_canvas.rs` - expose canvas size signal
- `fractalwonder-ui/src/app.rs` - compose UIPanel with InteractiveCanvas

## Browser Tests

- See UI panel overlay on canvas
- Click fullscreen button → app goes fullscreen
- Leave mouse idle 2s → UI panel fades out
- Move mouse → UI panel fades back in
- Click "i" button → info popover appears
- Panel shows "Canvas: 1920 x 1080" (or current size)

## Unit Tests

- `use_ui_visibility`: autohide signal responds to mouse activity timeout
- `use_fullscreen`: toggle calls correct browser API
