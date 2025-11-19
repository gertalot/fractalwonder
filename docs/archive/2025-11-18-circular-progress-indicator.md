# Circular Progress Indicator Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement a 24×24px circular pie chart progress indicator in the bottom-left corner that shows rendering progress when the UI panel is hidden.

**Architecture:** Create a new Leptos component (`CircularProgress`) that consumes the existing `RenderProgress` signal from `app.rs` and the `is_visible` signal from `use_ui_visibility()`. The component uses SVG path generation to create a pie chart that fills clockwise from 12 o'clock. Visibility is inverse to the UI panel with the same 300ms fade transition.

**Tech Stack:** Leptos 0.6+, Rust, SVG, Tailwind CSS

---

## Task 1: Create CircularProgress Component with Tests

**Files:**
- Create: `fractalwonder-ui/src/components/circular_progress.rs`

**Step 1: Write the failing test for SVG path generation**

Create the component file with tests first (TDD):

```rust
use leptos::*;

/// Generate SVG path for pie chart based on percentage
fn create_pie_path(percent: f64) -> String {
    if percent <= 0.0 {
        return String::new();
    }

    let angle = (percent / 100.0) * 360.0;
    let radians = (angle - 90.0).to_radians(); // -90 to start at 12 o'clock
    let end_x = 12.0 + 10.0 * radians.cos();
    let end_y = 12.0 + 10.0 * radians.sin();
    let large_arc = if angle > 180.0 { 1 } else { 0 };

    format!(
        "M 12 12 L 12 2 A 10 10 0 {} 1 {:.2} {:.2} Z",
        large_arc, end_x, end_y
    )
}

#[component]
pub fn CircularProgress(
    progress: Signal<crate::rendering::RenderProgress>,
    is_ui_visible: ReadSignal<bool>,
) -> impl IntoView {
    view! { <div></div> }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_pie_path_zero_percent() {
        let path = create_pie_path(0.0);
        assert_eq!(path, "");
    }

    #[test]
    fn test_create_pie_path_25_percent() {
        let path = create_pie_path(25.0);
        // At 25%, we're at 3 o'clock (90 degrees from 12 o'clock)
        // Starting at -90 degrees (12 o'clock), adding 90 degrees = 0 degrees (3 o'clock)
        // cos(0) = 1, sin(0) = 0
        // end_x = 12 + 10*1 = 22, end_y = 12 + 10*0 = 12
        assert!(path.contains("M 12 12 L 12 2 A 10 10 0 0 1 22.00 12.00 Z"));
    }

    #[test]
    fn test_create_pie_path_50_percent() {
        let path = create_pie_path(50.0);
        // At 50%, we're at 6 o'clock (180 degrees from 12 o'clock)
        // Starting at -90 degrees, adding 180 degrees = 90 degrees (6 o'clock)
        // cos(90deg) = 0, sin(90deg) = 1
        // end_x = 12 + 10*0 = 12, end_y = 12 + 10*1 = 22
        assert!(path.contains("M 12 12 L 12 2 A 10 10 0 0 1 12.00 22.00 Z"));
    }

    #[test]
    fn test_create_pie_path_75_percent() {
        let path = create_pie_path(75.0);
        // At 75%, we're at 9 o'clock (270 degrees from 12 o'clock)
        // Starting at -90 degrees, adding 270 degrees = 180 degrees (9 o'clock)
        // cos(180deg) = -1, sin(180deg) = 0
        // end_x = 12 + 10*(-1) = 2, end_y = 12 + 10*0 = 12
        assert!(path.contains("M 12 12 L 12 2 A 10 10 0 1 1 2.00 12.00 Z"));
    }

    #[test]
    fn test_create_pie_path_100_percent() {
        let path = create_pie_path(100.0);
        // At 100%, we're back at 12 o'clock (360 degrees)
        // Starting at -90 degrees, adding 360 degrees = 270 degrees (back to 12 o'clock)
        // Large arc flag should be 1 (>180 degrees)
        assert!(path.contains("A 10 10 0 1 1"));
    }
}
```

**Step 2: Run test to verify it passes**

The `create_pie_path` function implementation is already included, so tests should pass.

Run: `cargo test --package fractalwonder-ui --lib components::circular_progress::tests -- --nocapture`

Expected: All 5 tests PASS

**Step 3: Implement the component rendering logic**

Replace the placeholder component with the full implementation:

```rust
#[component]
pub fn CircularProgress(
    progress: Signal<crate::rendering::RenderProgress>,
    is_ui_visible: ReadSignal<bool>,
) -> impl IntoView {
    // Calculate progress percentage
    let progress_percent = create_memo(move |_| {
        let p = progress.get();
        if p.total_tiles > 0 {
            (p.completed_tiles as f64 / p.total_tiles as f64 * 100.0).min(100.0)
        } else {
            0.0
        }
    });

    // Visibility: show when rendering AND UI is hidden
    let should_show = create_memo(move |_| {
        let p = progress.get();
        p.total_tiles > 0 && !p.is_complete && !is_ui_visible.get()
    });

    let opacity_class = move || {
        if should_show.get() {
            "opacity-100"
        } else {
            "opacity-0"
        }
    };

    view! {
        <div
            class=move || format!(
                "fixed left-[28px] bottom-[24px] transition-opacity duration-300 pointer-events-none {}",
                opacity_class()
            )
        >
            <div class="w-6 h-6 bg-black/50 backdrop-blur-sm rounded-full flex items-center justify-center">
                <svg width="24" height="24" viewBox="0 0 24 24" class="transform">
                    // Background circle (unfilled portion)
                    <circle
                        cx="12"
                        cy="12"
                        r="10"
                        fill="none"
                        stroke="rgb(100,100,100)"
                        stroke-width="1"
                        opacity="0.2"
                    />

                    // Progress pie slice
                    <path
                        d={move || create_pie_path(progress_percent.get())}
                        fill="rgb(244,244,244)"
                    />
                </svg>
            </div>
        </div>
    }
}
```

**Step 4: Run Clippy and format**

Run: `cargo clippy --package fractalwonder-ui -- -D warnings && cargo fmt --package fractalwonder-ui`

Expected: No warnings or errors

**Step 5: Commit**

```bash
git add fractalwonder-ui/src/components/circular_progress.rs
git commit -m "feat: add CircularProgress component with SVG pie chart

- Implements SVG path generation for clockwise pie chart from 12 o'clock
- Tests for 0%, 25%, 50%, 75%, 100% progress
- Visibility inverse to UI panel (shows when UI hidden)
- Positioning aligned with info icon (28px left, 24px bottom)
- Styling matches UI panel (bg-black/50, backdrop-blur-sm)

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Task 2: Export CircularProgress Component

**Files:**
- Modify: `fractalwonder-ui/src/components/mod.rs`

**Step 1: Add module declaration and export**

Add the new component to the module:

```rust
pub mod circular_progress;
pub mod dropdown_menu;
pub mod interactive_canvas;
pub mod ui;

pub use circular_progress::CircularProgress;
pub use interactive_canvas::InteractiveCanvas;
pub use ui::UI;
```

**Step 2: Verify compilation**

Run: `cargo check --package fractalwonder-ui`

Expected: Compiles successfully

**Step 3: Commit**

```bash
git add fractalwonder-ui/src/components/mod.rs
git commit -m "feat: export CircularProgress component

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Task 3: Integrate CircularProgress into App

**Files:**
- Modify: `fractalwonder-ui/src/app.rs:1-2` (add import)
- Modify: `fractalwonder-ui/src/app.rs:268-292` (add component to view)

**Step 1: Add import at top of file**

After line 2, add the import:

```rust
use crate::components::interactive_canvas::{CanvasRendererTrait, InteractiveCanvas};
use crate::components::ui::UI;
use crate::components::CircularProgress;  // Add this line
use crate::hooks::fullscreen::toggle_fullscreen;
```

**Step 2: Add component to the view**

In the `view!` macro (around line 268-292), add the CircularProgress component after the UI component:

```rust
view! {
    <div class="relative w-screen h-screen overflow-hidden bg-black">
        <InteractiveCanvas
            canvas_renderer=canvas_renderer
            viewport=viewport
            set_viewport=set_viewport
            set_render_time_ms=set_render_time_ms
            natural_bounds=natural_bounds.into()
        />
        <UI
            info=renderer_info
            is_visible=ui_visibility.is_visible
            set_is_hovering=ui_visibility.set_is_hovering
            on_home_click=on_home_click
            on_fullscreen_click=on_fullscreen_click
            render_function_options=render_function_options.into()
            selected_renderer_id=Signal::derive(move || selected_renderer_id.get())
            on_renderer_select=move |id: String| set_selected_renderer_id.set(id)
            color_scheme_options=color_scheme_options.into()
            selected_color_scheme_id=Signal::derive(move || selected_color_scheme_id.get())
            on_color_scheme_select=on_color_scheme_select
            progress=progress.get().into()
        />
        <CircularProgress
            progress=progress.get().into()
            is_ui_visible=ui_visibility.is_visible
        />
    </div>
}
```

**Step 3: Verify compilation**

Run: `cargo check --workspace`

Expected: Compiles successfully

**Step 4: Run all tests**

Run: `cargo test --workspace -- --nocapture`

Expected: All tests pass

**Step 5: Commit**

```bash
git add fractalwonder-ui/src/app.rs
git commit -m "feat: integrate CircularProgress into App

Wire CircularProgress component to existing progress signal and UI
visibility. Component appears in bottom-left when UI is hidden.

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Task 4: Browser Verification and Testing

**Files:**
- No file changes

**Step 1: Start development server**

Run: `trunk serve`

Expected: Server starts on http://localhost:8080

**Step 2: Manual browser testing using chrome-devtools MCP**

Test the following scenarios:

1. **Initial state:**
   - Navigate to http://localhost:8080
   - Verify UI panel is visible
   - Verify circular progress is NOT visible

2. **UI fade out:**
   - Wait 2 seconds without moving mouse
   - Verify UI panel fades out
   - Verify circular progress fades in (if render in progress)
   - Verify alignment with where info icon was

3. **Mouse movement:**
   - Move mouse
   - Verify UI panel fades in
   - Verify circular progress fades out

4. **Progress accuracy:**
   - Zoom in to trigger long render
   - Observe circular progress fills clockwise from 12 o'clock
   - Verify progress percentage matches visual fill
   - Verify indicator disappears when render completes

5. **Styling verification:**
   - Inspect circular progress element
   - Verify background: black with 50% opacity, backdrop blur
   - Verify foreground: rgb(244,244,244)
   - Verify size: 24×24px
   - Verify position: 28px from left, 24px from bottom

**Step 3: Document any issues found**

If issues found, document them and create follow-up tasks.

Expected: All scenarios work as designed

**Step 4: Final verification commit**

```bash
git commit --allow-empty -m "test: verify CircularProgress browser behavior

Manual testing confirms:
- Inverse visibility to UI panel (300ms fade)
- Clockwise progress from 12 o'clock
- Correct positioning and styling
- Disappears when render complete

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Task 5: Run Full Quality Checks

**Files:**
- No file changes

**Step 1: Format all code**

Run: `cargo fmt --all`

Expected: Code formatted

**Step 2: Run Clippy**

Run: `cargo clippy --all-targets --all-features -- -D warnings -W clippy::all`

Expected: No warnings or errors

**Step 3: Check compilation**

Run: `cargo check --workspace --all-targets --all-features`

Expected: Compiles successfully

**Step 4: Run all tests**

Run: `cargo test --workspace --all-targets --all-features -- --nocapture`

Expected: All tests pass

**Step 5: Final commit**

```bash
git commit --allow-empty -m "chore: verify all quality checks pass

All checks pass:
- cargo fmt
- cargo clippy (no warnings)
- cargo check (compiles)
- cargo test (all pass)

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Summary

This plan implements the circular progress indicator in 5 tasks:

1. **Create component with tests** - TDD approach, test SVG path generation first
2. **Export component** - Make it available to the app
3. **Integrate into App** - Wire to existing signals
4. **Browser verification** - Manual testing with chrome-devtools
5. **Quality checks** - Ensure all tests pass and code is clean

**Key principles applied:**
- **TDD:** Tests written first for SVG path generation
- **DRY:** Reuses existing RenderProgress signal and UI visibility logic
- **YAGNI:** No unnecessary features, minimal implementation
- **Frequent commits:** Each task has its own commit
