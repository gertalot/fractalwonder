use fractalwonder_core::MandelbrotData;

/// Layout of SharedArrayBuffer for worker-main communication
///
/// Memory layout:
/// - Bytes 0-3: Tile index counter (AtomicU32)
/// - Bytes 4-7: Render ID (AtomicU32) - for cancellation
/// - Bytes 8+: Tile data (8 bytes per pixel: 4 bytes iterations + 4 bytes escaped flag)
pub struct SharedBufferLayout {
    /// Offset in bytes for tile counter
    tile_index_offset: usize,
    /// Offset in bytes for render ID
    render_id_offset: usize,
    /// Total pixels in canvas
    pub total_pixels: usize,
}

impl SharedBufferLayout {
    const TILE_INDEX_OFFSET: usize = 0;
    const RENDER_ID_OFFSET: usize = 4;
    const DATA_OFFSET: usize = 8;
    const BYTES_PER_PIXEL: usize = 8; // u32 iterations + u32 escaped flag

    pub fn new(canvas_width: u32, canvas_height: u32) -> Self {
        Self {
            tile_index_offset: Self::TILE_INDEX_OFFSET,
            render_id_offset: Self::RENDER_ID_OFFSET,
            total_pixels: (canvas_width * canvas_height) as usize,
        }
    }

    /// Calculate total buffer size needed
    pub fn buffer_size(&self) -> usize {
        Self::DATA_OFFSET + (self.total_pixels * Self::BYTES_PER_PIXEL)
    }

    /// Get offset for pixel data at index
    pub fn pixel_offset(&self, pixel_index: usize) -> usize {
        Self::DATA_OFFSET + (pixel_index * Self::BYTES_PER_PIXEL)
    }

    /// Get tile index counter offset
    pub fn tile_index_offset(&self) -> usize {
        self.tile_index_offset
    }

    /// Get render ID offset
    pub fn render_id_offset(&self) -> usize {
        self.render_id_offset
    }

    /// Encode MandelbrotData to bytes
    pub fn encode_pixel(data: &MandelbrotData) -> [u8; 8] {
        let mut bytes = [0u8; 8];
        bytes[0..4].copy_from_slice(&data.iterations.to_le_bytes());
        bytes[4..8].copy_from_slice(&(data.escaped as u32).to_le_bytes());
        bytes
    }

    /// Decode bytes to MandelbrotData
    pub fn decode_pixel(bytes: &[u8]) -> MandelbrotData {
        let iterations = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        let escaped = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]) != 0;
        MandelbrotData {
            iterations,
            escaped,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_buffer_layout() {
        let layout = SharedBufferLayout::new(800, 600);
        assert_eq!(layout.total_pixels, 480_000);
        assert_eq!(layout.buffer_size(), 8 + (480_000 * 8));
    }

    #[test]
    fn test_pixel_encoding() {
        let data = MandelbrotData {
            iterations: 42,
            escaped: true,
        };

        let bytes = SharedBufferLayout::encode_pixel(&data);
        let decoded = SharedBufferLayout::decode_pixel(&bytes);

        assert_eq!(decoded.iterations, 42);
        assert!(decoded.escaped);
    }

    #[test]
    fn test_pixel_offset() {
        let layout = SharedBufferLayout::new(100, 100);

        // First pixel
        assert_eq!(layout.pixel_offset(0), 8);

        // Second pixel (8 bytes per pixel)
        assert_eq!(layout.pixel_offset(1), 16);

        // 100th pixel
        assert_eq!(layout.pixel_offset(99), 8 + (99 * 8));
    }
}
