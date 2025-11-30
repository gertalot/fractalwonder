# Color Options UI Design

## Overview

Replace the current preset-based color scheme selector with composable controls:
- **Palette menu**: Select from 11 color palettes
- **Options menu**: Toggle 3D shading, smooth iteration, adjust cycle count
- **Toast notifications**: Brief feedback when settings change via keyboard
- **Keyboard shortcuts**: Full control without mouse

## Data Model

### ColorOptions struct

```rust
/// User-configurable color options (persisted to localStorage)
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ColorOptions {
    pub palette_id: String,      // "classic", "fire", "ocean", etc.
    pub shading_enabled: bool,   // 3D slope shading
    pub smooth_enabled: bool,    // Smooth iteration coloring
    pub cycle_count: u32,        // Palette repetitions: 1, 2, 4, 8, 16, 32, 64, 128
}

impl Default for ColorOptions {
    fn default() -> Self {
        Self {
            palette_id: "classic".to_string(),
            shading_enabled: false,
            smooth_enabled: true,
            cycle_count: 32,
        }
    }
}
```

### Palette Registry

Replace `presets()` with a simpler palette list:

```rust
pub struct PaletteEntry {
    pub id: &'static str,
    pub name: &'static str,
    pub palette: Palette,
}

pub fn palettes() -> Vec<PaletteEntry> {
    vec![
        PaletteEntry { id: "classic", name: "Classic", palette: Palette::ultra_fractal() },
        PaletteEntry { id: "fire", name: "Fire", palette: Palette::fire() },
        PaletteEntry { id: "ocean", name: "Ocean", palette: Palette::ocean() },
        PaletteEntry { id: "electric", name: "Electric", palette: Palette::electric() },
        PaletteEntry { id: "grayscale", name: "Grayscale", palette: Palette::grayscale() },
        PaletteEntry { id: "rainbow", name: "Rainbow", palette: Palette::rainbow() },
        PaletteEntry { id: "neon", name: "Neon", palette: Palette::neon() },
        PaletteEntry { id: "twilight", name: "Twilight", palette: Palette::twilight() },
        PaletteEntry { id: "candy", name: "Candy", palette: Palette::candy() },
        PaletteEntry { id: "inferno", name: "Inferno", palette: Palette::inferno() },
        PaletteEntry { id: "aurora", name: "Aurora", palette: Palette::aurora() },
    ]
}
```

### Migration from old format

On load, if `color_scheme_id` exists instead of `ColorOptions`:
- Parse preset name to extract palette (e.g., "Fire 3D" → palette: "fire", shading: true)
- Use defaults for smooth (true) and cycles (32)

## UI Components

### Palette Menu

Dropdown replacing current "Color" menu:
- Label: "Palette"
- Options: 11 palette names
- Selection updates `color_options.palette_id`

### Options Menu

Dropdown with grouped sections:

```
┌─────────────────────────┐
│ Effects                 │
│   ☐ 3D             [3]  │
│   ☑ Smooth         [S]  │
├─────────────────────────┤
│ Cycles                  │
│      32           [↑↓]  │
└─────────────────────────┘
```

- **Effects section**: Checkboxes for 3D and Smooth toggles
- **Cycles section**: Current value display with keyboard hint
- Keyboard hints shown in muted text

### Toast Component

New component for transient feedback:

```rust
#[component]
pub fn Toast(
    message: Signal<Option<String>>,
    ui_visible: Signal<bool>,
) -> impl IntoView
```

Behavior:
- Position: Fixed, bottom center, 48px from bottom
- Style: Semi-transparent dark background (#000000cc), white text, rounded corners
- Animation: Fade in 150ms, hold 1.5s, fade out 300ms
- Visibility: Only shown when `ui_visible` is false
- Content examples: "3D: On", "Smooth: Off", "Cycles: 64", "Palette: Fire"

## Keyboard Shortcuts

| Key | Action | Toast message |
|-----|--------|---------------|
| `←` | Previous palette (wraps) | "Palette: {name}" |
| `→` | Next palette (wraps) | "Palette: {name}" |
| `↑` | Double cycle count (max 128) | "Cycles: {n}" |
| `↓` | Halve cycle count (min 1) | "Cycles: {n}" |
| `3` | Toggle 3D shading | "3D: On" / "3D: Off" |
| `s` | Toggle smooth iteration | "Smooth: On" / "Smooth: Off" |

All shortcuts work globally (not just when menu open).

## Renderer Integration

### Building ColorSettings from ColorOptions

```rust
impl ColorOptions {
    pub fn to_color_settings(&self) -> ColorSettings {
        let palette = palettes()
            .into_iter()
            .find(|p| p.id == self.palette_id)
            .map(|p| p.palette)
            .unwrap_or_else(Palette::ultra_fractal);

        ColorSettings {
            palette,
            cycle_count: self.cycle_count as f64,
            shading: if self.shading_enabled {
                ShadingSettings::enabled()
            } else {
                ShadingSettings::disabled()
            },
        }
    }
}
```

### Smooth iteration toggle

When `smooth_enabled` is false:
- Use discrete integer iteration count for coloring
- Results in sharp color bands (classic 8-bit fractal aesthetic)
- Implementation: Skip smooth iteration calculation, use raw `iterations` value

### Re-colorization

Color option changes trigger re-colorization only (not re-computation):
- Iteration data cached in tiles
- Only the colorization pass runs with new settings
- Fast feedback for color changes

## Persistence

### PersistedState changes

```rust
pub struct PersistedState {
    pub viewport: Viewport,
    pub config_id: String,
    pub color_options: ColorOptions,  // replaces color_scheme_id
}
```

### localStorage format

```json
{
  "viewport": { ... },
  "config_id": "mandelbrot",
  "color_options": {
    "palette_id": "fire",
    "shading_enabled": true,
    "smooth_enabled": true,
    "cycle_count": 64
  }
}
```

## Files to modify

1. `rendering/colorizers/mod.rs` - Add `PaletteEntry`, `palettes()` function
2. `rendering/colorizers/settings.rs` - Add `ColorOptions` struct
3. `hooks/persistence.rs` - Update `PersistedState`, add migration
4. `app.rs` - Update keyboard handler, add toast signal, pass color options
5. `components/mod.rs` - Export new components
6. `components/toast.rs` - New toast component
7. `components/ui_panel.rs` - Replace color dropdown with palette + options menus
8. `components/dropdown_menu.rs` - May need grouped section support
9. `rendering/colorizers/presets.rs` - Deprecate or remove

## Implementation order

1. Add `ColorOptions` struct and `palettes()` registry
2. Update persistence layer with migration
3. Create Toast component
4. Update UIPanel with new menus
5. Update keyboard handler in app.rs
6. Wire color options to renderer
7. Implement smooth toggle in colorizer
8. Remove old preset system
