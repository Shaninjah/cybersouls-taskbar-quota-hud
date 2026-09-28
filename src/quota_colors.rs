use std::time::{Duration, SystemTime};

use crate::native_interop::Color;

pub const CODEX_SESSION_CYCLE: Duration = Duration::from_secs(5 * 60 * 60);
pub const CODEX_WEEKLY_CYCLE: Duration = Duration::from_secs(7 * 24 * 60 * 60);
const SECONDS_PER_HOUR: f64 = 3_600.0;

/// Linear budget comparison, with fractional hours and no display rounding.
pub fn quota_buffer_hours(
    remaining: f64,
    cycle: Duration,
    time_until_reset: Duration,
) -> Option<f64> {
    if !remaining.is_finite() || cycle.is_zero() {
        return None;
    }
    Some(
        (remaining.clamp(0.0, 100.0) / 100.0 * cycle.as_secs_f64()
            - time_until_reset.as_secs_f64())
            / SECONDS_PER_HOUR,
    )
}

// Ordered from comfortably ahead to critically behind. Each color family has
// four nearby shades; light-theme colors stay dark enough for small value text.
const PACE_PALETTE: [(f64, &str, &str); 16] = [
    (36.0, "#22C55E", "#166534"),
    (24.0, "#4ACF65", "#236A2B"),
    (12.0, "#72D56C", "#356E25"),
    (0.0, "#9BDC72", "#4B7221"),
    (-6.0, "#C6DD6B", "#65741D"),
    (-12.0, "#DDE05B", "#777019"),
    (-18.0, "#EACD47", "#8A6817"),
    (-24.0, "#F2BC35", "#9B5D14"),
    (-30.0, "#F7A72B", "#A65216"),
    (-36.0, "#F99028", "#AF471C"),
    (-42.0, "#F77B2D", "#B63C22"),
    (-48.0, "#F36835", "#BB3128"),
    (-60.0, "#EF593E", "#BA2B2D"),
    (-72.0, "#EF5342", "#B72530"),
    (-84.0, "#EF4D47", "#AC2030"),
    (f64::NEG_INFINITY, "#EF474C", "#991B2B"),
];

fn pace_band(buffer_hours: f64, cycle: Duration) -> usize {
    // Weekly thresholds apply proportionally to the five-hour cycle as well.
    // Sub-microsecond tolerance keeps exact thresholds stable after f64 scaling.
    let weekly_buffer_hours =
        buffer_hours * CODEX_WEEKLY_CYCLE.as_secs_f64() / cycle.as_secs_f64() + 24e-12;
    PACE_PALETTE
        .iter()
        .position(|(minimum, _, _)| weekly_buffer_hours >= *minimum)
        .unwrap_or(PACE_PALETTE.len() - 1)
}

/// Pace color uses remaining budget and its own reset, never the display mode.
/// Missing/expired reset times or invalid data have no inferred pace color.
pub fn codex_quota_color_from_pace(
    remaining: f64,
    resets_at: Option<SystemTime>,
    cycle: Duration,
    now: SystemTime,
    is_dark: bool,
) -> Option<Color> {
    let until_reset = resets_at?.duration_since(now).ok()?;
    let buffer = quota_buffer_hours(remaining, cycle, until_reset)?;
    let (_, dark, light) = PACE_PALETTE[pace_band(buffer, cycle)];
    let hex = if is_dark { dark } else { light };
    Some(Color::from_hex(hex))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::poller::remaining_percentage;

    fn hours(value: f64) -> Duration {
        Duration::from_secs_f64(value * SECONDS_PER_HOUR)
    }

    #[test]
    fn weekly_examples_compare_quota_with_actual_reset_time() {
        for (remaining, reset_hours, buffer, expected) in [
            (50.0, 48.0, 36.0, 0),
            (50.0, 120.0, -36.0, 9),
            (10.0, 120.0, -103.2, 15),
            (50.0, 84.0, 0.0, 3),
            (100.0, 168.0, 0.0, 3),
        ] {
            let actual =
                quota_buffer_hours(remaining, CODEX_WEEKLY_CYCLE, hours(reset_hours)).unwrap();
            assert!((actual - buffer).abs() < 1e-10);
            assert_eq!(pace_band(actual, CODEX_WEEKLY_CYCLE), expected);
        }
    }

    #[test]
    fn all_buffer_boundaries_have_the_requested_bands() {
        for (boundary, expected) in [
            (36.0, 0),
            (24.0, 1),
            (12.0, 2),
            (0.0, 3),
            (-6.0, 4),
            (-12.0, 5),
            (-18.0, 6),
            (-24.0, 7),
            (-30.0, 8),
            (-36.0, 9),
            (-42.0, 10),
            (-48.0, 11),
            (-60.0, 12),
            (-72.0, 13),
            (-84.0, 14),
        ] {
            for cycle in [CODEX_WEEKLY_CYCLE, CODEX_SESSION_CYCLE] {
                let scale = cycle.as_secs_f64() / CODEX_WEEKLY_CYCLE.as_secs_f64();
                assert_eq!(pace_band(boundary * scale, cycle), expected);
                assert_eq!(pace_band((boundary - 0.001) * scale, cycle), expected + 1);
                assert_eq!(pace_band((boundary + 0.001) * scale, cycle), expected);
            }
        }
        assert_eq!(pace_band(-168.0, CODEX_WEEKLY_CYCLE), 15);
        assert_eq!(pace_band(168.0, CODEX_WEEKLY_CYCLE), 0);
    }

    #[test]
    fn precise_seconds_and_elapsed_time_change_pace_without_a_new_fetch() {
        let now = SystemTime::UNIX_EPOCH + hours(2400.0);
        let reset = now + hours(84.0);
        let fractional = quota_buffer_hours(
            50.0,
            CODEX_WEEKLY_CYCLE,
            hours(84.0) + Duration::from_secs_f64(0.25),
        )
        .unwrap();
        assert!((fractional + 0.25 / SECONDS_PER_HOUR).abs() < 1e-12);
        let color = |time| {
            codex_quota_color_from_pace(50.0, Some(reset), CODEX_WEEKLY_CYCLE, time, true)
                .unwrap()
                .to_colorref()
        };
        assert_eq!(color(now), Color::from_hex("#9BDC72").to_colorref());
        assert_eq!(
            color(now - Duration::from_secs(1)),
            Color::from_hex("#C6DD6B").to_colorref()
        );
        assert_eq!(
            color(now + hours(36.0)),
            Color::from_hex("#22C55E").to_colorref()
        );
    }

    #[test]
    fn five_hour_cycle_uses_proportionate_weekly_thresholds() {
        for buffer in [36.0, 12.0, 0.0, -12.0, -24.0, -48.0, -72.0] {
            let fraction = buffer / 168.0;
            assert_eq!(
                pace_band(
                    fraction * CODEX_SESSION_CYCLE.as_secs_f64() / SECONDS_PER_HOUR,
                    CODEX_SESSION_CYCLE
                ),
                pace_band(buffer, CODEX_WEEKLY_CYCLE)
            );
        }
        assert_eq!(
            quota_buffer_hours(80.0, CODEX_SESSION_CYCLE, Duration::from_secs(4 * 3600)),
            Some(0.0)
        );
    }

    #[test]
    fn sixteen_shades_are_distinct_readable_and_have_small_adjacent_steps() {
        fn luminance(color: Color) -> f64 {
            let linear = |channel: u8| {
                let value = f64::from(channel) / 255.0;
                if value <= 0.04045 {
                    value / 12.92
                } else {
                    ((value + 0.055) / 1.055).powf(2.4)
                }
            };
            0.2126 * linear(color.r) + 0.7152 * linear(color.g) + 0.0722 * linear(color.b)
        }

        for is_dark in [true, false] {
            let background =
                luminance(Color::from_hex(if is_dark { "#1C1C1C" } else { "#F3F3F3" }));
            let colors: Vec<_> = PACE_PALETTE
                .iter()
                .map(|(_, dark, light)| Color::from_hex(if is_dark { dark } else { light }))
                .collect();
            let distinct: std::collections::HashSet<_> =
                colors.iter().map(|color| color.to_colorref()).collect();
            assert_eq!(distinct.len(), 16);
            for color in &colors {
                let foreground = luminance(*color);
                let contrast =
                    (foreground.max(background) + 0.05) / (foreground.min(background) + 0.05);
                assert!(contrast >= 4.5, "contrast {contrast}, dark: {is_dark}");
            }
            for pair in colors.windows(2) {
                let step = ((f64::from(pair[0].r) - f64::from(pair[1].r)).powi(2)
                    + (f64::from(pair[0].g) - f64::from(pair[1].g)).powi(2)
                    + (f64::from(pair[0].b) - f64::from(pair[1].b)).powi(2))
                .sqrt();
                assert!(step <= 65.0, "adjacent RGB step {step}, dark: {is_dark}");
            }
        }
    }

    #[test]
    fn used_conversion_palette_and_unknown_data_remain_safe() {
        let now = SystemTime::UNIX_EPOCH + hours(2400.0);
        for (buffer, dark, light) in [
            (36.0, "#22C55E", "#166534"),
            (24.0, "#4ACF65", "#236A2B"),
            (12.0, "#72D56C", "#356E25"),
            (0.0, "#9BDC72", "#4B7221"),
            (-6.0, "#C6DD6B", "#65741D"),
            (-12.0, "#DDE05B", "#777019"),
            (-18.0, "#EACD47", "#8A6817"),
            (-24.0, "#F2BC35", "#9B5D14"),
            (-30.0, "#F7A72B", "#A65216"),
            (-36.0, "#F99028", "#AF471C"),
            (-42.0, "#F77B2D", "#B63C22"),
            (-48.0, "#F36835", "#BB3128"),
            (-60.0, "#EF593E", "#BA2B2D"),
            (-72.0, "#EF5342", "#B72530"),
            (-84.0, "#EF4D47", "#AC2030"),
            (-90.0, "#EF474C", "#991B2B"),
        ] {
            let reset = now + hours(84.0 - buffer);
            for (is_dark, hex) in [(true, dark), (false, light)] {
                assert_eq!(
                    codex_quota_color_from_pace(
                        remaining_percentage(50.0),
                        Some(reset),
                        CODEX_WEEKLY_CYCLE,
                        now,
                        is_dark
                    )
                    .unwrap()
                    .to_colorref(),
                    Color::from_hex(hex).to_colorref()
                );
            }
        }
        for (remaining, reset, cycle) in [
            (50.0, None, CODEX_WEEKLY_CYCLE),
            (50.0, Some(now - Duration::from_secs(1)), CODEX_WEEKLY_CYCLE),
            (f64::NAN, Some(now + hours(48.0)), CODEX_WEEKLY_CYCLE),
            (50.0, Some(now + hours(48.0)), Duration::ZERO),
        ] {
            assert!(codex_quota_color_from_pace(remaining, reset, cycle, now, true).is_none());
        }
        assert_eq!(
            codex_quota_color_from_pace(0.0, Some(now), CODEX_WEEKLY_CYCLE, now, true)
                .unwrap()
                .to_colorref(),
            Color::from_hex("#9BDC72").to_colorref()
        );
    }
}
