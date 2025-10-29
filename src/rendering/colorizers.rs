use super::app_data::{AppData, TestImageData};

/// Colorizer function type - converts Data to RGBA
pub type Colorizer<D> = fn(&D) -> (u8, u8, u8, u8);

/// Colorize TestImageData
pub fn test_image_colorizer(data: &AppData) -> (u8, u8, u8, u8) {
    match data {
        AppData::TestImageData(d) => test_image_data_to_rgba(d),
        #[allow(unreachable_patterns)]
        _ => (0, 0, 0, 255), // Black for wrong type
    }
}

fn test_image_data_to_rgba(data: &TestImageData) -> (u8, u8, u8, u8) {
    // Circle distance < 0.1 means on a circle -> red
    if data.circle_distance < 0.1 {
        return (255, 0, 0, 255); // Red circle
    }

    // Checkerboard pattern
    if data.checkerboard {
        (255, 255, 255, 255) // White
    } else {
        (204, 204, 204, 255) // Light grey
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_colorizer_on_circle() {
        let data = AppData::TestImageData(TestImageData::new(true, 0.05));
        let color = test_image_colorizer(&data);
        assert_eq!(color, (255, 0, 0, 255)); // Red
    }

    #[test]
    fn test_colorizer_checkerboard_white() {
        let data = AppData::TestImageData(TestImageData::new(true, 5.0));
        let color = test_image_colorizer(&data);
        assert_eq!(color, (255, 255, 255, 255)); // White
    }

    #[test]
    fn test_colorizer_checkerboard_grey() {
        let data = AppData::TestImageData(TestImageData::new(false, 5.0));
        let color = test_image_colorizer(&data);
        assert_eq!(color, (204, 204, 204, 255)); // Grey
    }
}
