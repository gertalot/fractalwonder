use leptos::*;

/// Calculate gradient color for a pixel position.
/// R increases left-to-right, G increases top-to-bottom, B constant at 128.
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
fn gradient_color(x: u32, y: u32, width: u32, height: u32) -> [u8; 4] {
    let r = ((x as f64 / width as f64) * 255.0) as u8;
    let g = ((y as f64 / height as f64) * 255.0) as u8;
    let b = 128u8;
    let a = 255u8;
    [r, g, b, a]
}

#[component]
pub fn InteractiveCanvas() -> impl IntoView {
    view! {
        <canvas class="block" />
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gradient_top_left_is_green_blue() {
        let [r, g, b, a] = gradient_color(0, 0, 100, 100);
        assert_eq!(r, 0, "top-left red should be 0");
        assert_eq!(g, 0, "top-left green should be 0");
        assert_eq!(b, 128, "blue should be constant 128");
        assert_eq!(a, 255, "alpha should be 255");
    }

    #[test]
    fn gradient_bottom_right_is_red_green_blue() {
        let [r, g, b, a] = gradient_color(99, 99, 100, 100);
        // 99/100 * 255 = 252.45 -> 252
        assert_eq!(r, 252, "bottom-right red should be ~252");
        assert_eq!(g, 252, "bottom-right green should be ~252");
        assert_eq!(b, 128, "blue should be constant 128");
        assert_eq!(a, 255, "alpha should be 255");
    }

    #[test]
    fn gradient_center_is_half_intensity() {
        let [r, g, b, a] = gradient_color(50, 50, 100, 100);
        // 50/100 * 255 = 127.5 -> 127
        assert_eq!(r, 127, "center red should be ~127");
        assert_eq!(g, 127, "center green should be ~127");
        assert_eq!(b, 128, "blue should be constant 128");
        assert_eq!(a, 255, "alpha should be 255");
    }
}
