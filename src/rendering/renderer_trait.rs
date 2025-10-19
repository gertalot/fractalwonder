use crate::rendering::coords::Rect;

pub trait CanvasRenderer {
    type Coord: Clone;

    /// Returns the natural bounds of this renderer (what zoom 1.0 should display)
    fn natural_bounds(&self) -> Rect<Self::Coord>;

    /// Renders the specified image-space rectangle to pixel data
    /// Returns RGBA pixel data (width * height * 4 bytes)
    fn render(&self, target_rect: &Rect<Self::Coord>, width: u32, height: u32) -> Vec<u8>;
}
