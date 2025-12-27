//! Slide-out palette editor panel.

use crate::components::{
    CollapsibleSection, ConfirmDialog, CurveEditor, EditMode, GradientEditor, LightingControl,
    LightingSlider, PaletteEditorState,
};
use crate::rendering::colorizers::{Curve, Gradient, Palette};
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

    // Collapsible section state
    let palette_expanded = create_rw_signal(true);
    let light_effects_expanded = create_rw_signal(true);

    // Derived: is editor visible?
    let is_visible = Signal::derive(move || state.get().is_some());

    // Derived: current working palette name
    let palette_name = Signal::derive(move || {
        state
            .get()
            .map(|s| s.working_palette.name.clone())
            .unwrap_or_default()
    });

    // Derived: is dirty?
    let is_dirty = Signal::derive(move || state.get().map(|s| s.is_dirty()).unwrap_or(false));

    // Derived: edit mode
    let edit_mode =
        Signal::derive(move || state.get().map(|s| s.edit_mode).unwrap_or(EditMode::Edit));

    // Derived: shadows factory?
    let shadows_factory = Signal::derive(move || {
        state
            .get()
            .map(|s| s.shadows_factory(&factory_names.get()))
            .unwrap_or(false)
    });

    // Derived: delete button label and enabled state
    let delete_button_label = Signal::derive(move || {
        if edit_mode.get() == EditMode::Edit && shadows_factory.get() {
            "Reset"
        } else {
            "Delete"
        }
    });

    let delete_button_enabled = Signal::derive(move || edit_mode.get() == EditMode::Edit);

    // Derived: checkbox values
    let histogram_enabled = Signal::derive(move || {
        state
            .get()
            .map(|s| s.working_palette.histogram_enabled)
            .unwrap_or(false)
    });

    let smooth_enabled = Signal::derive(move || {
        state
            .get()
            .map(|s| s.working_palette.smooth_enabled)
            .unwrap_or(false)
    });

    let shading_enabled = Signal::derive(move || {
        state
            .get()
            .map(|s| s.working_palette.shading_enabled)
            .unwrap_or(false)
    });

    // Derived: current gradient
    let gradient_signal =
        Signal::derive(move || state.get().map(|s| s.working_palette.gradient.clone()));

    // Callback for gradient changes
    let on_gradient_change = Callback::new(move |new_gradient: Gradient| {
        state.update(|opt| {
            if let Some(s) = opt {
                s.working_palette.gradient = new_gradient;
            }
        });
    });

    // Derived: current transfer curve
    let transfer_curve_signal = Signal::derive(move || {
        state
            .get()
            .map(|s| s.working_palette.transfer_curve.clone())
    });

    // Callback for transfer curve changes
    let on_transfer_curve_change = Callback::new(move |new_curve: Curve| {
        state.update(|opt| {
            if let Some(s) = opt {
                s.working_palette.transfer_curve = new_curve;
            }
        });
    });

    // Derived: falloff curve
    let falloff_curve_signal =
        Signal::derive(move || state.get().map(|s| s.working_palette.falloff_curve.clone()));

    // Callback for falloff curve changes
    let on_falloff_curve_change = Callback::new(move |new_curve: Curve| {
        state.update(|opt| {
            if let Some(s) = opt {
                s.working_palette.falloff_curve = new_curve;
            }
        });
    });

    // Derived: lighting parameters
    let ambient = Signal::derive(move || {
        state
            .get()
            .map(|s| s.working_palette.lighting.ambient)
            .unwrap_or(0.0)
    });
    let diffuse = Signal::derive(move || {
        state
            .get()
            .map(|s| s.working_palette.lighting.diffuse)
            .unwrap_or(0.0)
    });
    let specular = Signal::derive(move || {
        state
            .get()
            .map(|s| s.working_palette.lighting.specular)
            .unwrap_or(0.0)
    });
    let shininess = Signal::derive(move || {
        state
            .get()
            .map(|s| s.working_palette.lighting.shininess)
            .unwrap_or(1.0)
    });
    let strength = Signal::derive(move || {
        state
            .get()
            .map(|s| s.working_palette.lighting.strength)
            .unwrap_or(0.0)
    });
    let azimuth = Signal::derive(move || {
        state
            .get()
            .map(|s| s.working_palette.lighting.azimuth)
            .unwrap_or(0.0)
    });
    let elevation = Signal::derive(move || {
        state
            .get()
            .map(|s| s.working_palette.lighting.elevation)
            .unwrap_or(0.0)
    });

    // Callbacks for lighting parameters
    let on_ambient_change = Callback::new(move |value: f64| {
        state.update(|opt| {
            if let Some(s) = opt {
                s.working_palette.lighting.ambient = value;
            }
        });
    });
    let on_diffuse_change = Callback::new(move |value: f64| {
        state.update(|opt| {
            if let Some(s) = opt {
                s.working_palette.lighting.diffuse = value;
            }
        });
    });
    let on_specular_change = Callback::new(move |value: f64| {
        state.update(|opt| {
            if let Some(s) = opt {
                s.working_palette.lighting.specular = value;
            }
        });
    });
    let on_shininess_change = Callback::new(move |value: f64| {
        state.update(|opt| {
            if let Some(s) = opt {
                s.working_palette.lighting.shininess = value;
            }
        });
    });
    let on_strength_change = Callback::new(move |value: f64| {
        state.update(|opt| {
            if let Some(s) = opt {
                s.working_palette.lighting.strength = value;
            }
        });
    });
    let on_direction_change = Callback::new(move |(az, el): (f64, f64)| {
        state.update(|opt| {
            if let Some(s) = opt {
                s.working_palette.lighting.azimuth = az;
                s.working_palette.lighting.elevation = el;
            }
        });
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
        // Just close the editor - don't modify active_palette.
        // The palette signal already has the correct value (the palette that was
        // active before editing). render_palette will show it once editor closes.
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
            let next = factory
                .first()
                .cloned()
                .unwrap_or_else(|| "Default".to_string());
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

    // Dialog content derived signals
    let dialog_title = Signal::derive(move || match dialog_kind.get() {
        Some(DialogKind::Cancel) => "Unsaved Changes",
        Some(DialogKind::Delete) => "Delete Palette",
        Some(DialogKind::Reset) => "Reset Palette",
        None => "",
    });

    let dialog_message = Signal::derive(move || match dialog_kind.get() {
        Some(DialogKind::Cancel) => {
            "There are unsaved changes that will be lost. Continue?".to_string()
        }
        Some(DialogKind::Delete) => {
            format!(
                "Are you sure you want to delete \"{}\"?",
                palette_name.get()
            )
        }
        Some(DialogKind::Reset) => {
            format!(
                "Are you sure you want to reset \"{}\" to factory defaults?",
                palette_name.get()
            )
        }
        None => String::new(),
    });

    let dialog_confirm_label = Signal::derive(move || match dialog_kind.get() {
        Some(DialogKind::Cancel) => "Continue",
        Some(DialogKind::Delete) => "Delete",
        Some(DialogKind::Reset) => "Reset",
        None => "",
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
                                    class="w-full bg-white/5 border border-white/20 rounded-lg px-3 py-1.5 \
                                           text-white text-sm outline-none focus:border-white/40"
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
                            class="flex-1 flex items-center justify-center gap-1.5 px-3 py-1.5 \
                                   rounded-lg border border-white/20 text-white text-sm \
                                   hover:bg-white/10 transition-colors"
                            on:click=on_cancel_click
                        >
                            <XIcon />
                            "Cancel"
                        </button>
                        <button
                            class=move || format!(
                                "flex-1 flex items-center justify-center gap-1.5 px-3 py-1.5 \
                                 rounded-lg bg-white/20 text-white text-sm transition-colors {}",
                                if is_dirty.get() {
                    "hover:bg-white/30"
                } else {
                    "opacity-30 cursor-not-allowed"
                }
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
                            class="flex-1 flex items-center justify-center gap-1.5 px-3 py-1.5 \
                                   rounded-lg border border-white/10 text-white text-sm \
                                   hover:bg-white/10 transition-colors"
                            on:click=on_duplicate
                        >
                            <CopyIcon />
                            "Duplicate"
                        </button>
                        <button
                            class=move || format!(
                                "flex-1 flex items-center justify-center gap-1.5 px-3 py-1.5 \
                                 rounded-lg border border-white/10 text-white text-sm transition-colors {}",
                                if delete_button_enabled.get() {
                    "hover:bg-white/10"
                } else {
                    "opacity-30 cursor-not-allowed"
                }
                            )
                            prop:disabled=move || !delete_button_enabled.get()
                            on:click=on_delete_click
                        >
                            <TrashIcon />
                            {move || delete_button_label.get()}
                        </button>
                    </div>
                </div>

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

                    // Gradient editor
                    <div class="text-white/50 text-xs px-1">"Color Gradient"</div>
                    <GradientEditor
                        gradient=gradient_signal
                        on_change=on_gradient_change
                    />

                    // Transfer curve editor
                    <div class="text-white/50 text-xs px-1">"Transfer Curve"</div>
                    <CurveEditor
                        curve=transfer_curve_signal
                        on_change=on_transfer_curve_change
                    />
                </CollapsibleSection>

                // Light Effects Section
                <CollapsibleSection title="Light Effects" expanded=light_effects_expanded>
                    // 3D Lighting toggle
                    <div class="space-y-1">
                        <label class="flex items-center gap-2 px-2 py-1 rounded hover:bg-white/5 \
                                      cursor-pointer transition-colors">
                            <input
                                type="checkbox"
                                class="w-3.5 h-3.5 rounded accent-white"
                                prop:checked=move || shading_enabled.get()
                                on:change=move |ev| {
                                    let checked = event_target_checked(&ev);
                                    state.update(|opt| {
                                        if let Some(s) = opt {
                                            s.working_palette.shading_enabled = checked;
                                        }
                                    });
                                }
                            />
                            <span class="text-white text-sm">"3D Lighting"</span>
                        </label>
                    </div>

                    // Conditional content when 3D enabled
                    <Show when=move || shading_enabled.get()>
                        // Falloff Curve
                        <div class="text-white/50 text-xs px-1">"3D Falloff Curve"</div>
                        <CurveEditor
                            curve=falloff_curve_signal
                            on_change=on_falloff_curve_change
                        />

                        // Lighting Parameters
                        <div class="text-white/50 text-xs px-1">"Lighting Parameters"</div>
                        <div class="space-y-2">
                            <LightingSlider
                                label="Ambient"
                                value=ambient
                                on_change=on_ambient_change
                                min=0.0
                                max=1.0
                                step=0.01
                                precision=2
                            />
                            <LightingSlider
                                label="Diffuse"
                                value=diffuse
                                on_change=on_diffuse_change
                                min=0.0
                                max=1.0
                                step=0.01
                                precision=2
                            />
                            <LightingSlider
                                label="Specular"
                                value=specular
                                on_change=on_specular_change
                                min=0.0
                                max=1.0
                                step=0.01
                                precision=2
                            />
                            <LightingSlider
                                label="Shininess"
                                value=shininess
                                on_change=on_shininess_change
                                min=1.0
                                max=128.0
                                step=1.0
                                precision=0
                            />
                            <LightingSlider
                                label="Strength"
                                value=strength
                                on_change=on_strength_change
                                min=0.0
                                max=2.0
                                step=0.01
                                precision=2
                            />
                        </div>

                        // Light Direction
                        <div class="text-white/50 text-xs px-1">"Light Direction"</div>
                        <LightingControl
                            azimuth=azimuth
                            elevation=elevation
                            on_change=on_direction_change
                        />
                    </Show>
                </CollapsibleSection>
            </div>
        </div>

        // Single Confirmation Dialog with dynamic content
        <Show when=move || dialog_kind.get().is_some()>
            {move || {
                let title = dialog_title.get().to_string();
                let message = dialog_message.get();
                let confirm_label = dialog_confirm_label.get().to_string();
                view! {
                    <ConfirmDialog
                        visible=Signal::derive(|| true)
                        title=title
                        message=message
                        cancel_label="Cancel"
                        confirm_label=confirm_label
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
            }}
        </Show>
    }
}

// Simple SVG icons (inline to avoid dependencies)
#[component]
fn XIcon() -> impl IntoView {
    view! {
        <svg class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor"
             stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M18 6 6 18"/>
            <path d="m6 6 12 12"/>
        </svg>
    }
}

#[component]
fn CheckIcon() -> impl IntoView {
    view! {
        <svg class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor"
             stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <polyline points="20 6 9 17 4 12"/>
        </svg>
    }
}

#[component]
fn CopyIcon() -> impl IntoView {
    view! {
        <svg class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor"
             stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <rect width="14" height="14" x="8" y="8" rx="2" ry="2"/>
            <path d="M4 16c-1.1 0-2-.9-2-2V4c0-1.1.9-2 2-2h10c1.1 0 2 .9 2 2"/>
        </svg>
    }
}

#[component]
fn TrashIcon() -> impl IntoView {
    view! {
        <svg class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor"
             stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M3 6h18"/>
            <path d="M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6"/>
            <path d="M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2"/>
        </svg>
    }
}
