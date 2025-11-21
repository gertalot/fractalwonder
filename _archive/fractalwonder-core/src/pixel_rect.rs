use serde::{Deserialize, Serialize};

/// Rectangle in pixel space, used for rendering sub-regions of canvas
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PixelRect {
    /// X coordinate of top-left corner
    pub x: u32,
    /// Y coordinate of top-left corner
    pub y: u32,
    /// Width in pixels
    pub width: u32,
    /// Height in pixels
    pub height: u32,
}

impl PixelRect {
    pub fn new(x: u32, y: u32, width: u32, height: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Create a PixelRect covering full canvas
    pub fn full_canvas(width: u32, height: u32) -> Self {
        Self {
            x: 0,
            y: 0,
            width,
            height,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let rect = PixelRect::new(10, 20, 100, 200);
        assert_eq!(rect.x, 10);
        assert_eq!(rect.y, 20);
        assert_eq!(rect.width, 100);
        assert_eq!(rect.height, 200);
    }

    #[test]
    fn test_full_canvas() {
        let rect = PixelRect::full_canvas(800, 600);
        assert_eq!(rect.x, 0);
        assert_eq!(rect.y, 0);
        assert_eq!(rect.width, 800);
        assert_eq!(rect.height, 600);
    }
}
