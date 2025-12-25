# Palette Editor Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement the palette editor slide-out panel with editable name, Cancel/Apply/Duplicate/Delete buttons, confirmation dialogs, and live preview.

**Architecture:** State lives in app.rs (`editor_state: RwSignal<Option<PaletteEditorState>>`). When editor is open, canvas uses `working_palette`. New components: `PaletteEditor`, `ConfirmDialog`. Existing `UIPanel` and `PaletteMenu` wire up to open the editor.

**Tech Stack:** Rust, Leptos 0.6, Tailwind CSS, WASM

---

## Task 1: Create PaletteEditorState and EditMode

**Files:**
- Create: `fractalwonder-ui/src/components/palette_editor_state.rs`
- Modify: `fractalwonder-ui/src/components/mod.rs`

**Step 1: Create the state module**

```rust
// fractalwonder-ui/src/components/palette_editor_state.rs
//! State management for the palette editor.

use crate::rendering::colorizers::Palette;

/// Edit mode determines button behavior and dirty state logic.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditMode {
    /// Editing an existing palette (custom or shadowed factory)
    Edit,
    /// Creating a new palette (duplicate, new, or editing factory default)
    Duplicate,
}

/// State for an active palette editing session.
#[derive(Clone, Debug)]
pub struct PaletteEditorState {
    /// Snapshot at open (for cancel/revert and dirty check)
    pub source_palette: Palette,
    /// Live edits (renderer uses this while editor is open)
    pub working_palette: Palette,
    /// Determines button behavior
    pub edit_mode: EditMode,
}

impl PaletteEditorState {
    /// Create state for editing an existing palette.
    pub fn edit(palette: Palette) -> Self {
        Self {
            source_palette: palette.clone(),
            working_palette: palette,
            edit_mode: EditMode::Edit,
        }
    }

    /// Create state for duplicating a palette with a new name.
    pub fn duplicate(palette: Palette, new_name: String) -> Self {
        Self {
            source_palette: palette.clone(),
            working_palette: Palette {
                name: new_name,
                ..palette
            },
            edit_mode: EditMode::Duplicate,
        }
    }

    /// Check if there are unsaved changes.
    ///
    /// In Duplicate mode, always dirty (new palette doesn't exist yet).
    /// In Edit mode, dirty if working differs from source.
    pub fn is_dirty(&self) -> bool {
        matches!(self.edit_mode, EditMode::Duplicate)
            || self.working_palette != self.source_palette
    }

    /// Check if source palette shadows a factory default.
    pub fn shadows_factory(&self, factory_names: &[String]) -> bool {
        factory_names.contains(&self.source_palette.name)
    }

    /// Transition to duplicate mode (Duplicate button clicked mid-edit).
    /// Preserves current working_palette changes under a new name.
    pub fn to_duplicate(&self, new_name: String) -> Self {
        Self {
            source_palette: self.source_palette.clone(),
            working_palette: Palette {
                name: new_name,
                ..self.working_palette.clone()
            },
            edit_mode: EditMode::Duplicate,
        }
    }
}

/// Generate a unique palette name: "X Copy", "X Copy (1)", etc.
pub fn generate_unique_name(base: &str, existing: &[String]) -> String {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn test_palette(name: &str) -> Palette {
        Palette {
            name: name.to_string(),
            ..Palette::default()
        }
    }

    #[test]
    fn edit_mode_not_dirty_initially() {
        let state = PaletteEditorState::edit(test_palette("Test"));
        assert!(!state.is_dirty());
    }

    #[test]
    fn edit_mode_dirty_after_change() {
        let mut state = PaletteEditorState::edit(test_palette("Test"));
        state.working_palette.smooth_enabled = !state.working_palette.smooth_enabled;
        assert!(state.is_dirty());
    }

    #[test]
    fn duplicate_mode_always_dirty() {
        let state = PaletteEditorState::duplicate(test_palette("Test"), "Test Copy".to_string());
        assert!(state.is_dirty());
    }

    #[test]
    fn shadows_factory_true_when_name_matches() {
        let state = PaletteEditorState::edit(test_palette("Classic"));
        let factory_names = vec!["Classic".to_string(), "Ocean".to_string()];
        assert!(state.shadows_factory(&factory_names));
    }

    #[test]
    fn shadows_factory_false_for_custom() {
        let state = PaletteEditorState::edit(test_palette("My Custom"));
        let factory_names = vec!["Classic".to_string(), "Ocean".to_string()];
        assert!(!state.shadows_factory(&factory_names));
    }

    #[test]
    fn generate_unique_name_simple() {
        let existing = vec!["Ocean".to_string()];
        assert_eq!(generate_unique_name("Ocean", &existing), "Ocean Copy");
    }

    #[test]
    fn generate_unique_name_increments() {
        let existing = vec![
            "Ocean".to_string(),
            "Ocean Copy".to_string(),
            "Ocean Copy (1)".to_string(),
        ];
        assert_eq!(generate_unique_name("Ocean", &existing), "Ocean Copy (2)");
    }

    #[test]
    fn to_duplicate_preserves_changes() {
        let mut state = PaletteEditorState::edit(test_palette("Test"));
        state.working_palette.histogram_enabled = true;
        let dup = state.to_duplicate("Test Copy".to_string());
        assert_eq!(dup.edit_mode, EditMode::Duplicate);
        assert_eq!(dup.working_palette.name, "Test Copy");
        assert!(dup.working_palette.histogram_enabled);
    }
}
```

**Step 2: Run tests to verify they pass**

Run: `cargo test --package fractalwonder-ui palette_editor_state -- --nocapture`
Expected: All tests pass

**Step 3: Update mod.rs to export**

Add to `fractalwonder-ui/src/components/mod.rs`:

```rust
mod palette_editor_state;

pub use palette_editor_state::{generate_unique_name, EditMode, PaletteEditorState};
```

**Step 4: Run cargo check**

Run: `cargo check --package fractalwonder-ui`
Expected: No errors

**Step 5: Commit**

```bash
git add fractalwonder-ui/src/components/palette_editor_state.rs fractalwonder-ui/src/components/mod.rs
git commit -m "feat(palette-editor): add PaletteEditorState and EditMode"
```

---

## Task 2: Create ConfirmDialog Component

**Files:**
- Create: `fractalwonder-ui/src/components/confirm_dialog.rs`
- Modify: `fractalwonder-ui/src/components/mod.rs`

**Step 1: Create the component**

```rust
// fractalwonder-ui/src/components/confirm_dialog.rs
//! Reusable confirmation dialog component.

use leptos::*;

/// A centered modal dialog for confirming destructive actions.
#[component]
pub fn ConfirmDialog(
    /// Whether the dialog is visible
    #[prop(into)]
    visible: Signal<bool>,
    /// Dialog title
    #[prop(into)]
    title: String,
    /// Dialog message
    #[prop(into)]
    message: String,
    /// Cancel button label (e.g., "Cancel")
    #[prop(into)]
    cancel_label: String,
    /// Confirm button label (e.g., "Delete", "Reset", "Continue")
    #[prop(into)]
    confirm_label: String,
    /// Called when cancel is clicked
    on_cancel: Callback<()>,
    /// Called when confirm is clicked
    on_confirm: Callback<()>,
) -> impl IntoView {
    view! {
        <Show when=move || visible.get()>
            // Backdrop
            <div
                class="fixed inset-0 z-[100] bg-black/50 backdrop-blur-sm flex items-center justify-center"
                on:click=move |_| on_cancel.call(())
            >
                // Dialog
                <div
                    class="bg-black/95 border border-white/10 rounded-lg p-4 max-w-sm mx-4 space-y-4"
                    on:click=|e| e.stop_propagation()
                >
                    <h3 class="text-white text-sm font-medium">{title}</h3>
                    <p class="text-gray-300 text-sm">{message}</p>
                    <div class="flex gap-2">
                        <button
                            class="flex-1 px-3 py-1.5 rounded-lg border border-white/20 text-white text-sm hover:bg-white/10 transition-colors"
                            on:click=move |_| on_cancel.call(())
                        >
                            {cancel_label}
                        </button>
                        <button
                            class="flex-1 px-3 py-1.5 rounded-lg bg-white/20 text-white text-sm hover:bg-white/30 transition-colors"
                            on:click=move |_| on_confirm.call(())
                        >
                            {confirm_label}
                        </button>
                    </div>
                </div>
            </div>
        </Show>
    }
}
```

**Step 2: Update mod.rs to export**

Add to `fractalwonder-ui/src/components/mod.rs`:

```rust
mod confirm_dialog;

pub use confirm_dialog::ConfirmDialog;
```

**Step 3: Run cargo check**

Run: `cargo check --package fractalwonder-ui`
Expected: No errors

**Step 4: Commit**

```bash
git add fractalwonder-ui/src/components/confirm_dialog.rs fractalwonder-ui/src/components/mod.rs
git commit -m "feat(palette-editor): add ConfirmDialog component"
```

---

## Task 3: Create PaletteEditor Component (Basic Structure)

**Files:**
- Create: `fractalwonder-ui/src/components/palette_editor.rs`
- Modify: `fractalwonder-ui/src/components/mod.rs`

**Step 1: Create the basic component with slide animation and header**

```rust
// fractalwonder-ui/src/components/palette_editor.rs
//! Slide-out palette editor panel.

use crate::components::{ConfirmDialog, EditMode, PaletteEditorState};
use crate::rendering::colorizers::Palette;
use leptos::*;

/// Which confirmation dialog is currently shown (if any).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DialogKind {
    Cancel,
    Delete,
    Reset,
}

/// Slide-out panel for editing palette settings.
#[component]
pub fn PaletteEditor(
    /// Editor state (None = closed)
    state: RwSignal<Option<PaletteEditorState>>,
    /// Active palette (updated on apply/cancel/delete/reset)
    active_palette: RwSignal<Palette>,
    /// All palette names (for unique name generation and factory check)
    #[prop(into)]
    all_palette_names: Signal<Vec<String>>,
    /// Factory palette names (for shadows_factory check)
    #[prop(into)]
    factory_names: Signal<Vec<String>>,
) -> impl IntoView {
    // Local state for name editing
    let (is_editing_name, set_is_editing_name) = create_signal(false);
    let (name_input, set_name_input) = create_signal(String::new());

    // Dialog state
    let (dialog_kind, set_dialog_kind) = create_signal(None::<DialogKind>);

    // Derived: is editor visible?
    let is_visible = Signal::derive(move || state.get().is_some());

    // Derived: current working palette name
    let palette_name = Signal::derive(move || {
        state.get().map(|s| s.working_palette.name.clone()).unwrap_or_default()
    });

    // Derived: is dirty?
    let is_dirty = Signal::derive(move || {
        state.get().map(|s| s.is_dirty()).unwrap_or(false)
    });

    // Derived: edit mode
    let edit_mode = Signal::derive(move || {
        state.get().map(|s| s.edit_mode).unwrap_or(EditMode::Edit)
    });

    // Derived: shadows factory?
    let shadows_factory = Signal::derive(move || {
        state.get()
            .map(|s| s.shadows_factory(&factory_names.get()))
            .unwrap_or(false)
    });

    // Derived: delete button label and enabled state
    let delete_button_label = Signal::derive(move || {
        if edit_mode.get() == EditMode::Duplicate {
            "Delete".to_string()
        } else if shadows_factory.get() {
            "Reset".to_string()
        } else {
            "Delete".to_string()
        }
    });

    let delete_button_enabled = Signal::derive(move || {
        edit_mode.get() == EditMode::Edit
    });

    // Sync name_input when state changes
    create_effect(move |_| {
        if let Some(s) = state.get() {
            set_name_input.set(s.working_palette.name.clone());
        }
    });

    // Handle name edit completion
    let commit_name = move || {
        set_is_editing_name.set(false);
        let new_name = name_input.get().trim().to_string();
        if !new_name.is_empty() {
            state.update(|opt| {
                if let Some(s) = opt {
                    s.working_palette.name = new_name;
                }
            });
        }
    };

    // Actions
    let on_apply = move |_| {
        if let Some(s) = state.get() {
            let _ = s.working_palette.save();
            active_palette.set(s.working_palette.clone());
            state.set(None);
        }
    };

    let on_cancel_click = move |_| {
        if is_dirty.get() {
            set_dialog_kind.set(Some(DialogKind::Cancel));
        } else {
            state.set(None);
        }
    };

    let on_cancel_confirm = move |_| {
        if let Some(s) = state.get() {
            active_palette.set(s.source_palette.clone());
        }
        set_dialog_kind.set(None);
        state.set(None);
    };

    let on_duplicate = move |_| {
        if let Some(s) = state.get() {
            let names = all_palette_names.get();
            let new_name = crate::components::generate_unique_name(&s.working_palette.name, &names);
            state.set(Some(s.to_duplicate(new_name)));
        }
    };

    let on_delete_click = move |_| {
        if shadows_factory.get() {
            set_dialog_kind.set(Some(DialogKind::Reset));
        } else {
            set_dialog_kind.set(Some(DialogKind::Delete));
        }
    };

    let on_delete_confirm = move |_| {
        if let Some(s) = state.get() {
            Palette::delete(&s.source_palette.name);
            // Get next available palette
            let factory = factory_names.get();
            let next = factory.first().cloned().unwrap_or_else(|| "Default".to_string());
            spawn_local(async move {
                if let Some(pal) = Palette::get(&next).await {
                    active_palette.set(pal);
                } else {
                    active_palette.set(Palette::default());
                }
            });
        }
        set_dialog_kind.set(None);
        state.set(None);
    };

    let on_reset_confirm = move |_| {
        if let Some(s) = state.get() {
            Palette::delete(&s.source_palette.name);
            let name = s.source_palette.name.clone();
            spawn_local(async move {
                if let Some(factory_pal) = Palette::get(&name).await {
                    active_palette.set(factory_pal);
                }
            });
        }
        set_dialog_kind.set(None);
        state.set(None);
    };

    let on_dialog_cancel = move |_| {
        set_dialog_kind.set(None);
    };

    // Dialog content
    let dialog_title = Signal::derive(move || {
        match dialog_kind.get() {
            Some(DialogKind::Cancel) => "Unsaved Changes".to_string(),
            Some(DialogKind::Delete) => "Delete Palette".to_string(),
            Some(DialogKind::Reset) => "Reset Palette".to_string(),
            None => String::new(),
        }
    });

    let dialog_message = Signal::derive(move || {
        let name = palette_name.get();
        match dialog_kind.get() {
            Some(DialogKind::Cancel) => "There are unsaved changes that will be lost. Continue?".to_string(),
            Some(DialogKind::Delete) => format!("Are you sure you want to delete \"{}\"?", name),
            Some(DialogKind::Reset) => format!("Are you sure you want to reset \"{}\" to factory defaults?", name),
            None => String::new(),
        }
    });

    let dialog_confirm_label = Signal::derive(move || {
        match dialog_kind.get() {
            Some(DialogKind::Cancel) => "Continue".to_string(),
            Some(DialogKind::Delete) => "Delete".to_string(),
            Some(DialogKind::Reset) => "Reset".to_string(),
            None => String::new(),
        }
    });

    view! {
        // Panel
        <div
            class=move || format!(
                "fixed top-0 right-0 h-full w-[380px] bg-black/90 backdrop-blur-md border-l border-white/10 \
                 transition-transform duration-300 z-[60] overflow-y-auto {}",
                if is_visible.get() { "translate-x-0" } else { "translate-x-full" }
            )
        >
            <div class="p-4 space-y-3">
                // Header: Name
                <div class="space-y-3">
                    {move || {
                        if is_editing_name.get() {
                            view! {
                                <input
                                    type="text"
                                    class="w-full bg-white/5 border border-white/20 rounded-lg px-3 py-1.5 text-white text-sm outline-none focus:border-white/40"
                                    prop:value=move || name_input.get()
                                    on:input=move |ev| set_name_input.set(event_target_value(&ev))
                                    on:blur=move |_| commit_name()
                                    on:keydown=move |ev| {
                                        if ev.key() == "Enter" {
                                            commit_name();
                                        } else if ev.key() == "Escape" {
                                            set_is_editing_name.set(false);
                                            // Reset to current name
                                            if let Some(s) = state.get() {
                                                set_name_input.set(s.working_palette.name.clone());
                                            }
                                        }
                                    }
                                    autofocus
                                />
                            }.into_view()
                        } else {
                            view! {
                                <h2
                                    class="text-white cursor-pointer hover:text-gray-200 transition-colors"
                                    on:click=move |_| set_is_editing_name.set(true)
                                >
                                    {move || palette_name.get()}
                                </h2>
                            }.into_view()
                        }
                    }}

                    // Row 1: Cancel / Apply
                    <div class="flex gap-2">
                        <button
                            class="flex-1 flex items-center justify-center gap-1.5 px-3 py-1.5 rounded-lg border border-white/20 text-white text-sm hover:bg-white/10 transition-colors"
                            on:click=on_cancel_click
                        >
                            <XIcon />
                            "Cancel"
                        </button>
                        <button
                            class=move || format!(
                                "flex-1 flex items-center justify-center gap-1.5 px-3 py-1.5 rounded-lg bg-white/20 text-white text-sm transition-colors {}",
                                if is_dirty.get() { "hover:bg-white/30" } else { "opacity-30 cursor-not-allowed" }
                            )
                            prop:disabled=move || !is_dirty.get()
                            on:click=on_apply
                        >
                            <CheckIcon />
                            "Apply"
                        </button>
                    </div>

                    // Row 2: Duplicate / Delete
                    <div class="flex gap-2">
                        <button
                            class="flex-1 flex items-center justify-center gap-1.5 px-3 py-1.5 rounded-lg border border-white/10 text-white text-sm hover:bg-white/10 transition-colors"
                            on:click=on_duplicate
                        >
                            <CopyIcon />
                            "Duplicate"
                        </button>
                        <button
                            class=move || format!(
                                "flex-1 flex items-center justify-center gap-1.5 px-3 py-1.5 rounded-lg border border-white/10 text-white text-sm transition-colors {}",
                                if delete_button_enabled.get() { "hover:bg-white/10" } else { "opacity-30 cursor-not-allowed" }
                            )
                            prop:disabled=move || !delete_button_enabled.get()
                            on:click=on_delete_click
                        >
                            <TrashIcon />
                            {move || delete_button_label.get()}
                        </button>
                    </div>
                </div>

                // Placeholder for future sections
                <div class="border border-white/10 rounded-lg p-3">
                    <p class="text-gray-500 text-xs">"Palette controls coming soon..."</p>
                </div>
            </div>
        </div>

        // Confirmation Dialog
        <ConfirmDialog
            visible=Signal::derive(move || dialog_kind.get().is_some())
            title=dialog_title
            message=dialog_message
            cancel_label="Cancel"
            confirm_label=dialog_confirm_label
            on_cancel=Callback::new(on_dialog_cancel)
            on_confirm=Callback::new(move |_| {
                match dialog_kind.get() {
                    Some(DialogKind::Cancel) => on_cancel_confirm(()),
                    Some(DialogKind::Delete) => on_delete_confirm(()),
                    Some(DialogKind::Reset) => on_reset_confirm(()),
                    None => {}
                }
            })
        />
    }
}

// Simple SVG icons (inline to avoid dependencies)
#[component]
fn XIcon() -> impl IntoView {
    view! {
        <svg class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M18 6 6 18"/>
            <path d="m6 6 12 12"/>
        </svg>
    }
}

#[component]
fn CheckIcon() -> impl IntoView {
    view! {
        <svg class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <polyline points="20 6 9 17 4 12"/>
        </svg>
    }
}

#[component]
fn CopyIcon() -> impl IntoView {
    view! {
        <svg class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <rect width="14" height="14" x="8" y="8" rx="2" ry="2"/>
            <path d="M4 16c-1.1 0-2-.9-2-2V4c0-1.1.9-2 2-2h10c1.1 0 2 .9 2 2"/>
        </svg>
    }
}

#[component]
fn TrashIcon() -> impl IntoView {
    view! {
        <svg class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M3 6h18"/>
            <path d="M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6"/>
            <path d="M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2"/>
        </svg>
    }
}
```

**Step 2: Update mod.rs to export**

Add to `fractalwonder-ui/src/components/mod.rs`:

```rust
mod palette_editor;

pub use palette_editor::PaletteEditor;
```

**Step 3: Run cargo check**

Run: `cargo check --package fractalwonder-ui`
Expected: No errors

**Step 4: Commit**

```bash
git add fractalwonder-ui/src/components/palette_editor.rs fractalwonder-ui/src/components/mod.rs
git commit -m "feat(palette-editor): add PaletteEditor component with buttons and dialogs"
```

---

## Task 4: Update UIPanel to Wire Up Editor Callbacks

**Files:**
- Modify: `fractalwonder-ui/src/components/ui_panel.rs:125-140`

**Step 1: Update UIPanel props and callbacks**

Replace the TODO callbacks in `ui_panel.rs` (around lines 125-140):

Change:
```rust
on_new=Callback::new(|_| {
    // TODO: Open PaletteEditor slide-out panel
    // See docs/ux-palette-editor/ARCHITECTURE.md
})
on_edit=Callback::new(|_id| {
    // TODO: Open PaletteEditor for palette `_id`
    // See docs/ux-palette-editor/ARCHITECTURE.md
})
```

To:
```rust
on_new=on_new
on_edit=on_edit
```

And add props to the component signature. The full change:

At the top of `UIPanel` component, add new props:

```rust
/// Callback when "New..." is clicked in palette menu
on_new: Callback<()>,
/// Callback when edit icon is clicked (receives palette name)
on_edit: Callback<String>,
```

**Step 2: Run cargo check**

Run: `cargo check --package fractalwonder-ui`
Expected: No errors

**Step 3: Commit**

```bash
git add fractalwonder-ui/src/components/ui_panel.rs
git commit -m "feat(palette-editor): wire up on_new and on_edit callbacks in UIPanel"
```

---

## Task 5: Integrate PaletteEditor in App

**Files:**
- Modify: `fractalwonder-ui/src/app.rs`

**Step 1: Add imports**

Add to imports at top of `app.rs`:

```rust
use crate::components::{CircularProgress, InteractiveCanvas, PaletteEditor, Toast, UIPanel};
use crate::components::{generate_unique_name, PaletteEditorState};
```

**Step 2: Add editor state signal**

After the `palette_list` and `palette_options` setup (around line 85), add:

```rust
// Palette editor state (None = closed)
let (editor_state, set_editor_state) = create_signal(None::<PaletteEditorState>);

// Factory palette names (for shadows_factory check)
let factory_names = Signal::derive(move || {
    palette_list.get().iter().map(|p| p.name.clone()).collect::<Vec<_>>()
});

// All palette names (factory + any custom from localStorage)
// For now, just use factory names; localStorage enumeration can be added later
let all_palette_names = factory_names;
```

**Step 3: Update render_palette to use editor state**

Find the line that passes `palette` to `InteractiveCanvas` and update it. Change:

```rust
palette=palette.into()
```

To use a derived signal that checks editor state:

```rust
// Derive render palette: use working_palette when editing, else active palette
let render_palette = Signal::derive(move || {
    if let Some(state) = editor_state.get() {
        state.working_palette.clone()
    } else {
        palette.get()
    }
});
```

And in the view:
```rust
palette=render_palette
```

**Step 4: Add editor open callbacks**

Before the view! macro, add:

```rust
let on_palette_new = Callback::new(move |_: ()| {
    let names = all_palette_names.get();
    let new_name = generate_unique_name("Custom", &names);
    set_editor_state.set(Some(PaletteEditorState::duplicate(
        Palette::default(),
        new_name,
    )));
});

let on_palette_edit = Callback::new(move |name: String| {
    let palette_val = palette.get_untracked();
    let factory = factory_names.get_untracked();

    // If editing a factory palette that hasn't been shadowed, it's a duplicate
    let is_factory = factory.contains(&name);
    let state = if is_factory && Palette::load(&name).is_none() {
        // Factory palette, not shadowed - treat as duplicate (but keep name for shadowing)
        let names = all_palette_names.get_untracked();
        PaletteEditorState::duplicate(palette_val, name)
    } else {
        // Custom palette or shadowed factory - edit mode
        PaletteEditorState::edit(palette_val)
    };
    set_editor_state.set(Some(state));
});
```

**Step 5: Update UIPanel in view**

Pass the new callbacks to UIPanel:

```rust
<UIPanel
    // ... existing props ...
    on_new=on_palette_new
    on_edit=on_palette_edit
/>
```

**Step 6: Add PaletteEditor to view**

Add after `UIPanel` and before `Toast`:

```rust
<PaletteEditor
    state=editor_state
    active_palette=set_palette
    all_palette_names=all_palette_names
    factory_names=factory_names
/>
```

Wait, `active_palette` needs to be an `RwSignal`, not `WriteSignal`. Let me fix this.

Actually, looking at the component signature, `active_palette: RwSignal<Palette>`. We have `(palette, set_palette)` as separate signals. We need to create an RwSignal instead.

Change:
```rust
let (palette, set_palette) = create_signal(Palette::default());
```

To:
```rust
let palette = create_rw_signal(Palette::default());
```

And update all usages:
- `palette.get()` stays the same
- `set_palette.set(...)` becomes `palette.set(...)`

**Step 7: Run cargo check**

Run: `cargo check --package fractalwonder-ui`
Expected: No errors

**Step 8: Manual browser test**

1. Open http://localhost:8080
2. Click Palette dropdown → click pencil icon on any palette
3. Verify editor slides in from right
4. Verify name is displayed and click-to-edit works
5. Verify Cancel closes (with confirmation if dirty)
6. Verify Apply saves and closes
7. Verify Duplicate creates copy
8. Verify Delete/Reset works correctly

**Step 9: Commit**

```bash
git add fractalwonder-ui/src/app.rs
git commit -m "feat(palette-editor): integrate PaletteEditor in App with live preview"
```

---

## Task 6: Final Cleanup and Lint

**Step 1: Format code**

Run: `cargo fmt --all`

**Step 2: Run Clippy**

Run: `cargo clippy --all-targets --all-features -- -D warnings -W clippy::all`
Expected: No errors or warnings

**Step 3: Run all tests**

Run: `cargo test --workspace --all-targets --all-features -- --nocapture`
Expected: All tests pass

**Step 4: Final commit if any fixes**

```bash
git add -A
git commit -m "chore: lint and format palette editor code"
```

---

## Summary

| Task | Description | Files |
|------|-------------|-------|
| 1 | PaletteEditorState + EditMode | palette_editor_state.rs, mod.rs |
| 2 | ConfirmDialog component | confirm_dialog.rs, mod.rs |
| 3 | PaletteEditor component | palette_editor.rs, mod.rs |
| 4 | UIPanel callback wiring | ui_panel.rs |
| 5 | App integration | app.rs |
| 6 | Lint and cleanup | all |

Total: 6 tasks, ~5 commits
