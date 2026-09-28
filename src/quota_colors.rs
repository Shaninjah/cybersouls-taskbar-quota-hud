use std::time::{Duration, SystemTime};

use crate::native_interop::Color;

pub const CODEX_SESSION_CYCLE: Duration = Duration::from_secs(5 * 60 * 60);
pub const CODEX_WEEKLY_CYCLE: Duration = Duration::from_secs(7 * 24 * 60 * 60);
const SECONDS_PER_DAY: f64 = 86_400.0;

/// Linear budget comparison, with fractional days and no display rounding.
pub fn quota_buffer_days(
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
            / SECONDS_PER_DAY,
    )
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum QuotaPaceBand {
    DarkGreen,
    LightGreen,
    LightYellow,
    DarkYellow,
    LightRed,
    DarkRed,
}

fn pace_band(buffer_days: f64, cycle: Duration) -> QuotaPaceBand {
    // Weekly thresholds apply proportionally to the five-hour cycle as well.
    // Sub-microsecond tolerance keeps exact thresholds stable after f64 scaling.
    let weekly_buffer =
        buffer_days * CODEX_WEEKLY_CYCLE.as_secs_f64() / cycle.as_secs_f64() + 1e-12;
    if weekly_buffer >= 1.5 {
        QuotaPaceBand::DarkGreen
    } else if weekly_buffer >= 0.0 {
        QuotaPaceBand::LightGreen
    } else if weekly_buffer >= -0.5 {
        QuotaPaceBand::LightYellow
    } else if weekly_buffer >= -1.0 {
        QuotaPaceBand::DarkYellow
    } else if weekly_buffer >= -2.0 {
        QuotaPaceBand::LightRed
    } else {
        QuotaPaceBand::DarkRed
    }
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
    let buffer = quota_buffer_days(remaining, cycle, until_reset)?;
    let hex = match (pace_band(buffer, cycle), is_dark) {
        (QuotaPaceBand::DarkGreen, true) => "#22C55E",
        (QuotaPaceBand::DarkGreen, false) => "#166534",
        (QuotaPaceBand::LightGreen, true) => "#86EFAC",
        (QuotaPaceBand::LightGreen, false) => "#15803D",
        (QuotaPaceBand::LightYellow, true) => "#FEF08A",
        (QuotaPaceBand::LightYellow, false) => "#A16207",
        (QuotaPaceBand::DarkYellow, true) => "#EAB308",
        (QuotaPaceBand::DarkYellow, false) => "#854D0E",
        (QuotaPaceBand::LightRed, true) => "#FCA5A5",
        (QuotaPaceBand::LightRed, false) => "#DC2626",
        (QuotaPaceBand::DarkRed, true) => "#EF4444",
        (QuotaPaceBand::DarkRed, false) => "#991B1B",
    };
    Some(Color::from_hex(hex))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::poller::remaining_percentage;

    fn days(value: f64) -> Duration {
        Duration::from_secs_f64(value * SECONDS_PER_DAY)
    }

    #[test]
    fn weekly_examples_compare_quota_with_actual_reset_time() {
        for (remaining, reset_days, buffer, expected) in [
            (50.0, 2.0, 1.5, QuotaPaceBand::DarkGreen),
            (50.0, 5.0, -1.5, QuotaPaceBand::LightRed),
            (10.0, 5.0, -4.3, QuotaPaceBand::DarkRed),
            (50.0, 3.5, 0.0, QuotaPaceBand::LightGreen),
            (100.0, 7.0, 0.0, QuotaPaceBand::LightGreen),
        ] {
            let actual =
                quota_buffer_days(remaining, CODEX_WEEKLY_CYCLE, days(reset_days)).unwrap();
            assert!((actual - buffer).abs() < 1e-10);
            assert_eq!(pace_band(actual, CODEX_WEEKLY_CYCLE), expected);
        }
    }

    #[test]
    fn all_buffer_boundaries_have_the_requested_bands() {
        for (buffer, expected) in [
            (1.5, QuotaPaceBand::DarkGreen),
            (1.499, QuotaPaceBand::LightGreen),
            (0.5, QuotaPaceBand::LightGreen),
            (0.499, QuotaPaceBand::LightGreen),
            (0.0, QuotaPaceBand::LightGreen),
            (-0.001, QuotaPaceBand::LightYellow),
            (-0.5, QuotaPaceBand::LightYellow),
            (-0.501, QuotaPaceBand::DarkYellow),
            (-1.0, QuotaPaceBand::DarkYellow),
            (-1.001, QuotaPaceBand::LightRed),
            (-2.0, QuotaPaceBand::LightRed),
            (-2.001, QuotaPaceBand::DarkRed),
        ] {
            assert_eq!(pace_band(buffer, CODEX_WEEKLY_CYCLE), expected, "{buffer}");
        }
    }

    #[test]
    fn precise_seconds_and_elapsed_time_change_pace_without_a_new_fetch() {
        let now = SystemTime::UNIX_EPOCH + days(100.0);
        let reset = now + days(3.5);
        let color = |time| {
            codex_quota_color_from_pace(50.0, Some(reset), CODEX_WEEKLY_CYCLE, time, true)
                .unwrap()
                .to_colorref()
        };
        assert_eq!(color(now), Color::from_hex("#86EFAC").to_colorref());
        assert_eq!(
            color(now - Duration::from_secs(1)),
            Color::from_hex("#FEF08A").to_colorref()
        );
        assert_eq!(
            color(now + days(1.5)),
            Color::from_hex("#22C55E").to_colorref()
        );
    }

    #[test]
    fn five_hour_cycle_uses_proportionate_weekly_thresholds() {
        for buffer in [1.5, 0.5, 0.0, -0.5, -1.0, -2.0, -3.0] {
            let fraction = buffer / 7.0;
            assert_eq!(
                pace_band(
                    fraction * CODEX_SESSION_CYCLE.as_secs_f64() / SECONDS_PER_DAY,
                    CODEX_SESSION_CYCLE
                ),
                pace_band(buffer, CODEX_WEEKLY_CYCLE)
            );
        }
        assert_eq!(
            quota_buffer_days(80.0, CODEX_SESSION_CYCLE, Duration::from_secs(4 * 3600)),
            Some(0.0)
        );
    }

    #[test]
    fn used_conversion_palette_and_unknown_data_remain_safe() {
        let now = SystemTime::UNIX_EPOCH + days(100.0);
        for (buffer, dark, light) in [
            (1.5, "#22C55E", "#166534"),
            (0.0, "#86EFAC", "#15803D"),
            (-0.5, "#FEF08A", "#A16207"),
            (-1.0, "#EAB308", "#854D0E"),
            (-2.0, "#FCA5A5", "#DC2626"),
            (-2.1, "#EF4444", "#991B1B"),
        ] {
            let reset = now + days(3.5 - buffer);
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
            (f64::NAN, Some(now + days(2.0)), CODEX_WEEKLY_CYCLE),
            (50.0, Some(now + days(2.0)), Duration::ZERO),
        ] {
            assert!(codex_quota_color_from_pace(remaining, reset, cycle, now, true).is_none());
        }
        assert_eq!(
            codex_quota_color_from_pace(0.0, Some(now), CODEX_WEEKLY_CYCLE, now, true)
                .unwrap()
                .to_colorref(),
            Color::from_hex("#86EFAC").to_colorref()
        );
    }
}
