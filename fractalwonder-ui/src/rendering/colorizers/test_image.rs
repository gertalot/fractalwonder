use fractalwonder_core::TestImageData;

pub const ORIGIN_COLOR: [u8; 4] = [255, 0, 0, 255];
pub const AXIS_COLOR: [u8; 4] = [100, 100, 100, 255];
pub const MAJOR_TICK_COLOR: [u8; 4] = [50, 50, 50, 255];
pub const MEDIUM_TICK_COLOR: [u8; 4] = [80, 80, 80, 255];
pub const MINOR_TICK_COLOR: [u8; 4] = [120, 120, 120, 255];
pub const BACKGROUND_LIGHT: [u8; 4] = [245, 245, 245, 255];
pub const BACKGROUND_DARK: [u8; 4] = [255, 255, 255, 255];

/// Default colorizer for TestImageData.
pub fn colorize(data: &TestImageData) -> [u8; 4] {
    if data.is_on_origin {
        return ORIGIN_COLOR;
    }
    if data.is_on_major_tick_x || data.is_on_major_tick_y {
        return MAJOR_TICK_COLOR;
    }
    if data.is_on_medium_tick_x || data.is_on_medium_tick_y {
        return MEDIUM_TICK_COLOR;
    }
    if data.is_on_minor_tick_x || data.is_on_minor_tick_y {
        return MINOR_TICK_COLOR;
    }
    if data.is_on_x_axis || data.is_on_y_axis {
        return AXIS_COLOR;
    }
    if data.is_light_cell {
        BACKGROUND_LIGHT
    } else {
        BACKGROUND_DARK
    }
}
