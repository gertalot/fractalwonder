use fractalwonder_core::PixelRect;

/// Calculate tile size based on zoom level.
///
/// Uses smaller tiles at deep zoom for more frequent progress updates.
pub fn calculate_tile_size(zoom_factor: f64) -> u32 {
    const DEEP_ZOOM_THRESHOLD: f64 = 1e10;
    const NORMAL_TILE_SIZE: u32 = 128;
    const DEEP_ZOOM_TILE_SIZE: u32 = 32;

    if zoom_factor >= DEEP_ZOOM_THRESHOLD {
        DEEP_ZOOM_TILE_SIZE
    } else {
        NORMAL_TILE_SIZE
    }
}

/// Generate tiles covering the canvas, sorted by distance from center.
///
/// Center-out ordering provides better UX - users see the most important
/// part of the image first.
pub fn generate_tiles(width: u32, height: u32, tile_size: u32) -> Vec<PixelRect> {
    let mut tiles = Vec::new();

    // Generate grid of tiles
    for y_start in (0..height).step_by(tile_size as usize) {
        for x_start in (0..width).step_by(tile_size as usize) {
            let w = tile_size.min(width - x_start);
            let h = tile_size.min(height - y_start);
            tiles.push(PixelRect::new(x_start, y_start, w, h));
        }
    }

    // Sort by distance from canvas center
    let center_x = width as f64 / 2.0;
    let center_y = height as f64 / 2.0;

    tiles.sort_by(|a, b| {
        let a_center_x = a.x as f64 + a.width as f64 / 2.0;
        let a_center_y = a.y as f64 + a.height as f64 / 2.0;
        let a_dist_sq = (a_center_x - center_x).powi(2) + (a_center_y - center_y).powi(2);

        let b_center_x = b.x as f64 + b.width as f64 / 2.0;
        let b_center_y = b.y as f64 + b.height as f64 / 2.0;
        let b_dist_sq = (b_center_x - center_x).powi(2) + (b_center_y - center_y).powi(2);

        a_dist_sq
            .partial_cmp(&b_dist_sq)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    tiles
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normal_zoom_uses_128px_tiles() {
        assert_eq!(calculate_tile_size(1.0), 128);
        assert_eq!(calculate_tile_size(1e9), 128);
    }

    #[test]
    fn deep_zoom_uses_64px_tiles() {
        assert_eq!(calculate_tile_size(1e10), 64);
        assert_eq!(calculate_tile_size(1e50), 64);
    }

    #[test]
    fn generate_tiles_covers_canvas_exactly() {
        let tiles = generate_tiles(256, 256, 64);

        // Should be 4x4 = 16 tiles
        assert_eq!(tiles.len(), 16);

        // Total area should equal canvas area
        let total_area: u32 = tiles.iter().map(|t| t.area()).sum();
        assert_eq!(total_area, 256 * 256);
    }

    #[test]
    fn generate_tiles_handles_non_divisible_sizes() {
        let tiles = generate_tiles(100, 100, 64);

        // 100/64 = 1.56, so 2x2 = 4 tiles
        assert_eq!(tiles.len(), 4);

        // Edge tiles should be smaller
        let has_partial_width = tiles.iter().any(|t| t.width == 36);
        let has_partial_height = tiles.iter().any(|t| t.height == 36);
        assert!(has_partial_width);
        assert!(has_partial_height);
    }

    #[test]
    fn generate_tiles_center_out_ordering() {
        let tiles = generate_tiles(256, 256, 64);

        // First tile should be one of the center tiles
        let first = &tiles[0];
        let first_center_x = first.x as f64 + first.width as f64 / 2.0;
        let first_center_y = first.y as f64 + first.height as f64 / 2.0;

        // Should be close to canvas center (128, 128)
        let dist_to_center =
            ((first_center_x - 128.0).powi(2) + (first_center_y - 128.0).powi(2)).sqrt();
        assert!(dist_to_center < 64.0, "First tile should be near center");

        // Last tile should be a corner
        let last = &tiles[tiles.len() - 1];
        let last_center_x = last.x as f64 + last.width as f64 / 2.0;
        let last_center_y = last.y as f64 + last.height as f64 / 2.0;
        let last_dist = ((last_center_x - 128.0).powi(2) + (last_center_y - 128.0).powi(2)).sqrt();
        assert!(
            last_dist > dist_to_center,
            "Last tile should be farther from center"
        );
    }

    #[test]
    fn generate_tiles_no_overlap() {
        let tiles = generate_tiles(256, 256, 64);

        for (i, a) in tiles.iter().enumerate() {
            for (j, b) in tiles.iter().enumerate() {
                if i == j {
                    continue;
                }
                // Check no overlap: rectangles overlap if they intersect in both x and y
                let x_overlap = a.x < b.x + b.width && a.x + a.width > b.x;
                let y_overlap = a.y < b.y + b.height && a.y + a.height > b.y;
                assert!(!(x_overlap && y_overlap), "Tiles {} and {} overlap", i, j);
            }
        }
    }
}
