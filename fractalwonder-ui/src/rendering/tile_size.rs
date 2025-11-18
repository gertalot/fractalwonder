/// Calculate appropriate tile size based on zoom level
///
/// At extreme zoom levels, we use smaller tiles for more frequent
/// progressive rendering updates during long renders.
pub fn calculate_tile_size(zoom: f64) -> u32 {
    const DEEP_ZOOM_THRESHOLD: f64 = 1e10;
    const NORMAL_TILE_SIZE: u32 = 128;
    const DEEP_ZOOM_TILE_SIZE: u32 = 64;

    if zoom >= DEEP_ZOOM_THRESHOLD {
        DEEP_ZOOM_TILE_SIZE
    } else {
        NORMAL_TILE_SIZE
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normal_zoom_uses_128px_tiles() {
        assert_eq!(calculate_tile_size(1.0), 128);
        assert_eq!(calculate_tile_size(100.0), 128);
        assert_eq!(calculate_tile_size(1e9), 128);
        assert_eq!(calculate_tile_size(9.9e9), 128);
    }

    #[test]
    fn test_deep_zoom_uses_64px_tiles() {
        assert_eq!(calculate_tile_size(1e10), 64);
        assert_eq!(calculate_tile_size(1e11), 64);
        assert_eq!(calculate_tile_size(1e50), 64);
        assert_eq!(calculate_tile_size(1e100), 64);
    }

    #[test]
    fn test_threshold_boundary() {
        // Just below threshold
        assert_eq!(calculate_tile_size(1e10 - 1.0), 128);
        // At threshold
        assert_eq!(calculate_tile_size(1e10), 64);
        // Just above threshold
        assert_eq!(calculate_tile_size(1e10 + 1.0), 64);
    }
}
