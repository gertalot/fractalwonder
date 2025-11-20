use crate::rendering::Colorizer;
use fractalwonder_compute::Renderer;
use fractalwonder_core::{Rect, Viewport};
use leptos::RwSignal;
use std::rc::Rc;
use web_sys::HtmlCanvasElement;

/// Canvas renderer trait - takes a Renderer and Colorizer to render RGBA pixels on a canvas
///
/// Implementations handle the strategy for putting computed data onto canvas pixels:
/// - TilingCanvasRenderer: progressive tiled rendering with caching
/// - Future: SimpleCanvasRenderer, OffscreenCanvasRenderer, etc.
pub trait CanvasRenderer {
    type Scalar;
    type Data: Clone;

    /// Swap the renderer at runtime (invalidates cache)
    fn set_renderer(
        &mut self,
        renderer: Box<dyn Renderer<Scalar = Self::Scalar, Data = Self::Data>>,
    );

    /// Swap the colorizer at runtime (preserves cache if implementation supports it)
    fn set_colorizer(&mut self, colorizer: Colorizer);

    /// Main rendering entry point - renders viewport to canvas
    fn render(&self, viewport: &Viewport<Self::Scalar>, canvas: &HtmlCanvasElement);

    /// Get natural bounds from the underlying renderer
    fn natural_bounds(&self) -> Rect<Self::Scalar>;

    /// Cancel any in-progress render
    fn cancel_render(&self);

    /// Get render progress signal
    fn progress(&self) -> RwSignal<crate::rendering::RenderProgress>;
}

// Blanket implementation for Rc<dyn CanvasRenderer> to enable runtime polymorphism
impl<S, D: Clone> CanvasRenderer for Rc<dyn CanvasRenderer<Scalar = S, Data = D>> {
    type Scalar = S;
    type Data = D;

    fn set_renderer(
        &mut self,
        renderer: Box<dyn Renderer<Scalar = Self::Scalar, Data = Self::Data>>,
    ) {
        Rc::get_mut(self)
            .expect("Cannot modify renderer with multiple references")
            .set_renderer(renderer);
    }

    fn set_colorizer(&mut self, colorizer: Colorizer) {
        Rc::get_mut(self)
            .expect("Cannot modify renderer with multiple references")
            .set_colorizer(colorizer);
    }

    fn render(&self, viewport: &Viewport<Self::Scalar>, canvas: &HtmlCanvasElement) {
        (**self).render(viewport, canvas)
    }

    fn natural_bounds(&self) -> Rect<Self::Scalar> {
        (**self).natural_bounds()
    }

    fn cancel_render(&self) {
        (**self).cancel_render()
    }

    fn progress(&self) -> RwSignal<crate::rendering::RenderProgress> {
        (**self).progress()
    }
}
