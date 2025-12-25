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
