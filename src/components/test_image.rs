use crate::hooks::use_canvas_interaction::{use_canvas_interaction, TransformResult};
use crate::rendering::{
    apply_pixel_transform_to_viewport,
    coords::{Coord, Rect},
    render_with_viewport,
    renderer_trait::CanvasRenderer,
    transforms::pixel_to_image,
    viewport::Viewport,
};
use leptos::*;

#[derive(Clone)]
pub struct TestImageRenderer {
    checkerboard_size: f64,
    circle_radius_step: f64,
    circle_line_thickness: f64,
}

impl TestImageRenderer {
    pub fn new() -> Self {
        Self {
            checkerboard_size: 10.0,
            circle_radius_step: 10.0,
            circle_line_thickness: 0.5,
        }
    }

    fn compute_pixel_color(&self, x: f64, y: f64) -> (u8, u8, u8, u8) {
        // Draw bright green vertical line through the center (x=0)
        if x.abs() < 0.5 {
            return (0, 255, 0, 255); // Bright green
        }

        // Check if on circle first (circles drawn on top)
        let distance = (x * x + y * y).sqrt();
        let nearest_ring = (distance / self.circle_radius_step).round();
        let ring_distance = (distance - nearest_ring * self.circle_radius_step).abs();

        if ring_distance < self.circle_line_thickness / 2.0 && nearest_ring > 0.0 {
            return (255, 0, 0, 255); // Red circle
        }

        // Checkerboard: (0,0) is corner of four squares
        let square_x = (x / self.checkerboard_size).floor() as i32;
        let square_y = (y / self.checkerboard_size).floor() as i32;
        let is_light = (square_x + square_y) % 2 == 0;

        if is_light {
            (255, 255, 255, 255) // White
        } else {
            (204, 204, 204, 255) // Light grey
        }
    }
}

impl CanvasRenderer for TestImageRenderer {
    type Coord = f64;

    fn natural_bounds(&self) -> Rect<f64> {
        Rect::new(Coord::new(-50.0, -50.0), Coord::new(50.0, 50.0))
    }

    fn render(&self, target_rect: &Rect<f64>, width: u32, height: u32) -> Vec<u8> {
        let mut pixels = vec![0u8; (width * height * 4) as usize];

        for py in 0..height {
            for px in 0..width {
                // Map pixel to image coordinates using centralized transform
                let image_coord = pixel_to_image(px as f64, py as f64, target_rect, width, height);

                let color = self.compute_pixel_color(*image_coord.x(), *image_coord.y());

                let idx = ((py * width + px) * 4) as usize;
                pixels[idx] = color.0; // R
                pixels[idx + 1] = color.1; // G
                pixels[idx + 2] = color.2; // B
                pixels[idx + 3] = color.3; // A
            }
        }

        pixels
    }
}

#[component]
pub fn TestImageView() -> impl IntoView {
    let canvas_ref = create_node_ref::<leptos::html::Canvas>();

    // Initialize renderer and viewport state
    let renderer = TestImageRenderer::new();
    let natural_bounds = renderer.natural_bounds();

    // Viewport state: center position and zoom level
    let viewport = create_rw_signal(Viewport::new(Coord::new(0.0, 0.0), 1.0, natural_bounds));

    // Set up interaction hook with viewport update and re-render
    let handle = use_canvas_interaction(canvas_ref, move |result: TransformResult| {
        // Get current canvas dimensions
        if let Some(canvas) = canvas_ref.get_untracked() {
            let width = canvas.width();
            let height = canvas.height();

            // Apply transformation to viewport
            let new_viewport = apply_pixel_transform_to_viewport(
                &viewport.get_untracked(),
                &result,
                width,
                height,
            );

            // Update viewport state
            viewport.set(new_viewport);

            // Trigger full re-render (the effect below will handle it)
        }
    });

    // Initialize canvas on mount
    let renderer_for_init = renderer.clone();
    create_effect(move |_| {
        if let Some(canvas) = canvas_ref.get() {
            let window = web_sys::window().expect("should have window");
            canvas.set_width(window.inner_width().unwrap().as_f64().unwrap() as u32);
            canvas.set_height(window.inner_height().unwrap().as_f64().unwrap() as u32);

            // Initial render - draw test pattern with current viewport
            render_with_viewport(&canvas, &renderer_for_init, &viewport.get());
        }
    });

    // Re-render whenever viewport changes
    let renderer_for_updates = renderer.clone();
    create_effect(move |_| {
        let current_viewport = viewport.get();
        if let Some(canvas) = canvas_ref.get_untracked() {
            render_with_viewport(&canvas, &renderer_for_updates, &current_viewport);
        }
    });

    // Handle window resize
    create_effect({
        let on_canvas_resize = handle.on_canvas_resize.clone();
        let viewport_clone = viewport;
        move |_| {
            if let Some(canvas) = canvas_ref.get() {
                use wasm_bindgen::closure::Closure;
                use wasm_bindgen::JsCast;

                let canvas_clone = canvas.clone();
                let on_canvas_resize = on_canvas_resize.clone();
                let viewport_for_resize = viewport_clone;

                let resize_handler = Closure::wrap(Box::new(move || {
                    let window = web_sys::window().expect("should have window");
                    let new_width = window.inner_width().unwrap().as_f64().unwrap() as u32;
                    let new_height = window.inner_height().unwrap().as_f64().unwrap() as u32;

                    // Get old dimensions before setting new ones
                    let old_width = canvas_clone.width();
                    let old_height = canvas_clone.height();

                    // Only process if size actually changed
                    if old_width != new_width || old_height != new_height {
                        // Notify the interaction hook BEFORE changing canvas size
                        // This captures current ImageData and marks us as "interacting"
                        (on_canvas_resize)(new_width, new_height);

                        // Update canvas dimensions
                        // Note: This clears the canvas! But we're now "interacting", so RAF loop will re-draw
                        canvas_clone.set_width(new_width);
                        canvas_clone.set_height(new_height);

                        // The RAF loop will automatically re-draw the captured ImageData preview
                        // After interaction ends, the viewport effect will trigger a full re-render
                        // Force a viewport update to trigger re-render after resize completes
                        viewport_for_resize.update(|v| {
                            // No change to viewport, just trigger reactivity
                            *v = v.clone();
                        });
                    }
                }) as Box<dyn Fn() + 'static>);

                web_sys::window()
                    .expect("should have window")
                    .add_event_listener_with_callback(
                        "resize",
                        resize_handler.as_ref().unchecked_ref(),
                    )
                    .expect("should add resize listener");

                resize_handler.forget();
            }
        }
    });

    // Manually attach wheel event listener with passive: false
    // This is required to allow preventDefault() on wheel events
    create_effect({
        move |_| {
            if let Some(canvas) = canvas_ref.get() {
                use wasm_bindgen::JsCast;
                use web_sys::AddEventListenerOptions;

                let options = AddEventListenerOptions::new();
                options.set_passive(false);

                let on_wheel = handle.on_wheel.clone();
                let closure =
                    wasm_bindgen::closure::Closure::wrap(Box::new(move |ev: web_sys::WheelEvent| {
                        (on_wheel)(ev);
                    })
                        as Box<dyn Fn(web_sys::WheelEvent) + 'static>);

                canvas
                    .add_event_listener_with_callback_and_add_event_listener_options(
                        "wheel",
                        closure.as_ref().unchecked_ref(),
                        &options,
                    )
                    .expect("should add wheel listener");

                closure.forget();
            }
        }
    });

    view! {
        <div class="relative w-full h-full">
            <canvas
                node_ref=canvas_ref
                class="block w-full h-full"
                on:pointerdown=move |ev| (handle.on_pointer_down)(ev)
                on:pointermove=move |ev| (handle.on_pointer_move)(ev)
                on:pointerup=move |ev| (handle.on_pointer_up)(ev)
                style="touch-action: none; cursor: grab;"
            />
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper to create TransformResult with center-relative offsets
    fn make_transform_result(
        offset_x: f64,
        offset_y: f64,
        zoom_factor: f64,
        canvas_width: u32,
        canvas_height: u32,
    ) -> TransformResult {
        // Convert center-relative to absolute for matrix
        let canvas_center_x = canvas_width as f64 / 2.0;
        let canvas_center_y = canvas_height as f64 / 2.0;
        let absolute_offset_x = offset_x + canvas_center_x * (1.0 - zoom_factor);
        let absolute_offset_y = offset_y + canvas_center_y * (1.0 - zoom_factor);

        TransformResult {
            offset_x,
            offset_y,
            zoom_factor,
            matrix: [
                [zoom_factor, 0.0, absolute_offset_x],
                [0.0, zoom_factor, absolute_offset_y],
                [0.0, 0.0, 1.0],
            ],
        }
    }

    #[test]
    fn test_renderer_natural_bounds() {
        let renderer = TestImageRenderer::new();
        let bounds = renderer.natural_bounds();
        assert_eq!(*bounds.min.x(), -50.0);
        assert_eq!(*bounds.max.x(), 50.0);
    }

    #[test]
    fn test_pure_pan_right_and_down() {
        use crate::rendering::viewport::Viewport;

        // Start with viewport at origin, zoom 1.0
        let viewport = Viewport::new(
            Coord::new(0.0, 0.0),
            1.0,
            Rect::new(Coord::new(-50.0, -50.0), Coord::new(50.0, 50.0)),
        );

        // Canvas is 800x600, landscape aspect ratio
        let canvas_width = 800;
        let canvas_height = 600;

        // User panned right 100 pixels and down 50 pixels
        // With zoom=1.0, offset is pure pan
        let result = TransformResult {
            offset_x: 100.0,
            offset_y: 50.0,
            zoom_factor: 1.0,
            matrix: [[1.0, 0.0, 100.0], [0.0, 1.0, 50.0], [0.0, 0.0, 1.0]],
        };

        let new_viewport =
            apply_pixel_transform_to_viewport(&viewport, &result, canvas_width, canvas_height);

        // When user pans right, they're dragging the image to the right
        // This means we're looking at content that was to the left
        // So the viewport center should move LEFT (negative x)
        // With zoom=1.0, we use canvas center as zoom point, so the offset
        // moves us in the opposite direction
        assert!(
            *new_viewport.center.x() < 0.0,
            "center should move left when dragging right"
        );
        assert!(
            *new_viewport.center.y() < 0.0,
            "center should move up when dragging down"
        );
        assert_eq!(new_viewport.zoom, 1.0, "zoom should be unchanged");
    }

    #[test]
    fn test_zoom_centered_at_canvas_center() {
        use crate::rendering::viewport::Viewport;

        // Start at origin, zoom 1.0
        let viewport = Viewport::new(
            Coord::new(0.0, 0.0),
            1.0,
            Rect::new(Coord::new(-50.0, -50.0), Coord::new(50.0, 50.0)),
        );

        let canvas_width = 800;
        let canvas_height = 600;

        // User zooms in 2x at the center of the canvas (400, 300)
        // With center-relative offsets, zooming at canvas center produces offset (0, 0)
        let result = make_transform_result(0.0, 0.0, 2.0, canvas_width, canvas_height);

        let new_viewport =
            apply_pixel_transform_to_viewport(&viewport, &result, canvas_width, canvas_height);

        // When zooming at canvas center, the viewport center should stay the same
        // because we're zooming into the center of what we're looking at
        assert!(
            (*new_viewport.center.x() - 0.0).abs() < 0.01,
            "center.x should stay near 0.0, got {}",
            new_viewport.center.x()
        );
        assert!(
            (*new_viewport.center.y() - 0.0).abs() < 0.01,
            "center.y should stay near 0.0, got {}",
            new_viewport.center.y()
        );
        assert_eq!(new_viewport.zoom, 2.0);
    }

    #[test]
    fn test_zoom_centered_at_top_left() {
        use crate::rendering::viewport::Viewport;

        // Start at origin, zoom 1.0
        let viewport = Viewport::new(
            Coord::new(0.0, 0.0),
            1.0,
            Rect::new(Coord::new(-50.0, -50.0), Coord::new(50.0, 50.0)),
        );

        let canvas_width = 800;
        let canvas_height = 600;

        // User zooms in 2x at top-left corner (0, 0)
        // Mouse at (0, 0), canvas center at (400, 300)
        // Absolute offset = mouse * (1 - zoom) = 0 * (-1) = 0
        // Center-relative = absolute - canvas_center * (1 - zoom) = 0 - (400, 300) * (-1) = (400, 300)
        let result = make_transform_result(400.0, 300.0, 2.0, canvas_width, canvas_height);

        let new_viewport =
            apply_pixel_transform_to_viewport(&viewport, &result, canvas_width, canvas_height);

        // When zooming at top-left, we should be looking at the top-left part of the original view
        // The center should move towards the top-left
        assert!(
            *new_viewport.center.x() < 0.0,
            "center should move left, got x={}",
            new_viewport.center.x()
        );
        assert!(
            *new_viewport.center.y() < 0.0,
            "center should move up, got y={}",
            new_viewport.center.y()
        );
        assert_eq!(new_viewport.zoom, 2.0);
    }

    #[test]
    fn test_zoom_out_at_center() {
        use crate::rendering::viewport::Viewport;

        // Start zoomed in
        let viewport = Viewport::new(
            Coord::new(0.0, 0.0),
            2.0,
            Rect::new(Coord::new(-50.0, -50.0), Coord::new(50.0, 50.0)),
        );

        let canvas_width = 800;
        let canvas_height = 600;

        // User zooms out 0.5x at canvas center
        // Center-relative offset is (0, 0) for zooming at center
        let result = make_transform_result(0.0, 0.0, 0.5, canvas_width, canvas_height);

        let new_viewport =
            apply_pixel_transform_to_viewport(&viewport, &result, canvas_width, canvas_height);

        // Zooming out at center should keep center unchanged
        assert!(
            (*new_viewport.center.x() - 0.0).abs() < 0.01,
            "center.x should stay near 0.0, got {}",
            new_viewport.center.x()
        );
        assert!(
            (*new_viewport.center.y() - 0.0).abs() < 0.01,
            "center.y should stay near 0.0, got {}",
            new_viewport.center.y()
        );
        assert_eq!(new_viewport.zoom, 1.0); // 2.0 * 0.5 = 1.0
    }

    #[test]
    fn test_zoom_at_arbitrary_point() {
        use crate::rendering::viewport::Viewport;

        let viewport = Viewport::new(
            Coord::new(0.0, 0.0),
            1.0,
            Rect::new(Coord::new(-50.0, -50.0), Coord::new(50.0, 50.0)),
        );

        let canvas_width = 800;
        let canvas_height = 600;

        // Zoom 3x at an arbitrary point (200, 150)
        // Mouse at (200, 150), canvas center at (400, 300)
        // Absolute offset = mouse * (1 - zoom) = (200, 150) * (1 - 3) = (200, 150) * (-2) = (-400, -300)
        // Center-relative = absolute - canvas_center * (1 - zoom)
        //                 = (-400, -300) - (400, 300) * (-2)
        //                 = (-400, -300) - (-800, -600)
        //                 = (400, 300)
        let result = make_transform_result(400.0, 300.0, 3.0, canvas_width, canvas_height);

        let new_viewport =
            apply_pixel_transform_to_viewport(&viewport, &result, canvas_width, canvas_height);

        // Verify zoom level
        assert_eq!(new_viewport.zoom, 3.0);

        // The center should have moved to keep the zoom point fixed
        // We can't easily compute the exact expected center without duplicating the logic,
        // but we can verify it changed
        assert_ne!(*new_viewport.center.x(), 0.0, "center should have moved");
        assert_ne!(*new_viewport.center.y(), 0.0, "center should have moved");
    }

    #[test]
    fn test_pan_from_offset_viewport() {
        use crate::rendering::viewport::Viewport;

        // Start with viewport already offset
        let viewport = Viewport::new(
            Coord::new(20.0, -10.0),
            1.0,
            Rect::new(Coord::new(-50.0, -50.0), Coord::new(50.0, 50.0)),
        );

        let canvas_width = 800;
        let canvas_height = 600;

        // Pan left 50 pixels (negative offset means panning left)
        let result = TransformResult {
            offset_x: -50.0,
            offset_y: 0.0,
            zoom_factor: 1.0,
            matrix: [[1.0, 0.0, -50.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
        };

        let new_viewport =
            apply_pixel_transform_to_viewport(&viewport, &result, canvas_width, canvas_height);

        // Panning left (negative offset) means looking right, so center moves right (positive)
        assert!(*new_viewport.center.x() > 20.0, "center should move right");
        assert_eq!(*new_viewport.center.y(), -10.0, "y should be unchanged");
        assert_eq!(new_viewport.zoom, 1.0);
    }

    #[test]
    fn test_zoom_from_zoomed_viewport() {
        use crate::rendering::viewport::Viewport;

        // Start already zoomed in at offset position
        let viewport = Viewport::new(
            Coord::new(10.0, 5.0),
            4.0,
            Rect::new(Coord::new(-50.0, -50.0), Coord::new(50.0, 50.0)),
        );

        let canvas_width = 800;
        let canvas_height = 600;

        // Zoom in 2x more at center
        // Zooming at canvas center means center-relative offset is (0, 0)
        let result = make_transform_result(0.0, 0.0, 2.0, canvas_width, canvas_height);

        let new_viewport =
            apply_pixel_transform_to_viewport(&viewport, &result, canvas_width, canvas_height);

        // Zoom should multiply
        assert_eq!(new_viewport.zoom, 8.0); // 4.0 * 2.0
                                            // Center should stay roughly the same when zooming at canvas center
        assert!(
            (*new_viewport.center.x() - 10.0).abs() < 1.0,
            "center.x should stay near 10.0, got {}",
            new_viewport.center.x()
        );
    }

    #[test]
    fn test_renderer_produces_correct_pixel_count() {
        let renderer = TestImageRenderer::new();
        let bounds = Rect::new(Coord::new(-10.0, -10.0), Coord::new(10.0, 10.0));
        let pixels = renderer.render(&bounds, 100, 100);
        assert_eq!(pixels.len(), 100 * 100 * 4);
    }

    #[test]
    fn test_checkerboard_pattern_at_origin() {
        let renderer = TestImageRenderer::new();

        // Point at (-5, -5) should be in one square
        let color1 = renderer.compute_pixel_color(-5.0, -5.0);
        // Point at (5, 5) should be in same color (both negative square indices sum to even)
        let color2 = renderer.compute_pixel_color(5.0, 5.0);
        // Point at (5, -5) should be opposite color
        let color3 = renderer.compute_pixel_color(5.0, -5.0);

        assert_eq!(color1, color2);
        assert_ne!(color1, color3);
    }

    #[test]
    fn test_circle_at_radius_10() {
        let renderer = TestImageRenderer::new();

        // Point exactly on circle (radius 10)
        let color_on = renderer.compute_pixel_color(10.0, 0.0);
        assert_eq!(color_on, (255, 0, 0, 255)); // Red

        // Point between circles
        let color_off = renderer.compute_pixel_color(15.0, 0.0);
        assert_ne!(color_off, (255, 0, 0, 255)); // Not red
    }

    #[test]
    fn test_origin_is_corner_of_four_squares() {
        let renderer = TestImageRenderer::new();

        // (0,0) is corner, so nearby points in different quadrants have different colors
        let q1 = renderer.compute_pixel_color(1.0, 1.0);
        let q2 = renderer.compute_pixel_color(-1.0, 1.0);
        let q3 = renderer.compute_pixel_color(-1.0, -1.0);
        let q4 = renderer.compute_pixel_color(1.0, -1.0);

        // Opposite quadrants should have same color
        assert_eq!(q1, q3);
        assert_eq!(q2, q4);
        assert_ne!(q1, q2);
    }
}
