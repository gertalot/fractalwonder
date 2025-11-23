use serde::{Deserialize, Serialize};

/// Rectangle in pixel space (always u32 coordinates)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PixelRect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl PixelRect {
    /// Create new pixel rectangle
    pub fn new(x: u32, y: u32, width: u32, height: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Calculate area in pixels
    pub fn area(&self) -> u32 {
        self.width * self.height
    }

    /// Check if point is inside rectangle
    pub fn contains(&self, px: u32, py: u32) -> bool {
        px >= self.x && px < self.x + self.width && py >= self.y && py < self.y + self.height
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pixel_rect_creation() {
        let rect = PixelRect {
            x: 10,
            y: 20,
            width: 100,
            height: 200,
        };

        assert_eq!(rect.x, 10);
        assert_eq!(rect.y, 20);
        assert_eq!(rect.width, 100);
        assert_eq!(rect.height, 200);
    }

    #[test]
    fn test_pixel_rect_area() {
        let rect = PixelRect {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
        };

        assert_eq!(rect.area(), 1920 * 1080);
    }

    #[test]
    fn test_pixel_rect_contains_point() {
        let rect = PixelRect {
            x: 10,
            y: 20,
            width: 100,
            height: 50,
        };

        assert!(rect.contains(50, 40));
        assert!(rect.contains(10, 20)); // Top-left corner
        assert!(rect.contains(109, 69)); // Bottom-right corner
        assert!(!rect.contains(110, 70)); // Just outside
        assert!(!rect.contains(9, 20)); // Just left
        assert!(!rect.contains(50, 19)); // Just above
    }

    #[test]
    fn test_pixel_rect_serialization_roundtrip() {
        let original = PixelRect {
            x: 100,
            y: 200,
            width: 640,
            height: 480,
        };

        let json = serde_json::to_string(&original).unwrap();
        let restored: PixelRect = serde_json::from_str(&json).unwrap();

        assert_eq!(restored, original);
    }

    #[test]
    fn test_pixel_rect_new_helper() {
        let rect = PixelRect::new(5, 10, 200, 150);

        assert_eq!(rect.x, 5);
        assert_eq!(rect.y, 10);
        assert_eq!(rect.width, 200);
        assert_eq!(rect.height, 150);
    }
}
