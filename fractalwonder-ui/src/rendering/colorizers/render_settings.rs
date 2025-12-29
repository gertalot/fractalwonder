//! Runtime render settings separate from palette.

use serde::{Deserialize, Serialize};

/// Runtime settings that are not persisted with the palette.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RenderSettings {
    pub cycle_count: u32,
    pub use_gpu: bool,
    pub xray_enabled: bool,
    /// Force HDRFloat for all calculations (debug option)
    #[serde(default)]
    pub force_hdr_float: bool,
}

impl Default for RenderSettings {
    fn default() -> Self {
        Self {
            cycle_count: 1,
            use_gpu: true,
            xray_enabled: false,
            force_hdr_float: false,
        }
    }
}

impl RenderSettings {
    pub fn cycle_up(&mut self) {
        self.cycle_count = (self.cycle_count + 1).min(1024);
    }

    pub fn cycle_down(&mut self) {
        self.cycle_count = self.cycle_count.saturating_sub(1).max(1);
    }

    pub fn cycle_up_by(&mut self, amount: u32) {
        self.cycle_count = (self.cycle_count + amount).min(1024);
    }

    pub fn cycle_down_by(&mut self, amount: u32) {
        self.cycle_count = self.cycle_count.saturating_sub(amount).max(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_settings_default_cycle_count_is_one() {
        let settings = RenderSettings::default();
        assert_eq!(settings.cycle_count, 1);
    }

    #[test]
    fn render_settings_cycle_bounds() {
        let mut settings = RenderSettings {
            cycle_count: 1024,
            ..Default::default()
        };
        settings.cycle_up();
        assert_eq!(settings.cycle_count, 1024);

        settings.cycle_count = 1;
        settings.cycle_down();
        assert_eq!(settings.cycle_count, 1);
    }
}
