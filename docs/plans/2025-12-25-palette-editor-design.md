# Palette Editor Panel Design

This document describes the design for the palette editor slide-out panel, including state management, button logic, confirmation dialogs, and component structure.

## Overview

The palette editor is a 380px slide-out panel from the right edge that provides:
- Editable palette name
- Cancel/Apply buttons (primary actions)
- Duplicate/Delete buttons (secondary actions)
- Live preview (canvas updates as you edit)
- Confirmation dialogs for destructive actions

## State Model

### App-Level State

```rust
// In app.rs
active_palette: RwSignal<Palette>,
editor_state: RwSignal<Option<PaletteEditorState>>,

// Derived
let palette_editor_active = Signal::derive(move || editor_state.get().is_some());
```

### Editor State

```rust
struct PaletteEditorState {
    source_palette: Palette,    // Snapshot at open (for cancel/revert/dirty check)
    working_palette: Palette,   // Live edits (renderer uses this)
    edit_mode: EditMode,
}

enum EditMode {
    Edit,      // Editing existing palette
    Duplicate, // Creating new palette (duplicate, new, or editing factory default)
}

impl PaletteEditorState {
    fn is_dirty(&self) -> bool {
        matches!(self.edit_mode, EditMode::Duplicate)
            || self.working_palette != self.source_palette
    }

    fn shadows_factory(&self) -> bool {
        Palette::factory_defaults()
            .iter()
            .any(|p| p.name == self.source_palette.name)
    }
}
```

### Canvas Palette Selection

```rust
let render_palette = Signal::derive(move || {
    if let Some(state) = editor_state.get() {
        state.working_palette.clone()
    } else {
        active_palette.get()
    }
});
```

## State Transitions

### Opening the Editor

**Edit existing custom palette:**
```rust
fn open_edit(palette: Palette) -> PaletteEditorState {
    PaletteEditorState {
        source_palette: palette.clone(),
        working_palette: palette,
        edit_mode: EditMode::Edit,
    }
}
```

**Duplicate palette (from menu or button):**
```rust
fn open_duplicate(palette: Palette, all_names: &[String]) -> PaletteEditorState {
    PaletteEditorState {
        source_palette: palette.clone(),
        working_palette: Palette {
            name: generate_unique_name(&palette.name, all_names),
            ..palette
        },
        edit_mode: EditMode::Duplicate,
    }
}
```

**New palette:**
```rust
fn open_new(all_names: &[String]) -> PaletteEditorState {
    let default = Palette::minimal_default();
    PaletteEditorState {
        source_palette: default.clone(),
        working_palette: Palette {
            name: generate_unique_name("Custom", all_names),
            ..default
        },
        edit_mode: EditMode::Duplicate,
    }
}
```

**Edit factory default (creates shadow):**
- Same as `open_edit`, but since it shadows a factory palette, the Delete button shows "Reset"

### Mid-Edit Actions

**Duplicate current (button clicked while editing):**
```rust
fn duplicate_current(state: &PaletteEditorState, all_names: &[String]) -> PaletteEditorState {
    PaletteEditorState {
        source_palette: state.source_palette.clone(),
        working_palette: Palette {
            name: generate_unique_name(&state.working_palette.name, all_names),
            ..state.working_palette.clone() // Preserves unsaved changes
        },
        edit_mode: EditMode::Duplicate,
    }
}
```

**Edit any value:**
- Update `working_palette` field
- Canvas immediately re-renders with new palette
- `is_dirty` recalculates automatically

### Closing Actions

**Apply:**
```rust
fn apply(state: PaletteEditorState, active_palette: RwSignal<Palette>, editor_state: RwSignal<Option<...>>) {
    state.working_palette.save();              // Persist to localStorage
    active_palette.set(state.working_palette); // Update canvas
    editor_state.set(None);                    // Close editor
}
```

**Cancel:**
```rust
fn cancel(state: PaletteEditorState, active_palette: RwSignal<Palette>, editor_state: RwSignal<Option<...>>) {
    // Note: Caller must show confirmation dialog if is_dirty
    active_palette.set(state.source_palette);  // Revert canvas
    editor_state.set(None);                    // Close editor
}
```

**Delete (custom palette only):**
```rust
fn delete(state: PaletteEditorState, active_palette: RwSignal<Palette>, editor_state: RwSignal<Option<...>>) {
    // Note: Caller must show confirmation dialog
    Palette::delete(&state.source_palette.name);
    active_palette.set(next_available_palette());
    editor_state.set(None);
}
```

**Reset (shadowed factory palette only):**
```rust
fn reset(state: PaletteEditorState, active_palette: RwSignal<Palette>, editor_state: RwSignal<Option<...>>) {
    // Note: Caller must show confirmation dialog
    Palette::delete(&state.source_palette.name);  // Remove localStorage shadow
    let factory = Palette::get_factory(&state.source_palette.name).unwrap();
    active_palette.set(factory);
    editor_state.set(None);
}
```

## Button States

| Button | Condition | State |
|--------|-----------|-------|
| Apply | `is_dirty = false` | Disabled |
| Apply | `is_dirty = true` | Enabled |
| Cancel | Always | Enabled |
| Duplicate | Always | Enabled |
| Delete | `edit_mode = Duplicate` | Disabled (greyed out) |
| Delete | `edit_mode = Edit` AND `shadows_factory()` | Shows "Reset", enabled |
| Delete | `edit_mode = Edit` AND custom | Shows "Delete", enabled |

## Confirmation Dialogs

### ConfirmDialog Component

```rust
#[component]
pub fn ConfirmDialog(
    visible: ReadSignal<bool>,
    title: String,
    message: String,
    cancel_label: String,
    confirm_label: String,
    on_cancel: Callback<()>,
    on_confirm: Callback<()>,
) -> impl IntoView
```

### Dialog Instances

| Trigger | Title | Message | Buttons |
|---------|-------|---------|---------|
| Cancel when dirty | "Unsaved Changes" | "There are unsaved changes that will be lost. Continue?" | Cancel / Continue |
| Delete custom | "Delete Palette" | "Are you sure you want to delete {name}?" | Cancel / Delete |
| Reset factory | "Reset Palette" | "Are you sure you want to reset {name} to factory defaults?" | Cancel / Reset |

## Component Structure

### New Files

```
fractalwonder-ui/src/components/
├── palette_editor.rs      # Main slide-out panel
├── confirm_dialog.rs      # Reusable modal
└── mod.rs                 # Add exports
```

### PaletteEditor Component

```rust
#[component]
pub fn PaletteEditor(
    state: RwSignal<Option<PaletteEditorState>>,
    active_palette: RwSignal<Palette>,
    all_palette_names: Signal<Vec<String>>,
) -> impl IntoView
```

### Layout Structure

```
PaletteEditor (380px, fixed right, full height)
├── Header
│   ├── Editable name (click-to-edit input)
│   ├── Row 1: [Cancel] [Apply]
│   └── Row 2: [Duplicate] [Delete/Reset]
└── (Future: collapsible sections)

ConfirmDialog (centered modal overlay)
├── Title
├── Message
└── [Cancel] [Confirm]
```

## Styling

### Panel

- Fixed position: `fixed top-0 right-0 h-full`
- Width: `w-[380px]`
- Background: `bg-black/90 backdrop-blur-md`
- Border: `border-l border-white/10`
- Animation: `transition-transform duration-300`
- Hidden: `translate-x-full`
- Visible: `translate-x-0`

### Buttons

Primary row (Cancel/Apply):
- Cancel: `border border-white/20 hover:bg-white/10`
- Apply: `bg-white/20 hover:bg-white/30`
- Disabled: `opacity-30 cursor-not-allowed`

Secondary row (Duplicate/Delete):
- Both: `border border-white/10 hover:bg-white/10`
- Disabled: `opacity-30 cursor-not-allowed`

### Editable Name

- Display: `text-white cursor-pointer hover:text-gray-200`
- Edit mode: `bg-white/5 border border-white/20 focus:border-white/40`

### ConfirmDialog

- Overlay: `fixed inset-0 bg-black/50 backdrop-blur-sm`
- Modal: `bg-black/90 border border-white/10 rounded-lg`
- Centered: `flex items-center justify-center`

## Utility Functions

```rust
/// Generate unique name: "X Copy", "X Copy (1)", "X Copy (2)", etc.
fn generate_unique_name(base: &str, existing: &[String]) -> String {
    let copy_name = format!("{} Copy", base);
    if !existing.contains(&copy_name) {
        return copy_name;
    }
    for i in 1.. {
        let name = format!("{} Copy ({})", base, i);
        if !existing.contains(&name) {
            return name;
        }
    }
    unreachable!()
}

/// Get next available palette when deleting current
fn next_available_palette() -> Palette {
    // Return first factory palette, or minimal default
    Palette::factory_defaults()
        .into_iter()
        .next()
        .unwrap_or_else(Palette::minimal_default)
}
```

## Integration

### In app.rs

```rust
// State
let (active_palette, set_active_palette) = create_signal(initial_palette);
let (editor_state, set_editor_state) = create_signal(None::<PaletteEditorState>);

// Derived signals
let palette_editor_active = Signal::derive(move || editor_state.get().is_some());
let render_palette = Signal::derive(move || {
    editor_state.get()
        .map(|s| s.working_palette)
        .unwrap_or_else(|| active_palette.get())
});
let all_palette_names = Signal::derive(move || {
    // Collect names from factory + localStorage palettes
});

// View
view! {
    <InteractiveCanvas palette=render_palette /* ... */ />
    <UIPanel
        on_edit=Callback::new(move |name| {
            let palette = Palette::get(&name).unwrap();
            set_editor_state.set(Some(open_edit(palette)));
        })
        on_new=Callback::new(move |_| {
            set_editor_state.set(Some(open_new(&all_palette_names.get())));
        })
        /* ... */
    />
    <PaletteEditor
        state=editor_state
        active_palette=active_palette.into()
        all_palette_names=all_palette_names
    />
}
```

## Notes

- Editing factory palette name creates a custom palette, leaving factory default visible
- Persistent palettes are NOT modified while editor is open, only on Apply
- working_palette is NOT stored to localStorage until Apply
- Duplicate and new palettes DO NOT EXIST until Apply
