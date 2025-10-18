// ABOUTME: Core rendering abstractions for pluggable renderer architecture
// ABOUTME: Exports coordinate types, viewport, transforms, and renderer trait

pub mod coords;
pub mod transforms;
pub mod viewport;

pub use coords::{ImageCoord, ImageRect, PixelCoord};
pub use transforms::{calculate_visible_bounds, image_to_pixel, pixel_to_image};
pub use viewport::Viewport;
