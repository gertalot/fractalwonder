// ABOUTME: Core rendering abstractions for pluggable renderer architecture
// ABOUTME: Exports coordinate types, viewport, transforms, and renderer trait

pub mod coords;
pub mod viewport;

pub use coords::{ImageCoord, ImageRect, PixelCoord};
pub use viewport::Viewport;
