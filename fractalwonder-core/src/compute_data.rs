// fractalwonder-core/src/compute_data.rs

/// Data computed for a test image pixel.
/// All fields are bools derived from normalized coordinate comparisons.
#[derive(Clone, Debug, PartialEq)]
pub struct TestImageData {
    pub is_on_origin: bool,
    pub is_on_x_axis: bool,
    pub is_on_y_axis: bool,
    pub is_on_major_tick_x: bool,
    pub is_on_medium_tick_x: bool,
    pub is_on_minor_tick_x: bool,
    pub is_on_major_tick_y: bool,
    pub is_on_medium_tick_y: bool,
    pub is_on_minor_tick_y: bool,
    pub is_light_cell: bool,
}

impl Default for TestImageData {
    fn default() -> Self {
        Self {
            is_on_origin: false,
            is_on_x_axis: false,
            is_on_y_axis: false,
            is_on_major_tick_x: false,
            is_on_medium_tick_x: false,
            is_on_minor_tick_x: false,
            is_on_major_tick_y: false,
            is_on_medium_tick_y: false,
            is_on_minor_tick_y: false,
            is_light_cell: true,
        }
    }
}

/// Unified enum for all compute results.
#[derive(Clone, Debug)]
pub enum ComputeData {
    TestImage(TestImageData),
    // Mandelbrot(MandelbrotData),  // iteration 7
}
