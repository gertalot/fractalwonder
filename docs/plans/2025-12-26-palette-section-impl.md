# Palette Section Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add a collapsible "Palette" section to PaletteEditor with Histogram Equalization and Smooth Coloring checkboxes.

**Architecture:** Create reusable CollapsibleSection component, add chevron icons, integrate into PaletteEditor with derived signals for checkbox state. Changes update working_palette directly for live preview.

**Tech Stack:** Rust, Leptos 0.6, Tailwind CSS

---

### Task 1: Create CollapsibleSection Component

**Files:**
- Create: `fractalwonder-ui/src/components/collapsible_section.rs`

**Step 1: Create the component file with chevron icons and collapsible logic**

```rust
//! Reusable collapsible section component.

use leptos::*;

/// Collapsible section with header and expandable content.
#[component]
pub fn CollapsibleSection(
    /// Section title displayed in header
    title: &'static str,
    /// Expanded state signal
    expanded: RwSignal<bool>,
    /// Child content
    children: Children,
) -> impl IntoView {
    view! {
        <div class="border border-white/10 rounded-lg overflow-hidden">
            <button
                class="w-full flex items-center justify-between px-3 py-2 bg-white/5 \
                       hover:bg-white/10 transition-colors text-white text-sm"
                on:click=move |_| expanded.update(|v| *v = !*v)
            >
                <span>{title}</span>
                {move || if expanded.get() {
                    view! { <ChevronDownIcon /> }.into_view()
                } else {
                    view! { <ChevronRightIcon /> }.into_view()
                }}
            </button>

            <Show when=move || expanded.get()>
                <div class="p-3 space-y-3">
                    {children()}
                </div>
            </Show>
        </div>
    }
}

#[component]
fn ChevronDownIcon() -> impl IntoView {
    view! {
        <svg class="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor"
             stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="m6 9 6 6 6-6"/>
        </svg>
    }
}

#[component]
fn ChevronRightIcon() -> impl IntoView {
    view! {
        <svg class="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor"
             stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="m9 18 6-6-6-6"/>
        </svg>
    }
}
```

**Step 2: Verify file compiles**

Run: `cargo check -p fractalwonder-ui`
Expected: Error about module not declared (expected, we haven't added to mod.rs yet)

---

### Task 2: Export CollapsibleSection from mod.rs

**Files:**
- Modify: `fractalwonder-ui/src/components/mod.rs`

**Step 1: Read current mod.rs to understand structure**

Run: Read `fractalwonder-ui/src/components/mod.rs`

**Step 2: Add module declaration and re-export**

Add these lines in alphabetical order with existing modules:

```rust
mod collapsible_section;
pub use collapsible_section::CollapsibleSection;
```

**Step 3: Verify compilation**

Run: `cargo check -p fractalwonder-ui`
Expected: PASS (no errors)

**Step 4: Commit**

```bash
git add fractalwonder-ui/src/components/collapsible_section.rs fractalwonder-ui/src/components/mod.rs
git commit -m "feat(palette-editor): add CollapsibleSection component"
```

---

### Task 3: Add Palette Section to PaletteEditor

**Files:**
- Modify: `fractalwonder-ui/src/components/palette_editor.rs`

**Step 1: Add import for CollapsibleSection**

At the top of the file, add to the imports:

```rust
use crate::components::CollapsibleSection;
```

**Step 2: Add palette_expanded state signal**

After the existing state signals (around line 31), add:

```rust
// Collapsible section state
let palette_expanded = create_rw_signal(true);
```

**Step 3: Add derived signals for checkbox values**

After the existing derived signals (around line 60), add:

```rust
// Derived: checkbox values
let histogram_enabled = Signal::derive(move || {
    state.get().map(|s| s.working_palette.histogram_enabled).unwrap_or(false)
});

let smooth_enabled = Signal::derive(move || {
    state.get().map(|s| s.working_palette.smooth_enabled).unwrap_or(false)
});
```

**Step 4: Replace placeholder with Palette section**

Find the placeholder (around lines 314-317):

```rust
// Placeholder for future sections
<div class="border border-white/10 rounded-lg p-3">
    <p class="text-gray-500 text-xs">"Palette controls coming soon..."</p>
</div>
```

Replace with:

```rust
// Palette Section
<CollapsibleSection title="Palette" expanded=palette_expanded>
    <div class="space-y-1">
        <label class="flex items-center gap-2 px-2 py-1 rounded hover:bg-white/5 \
                      cursor-pointer transition-colors">
            <input
                type="checkbox"
                class="w-3.5 h-3.5 rounded accent-white"
                prop:checked=move || histogram_enabled.get()
                on:change=move |ev| {
                    let checked = event_target_checked(&ev);
                    state.update(|opt| {
                        if let Some(s) = opt {
                            s.working_palette.histogram_enabled = checked;
                        }
                    });
                }
            />
            <span class="text-white text-sm">"Histogram Equalization"</span>
        </label>

        <label class="flex items-center gap-2 px-2 py-1 rounded hover:bg-white/5 \
                      cursor-pointer transition-colors">
            <input
                type="checkbox"
                class="w-3.5 h-3.5 rounded accent-white"
                prop:checked=move || smooth_enabled.get()
                on:change=move |ev| {
                    let checked = event_target_checked(&ev);
                    state.update(|opt| {
                        if let Some(s) = opt {
                            s.working_palette.smooth_enabled = checked;
                        }
                    });
                }
            />
            <span class="text-white text-sm">"Smooth Coloring"</span>
        </label>
    </div>
</CollapsibleSection>
```

**Step 5: Verify compilation**

Run: `cargo check -p fractalwonder-ui`
Expected: PASS (no errors)

**Step 6: Commit**

```bash
git add fractalwonder-ui/src/components/palette_editor.rs
git commit -m "feat(palette-editor): add Palette section with checkboxes"
```

---

### Task 4: Run Full Quality Checks

**Step 1: Format code**

Run: `cargo fmt --all`

**Step 2: Run Clippy**

Run: `cargo clippy --all-targets --all-features -- -D warnings -W clippy::all`
Expected: PASS (no warnings)

**Step 3: Run tests**

Run: `cargo test --workspace --all-targets --all-features -- --nocapture`
Expected: PASS (all tests pass)

**Step 4: Verify in browser**

1. Open http://localhost:8080 (trunk serve should be running)
2. Click Palette button in bottom bar
3. Click edit icon on any palette
4. Verify: "Palette" section appears with chevron
5. Verify: Click header collapses/expands section
6. Verify: Both checkboxes visible and clickable
7. Verify: Toggling checkboxes enables Apply button (dirty state)

**Step 5: Final commit if any formatting changes**

```bash
git add -A
git commit -m "chore: format palette editor code"
```

---

## Summary

| Task | Description | Files |
|------|-------------|-------|
| 1 | Create CollapsibleSection component | collapsible_section.rs |
| 2 | Export from mod.rs | mod.rs |
| 3 | Add Palette section to PaletteEditor | palette_editor.rs |
| 4 | Quality checks and browser verification | - |
