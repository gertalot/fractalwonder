use leptos::*;

use crate::rendering::RenderProgress;

/// Generate SVG path for pie chart based on percentage.
fn create_pie_path(percent: f64) -> String {
    if percent <= 0.0 {
        return String::new();
    }

    let angle = (percent / 100.0) * 360.0;
    let radians = (angle - 90.0).to_radians(); // -90 to start at 12 o'clock
    let end_x = 12.0 + 10.0 * radians.cos();
    let end_y = 12.0 + 10.0 * radians.sin();
    let large_arc = if angle > 180.0 { 1 } else { 0 };

    format!(
        "M 12 12 L 12 2 A 10 10 0 {} 1 {:.2} {:.2} Z",
        large_arc, end_x, end_y
    )
}

/// Generate SVG path for a 90-degree arc (used for indeterminate spinner).
fn create_arc_path() -> &'static str {
    // Arc from 12 o'clock (0°) to 3 o'clock (90°)
    // Start at top (12, 2), arc to right (22, 12)
    "M 12 2 A 10 10 0 0 1 22 12"
}

#[component]
pub fn CircularProgress(
    progress: Signal<RwSignal<RenderProgress>>,
    is_ui_visible: ReadSignal<bool>,
) -> impl IntoView {
    // Get current phase
    let current_phase = create_memo(move |_| {
        let progress_signal = progress.get();
        progress_signal.get().phase
    });

    // Calculate progress percentage (for determinate mode)
    let progress_percent = create_memo(move |_| {
        let progress_signal = progress.get();
        let p = progress_signal.get();
        if p.total_steps > 0 {
            (p.completed_steps as f64 / p.total_steps as f64 * 100.0).min(100.0)
        } else {
            0.0
        }
    });

    // Visibility: show when active AND UI is hidden
    let should_show = create_memo(move |_| {
        let progress_signal = progress.get();
        let p = progress_signal.get();
        p.is_active() && !is_ui_visible.get()
    });

    let opacity_class = move || {
        if should_show.get() {
            "opacity-100"
        } else {
            "opacity-0"
        }
    };

    // Check if current phase is indeterminate
    let is_indeterminate = move || current_phase.get().is_indeterminate();

    view! {
        <div
            class=move || format!(
                "fixed left-[28px] bottom-[24px] transition-opacity duration-300 pointer-events-none {}",
                opacity_class()
            )
        >
            <div class="w-6 h-6 bg-black/50 backdrop-blur-sm rounded-full flex items-center justify-center">
                <svg width="24" height="24" viewBox="0 0 24 24" class="transform">
                    // Background circle
                    <circle
                        cx="12"
                        cy="12"
                        r="10"
                        fill="none"
                        stroke="rgb(100,100,100)"
                        stroke-width="1"
                        opacity="0.2"
                    />

                    // Indeterminate: rotating arc
                    {move || is_indeterminate().then(|| view! {
                        <g class="animate-spin" style="transform-origin: center; animation-duration: 1s;">
                            <path
                                d=create_arc_path()
                                fill="none"
                                stroke="rgb(244,244,244)"
                                stroke-width="2"
                                stroke-linecap="round"
                            />
                        </g>
                    })}

                    // Determinate: pie slice
                    {move || (!is_indeterminate()).then(|| view! {
                        <path
                            d={move || create_pie_path(progress_percent.get())}
                            fill="rgb(244,244,244)"
                        />
                    })}
                </svg>
            </div>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_pie_path_zero_percent() {
        let path = create_pie_path(0.0);
        assert_eq!(path, "");
    }

    #[test]
    fn test_create_pie_path_25_percent() {
        let path = create_pie_path(25.0);
        // At 25%, we're at 3 o'clock (90 degrees from 12 o'clock)
        assert!(path.contains("M 12 12 L 12 2 A 10 10 0 0 1 22.00 12.00 Z"));
    }

    #[test]
    fn test_create_pie_path_50_percent() {
        let path = create_pie_path(50.0);
        // At 50%, we're at 6 o'clock (180 degrees from 12 o'clock)
        assert!(path.contains("M 12 12 L 12 2 A 10 10 0 0 1 12.00 22.00 Z"));
    }

    #[test]
    fn test_create_pie_path_75_percent() {
        let path = create_pie_path(75.0);
        // At 75%, we're at 9 o'clock (270 degrees from 12 o'clock)
        assert!(path.contains("M 12 12 L 12 2 A 10 10 0 1 1 2.00 12.00 Z"));
    }

    #[test]
    fn test_create_pie_path_100_percent() {
        let path = create_pie_path(100.0);
        // At 100%, large arc flag should be 1 (>180 degrees)
        assert!(path.contains("A 10 10 0 1 1"));
    }

    #[test]
    fn test_arc_path_format() {
        let path = create_arc_path();
        assert!(path.starts_with("M 12 2"));
        assert!(path.contains("A 10 10"));
    }
}
