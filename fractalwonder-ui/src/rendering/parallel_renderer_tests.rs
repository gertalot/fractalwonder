//! Tests for parallel_renderer module.

use super::assemble_tiles_to_buffer;
use crate::workers::TileResult;
use fractalwonder_core::{ComputeData, MandelbrotData, PixelRect};

fn make_tile_data(iterations: &[u32]) -> Vec<ComputeData> {
    iterations
        .iter()
        .map(|&i| {
            ComputeData::Mandelbrot(MandelbrotData {
                iterations: i,
                escaped: i > 0,
                ..MandelbrotData::default()
            })
        })
        .collect()
}

fn get_iterations(buffer: &[ComputeData]) -> Vec<u32> {
    buffer
        .iter()
        .map(|d| match d {
            ComputeData::Mandelbrot(m) => m.iterations,
            _ => 0,
        })
        .collect()
}

#[test]
fn assemble_empty_tiles_returns_default_buffer() {
    let buffer = assemble_tiles_to_buffer(&[], 4, 4);

    assert_eq!(buffer.len(), 16);
    for pixel in &buffer {
        match pixel {
            ComputeData::Mandelbrot(m) => {
                assert_eq!(m.iterations, 0);
                assert!(!m.escaped);
            }
            _ => panic!("Expected MandelbrotData"),
        }
    }
}

#[test]
fn assemble_single_tile_covering_full_image() {
    let tile = TileResult {
        tile: PixelRect::new(0, 0, 4, 4),
        data: make_tile_data(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]),
        compute_time_ms: 0.0,
    };

    let buffer = assemble_tiles_to_buffer(&[tile], 4, 4);
    let iterations = get_iterations(&buffer);

    assert_eq!(
        iterations,
        vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]
    );
}

#[test]
fn assemble_tiles_placed_at_correct_positions() {
    // 4x4 image with two 2x2 tiles
    let top_left = TileResult {
        tile: PixelRect::new(0, 0, 2, 2),
        data: make_tile_data(&[1, 2, 3, 4]),
        compute_time_ms: 0.0,
    };
    let bottom_right = TileResult {
        tile: PixelRect::new(2, 2, 2, 2),
        data: make_tile_data(&[5, 6, 7, 8]),
        compute_time_ms: 0.0,
    };

    let buffer = assemble_tiles_to_buffer(&[top_left, bottom_right], 4, 4);
    let iterations = get_iterations(&buffer);

    // Layout: (0,0)=1, (1,0)=2, (0,1)=3, (1,1)=4 for top-left tile
    //         (2,2)=5, (3,2)=6, (2,3)=7, (3,3)=8 for bottom-right tile
    //         Rest should be 0 (default)
    #[rustfmt::skip]
    assert_eq!(iterations, vec![
        1, 2, 0, 0,
        3, 4, 0, 0,
        0, 0, 5, 6,
        0, 0, 7, 8,
    ]);
}

#[test]
fn assemble_tiles_out_of_order() {
    // Tiles can arrive in any order - verify they're placed correctly
    let bottom = TileResult {
        tile: PixelRect::new(0, 2, 4, 2),
        data: make_tile_data(&[5, 6, 7, 8, 9, 10, 11, 12]),
        compute_time_ms: 0.0,
    };
    let top = TileResult {
        tile: PixelRect::new(0, 0, 4, 2),
        data: make_tile_data(&[1, 2, 3, 4, 5, 6, 7, 8]),
        compute_time_ms: 0.0,
    };

    // Pass bottom first, then top
    let buffer = assemble_tiles_to_buffer(&[bottom, top], 4, 4);
    let iterations = get_iterations(&buffer);

    #[rustfmt::skip]
    assert_eq!(iterations, vec![
        1, 2, 3, 4,
        5, 6, 7, 8,
        5, 6, 7, 8,
        9, 10, 11, 12,
    ]);
}

#[test]
fn assemble_tile_extending_past_right_edge() {
    // Tile starts at x=3 with width=2, but image is only 4 wide
    let tile = TileResult {
        tile: PixelRect::new(3, 0, 2, 2),
        data: make_tile_data(&[1, 2, 3, 4]),
        compute_time_ms: 0.0,
    };

    let buffer = assemble_tiles_to_buffer(&[tile], 4, 2);
    let iterations = get_iterations(&buffer);

    // Only x=3 should be filled (x=4 is out of bounds)
    #[rustfmt::skip]
    assert_eq!(iterations, vec![
        0, 0, 0, 1,
        0, 0, 0, 3,
    ]);
}

#[test]
fn assemble_tile_extending_past_bottom_edge() {
    // Tile starts at y=1 with height=2, but image is only 2 high
    let tile = TileResult {
        tile: PixelRect::new(0, 1, 2, 2),
        data: make_tile_data(&[1, 2, 3, 4]),
        compute_time_ms: 0.0,
    };

    let buffer = assemble_tiles_to_buffer(&[tile], 2, 2);
    let iterations = get_iterations(&buffer);

    // Only y=1 should be filled (y=2 is out of bounds)
    assert_eq!(iterations, vec![0, 0, 1, 2]);
}

#[test]
fn assemble_tile_with_insufficient_data() {
    // Tile claims to be 2x2 but only has 2 pixels of data
    let tile = TileResult {
        tile: PixelRect::new(0, 0, 2, 2),
        data: make_tile_data(&[1, 2]), // Only 2 pixels instead of 4
        compute_time_ms: 0.0,
    };

    let buffer = assemble_tiles_to_buffer(&[tile], 2, 2);
    let iterations = get_iterations(&buffer);

    // Only first 2 pixels should be filled
    assert_eq!(iterations, vec![1, 2, 0, 0]);
}
