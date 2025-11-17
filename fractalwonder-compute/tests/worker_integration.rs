use fractalwonder_compute::SharedBufferLayout;
use fractalwonder_core::MandelbrotData;

#[test]
fn test_shared_buffer_roundtrip() {
    let original = MandelbrotData {
        iterations: 123,
        escaped: true,
    };

    let encoded = SharedBufferLayout::encode_pixel(&original);
    let decoded = SharedBufferLayout::decode_pixel(&encoded);

    assert_eq!(original.iterations, decoded.iterations);
    assert_eq!(original.escaped, decoded.escaped);
}

#[test]
fn test_buffer_size_calculation() {
    let layout = SharedBufferLayout::new(1920, 1080);
    let expected_pixels = 1920 * 1080;
    let expected_size = 8 + (expected_pixels * 8); // metadata + pixel data

    assert_eq!(layout.buffer_size(), expected_size);
}
