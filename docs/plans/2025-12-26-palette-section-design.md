# Palette Section Design

Add the "Palette" collapsible section to PaletteEditor with Histogram Equalization and Smooth Coloring checkboxes.

## Scope

- Collapsible "Palette" section matching prototype
- Two checkboxes: Histogram Equalization, Smooth Coloring
- Reusable `CollapsibleSection` component for future Light Effects section
- Deferred: GradientEditor, CurveEditor (placeholders only)

## Components

### CollapsibleSection

**File:** `fractalwonder-ui/src/components/collapsible_section.rs`

```rust
#[component]
pub fn CollapsibleSection(
    title: &'static str,
    expanded: RwSignal<bool>,
    children: Children,
) -> impl IntoView
```

**Styling:**
- Container: `border border-white/10 rounded-lg overflow-hidden`
- Header: `w-full flex items-center justify-between px-3 py-2 bg-white/5 hover:bg-white/10`
- Chevron: rotates based on expanded state
- Content: `p-3 space-y-3`

### PaletteEditor Changes

**New state:**
```rust
let (palette_expanded, set_palette_expanded) = create_signal(true);
```

**Derived signals:**
```rust
let histogram_enabled = Signal::derive(move || {
    state.get().map(|s| s.working_palette.histogram_enabled).unwrap_or(false)
});

let smooth_enabled = Signal::derive(move || {
    state.get().map(|s| s.working_palette.smooth_enabled).unwrap_or(false)
});
```

**Update handlers:**
```rust
let set_histogram = move |checked: bool| {
    state.update(|opt| {
        if let Some(s) = opt { s.working_palette.histogram_enabled = checked; }
    });
};

let set_smooth = move |checked: bool| {
    state.update(|opt| {
        if let Some(s) = opt { s.working_palette.smooth_enabled = checked; }
    });
};
```

**Checkbox markup:**
```html
<label class="flex items-center gap-2 px-2 py-1 rounded hover:bg-white/5 cursor-pointer transition-colors">
    <input type="checkbox" class="w-3.5 h-3.5 rounded accent-white" />
    <span class="text-white text-sm">Label</span>
</label>
```

## Data Flow

1. Checkbox change → updates `working_palette` field
2. `working_palette` change → triggers re-render via Leptos reactivity
3. `is_dirty()` automatically detects changes (compares working vs source)
4. Live preview works via existing `active_palette` signal flow in App

## Files to Modify

| File | Change |
|------|--------|
| `components/collapsible_section.rs` | New component |
| `components/mod.rs` | Export CollapsibleSection |
| `components/palette_editor.rs` | Add Palette section, replace placeholder |

## Styling Reference (from prototype)

```css
/* Section */
border border-white/10 rounded-lg overflow-hidden

/* Header button */
w-full flex items-center justify-between px-3 py-2 bg-white/5 hover:bg-white/10 transition-colors text-white text-sm

/* Checkbox container */
space-y-1

/* Checkbox label */
flex items-center gap-2 px-2 py-1 rounded hover:bg-white/5 cursor-pointer transition-colors

/* Checkbox input */
w-3.5 h-3.5 rounded accent-white

/* Checkbox text */
text-white text-sm
```
