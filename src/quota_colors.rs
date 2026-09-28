use crate::native_interop::Color;

/// Codex colors always describe quota remaining, never the displayed used value.
/// Dark colors are the base palette; light variants improve small-text contrast.
pub fn codex_quota_color_from_remaining(remaining: f64, is_dark: bool) -> Color {
    let remaining = if remaining.is_finite() {
        remaining.clamp(0.0, 100.0)
    } else {
        0.0
    };
    let hex = match (remaining, is_dark) {
        (p, true) if p >= 70.0 => "#22D3EE",
        (p, false) if p >= 70.0 => "#0E7490",
        (p, true) if p >= 40.0 => "#3B82F6",
        (p, false) if p >= 40.0 => "#1D4ED8",
        (p, true) if p >= 20.0 => "#F59E0B",
        (p, false) if p >= 20.0 => "#92400E",
        (_, true) => "#EF4444",
        (_, false) => "#B91C1C",
    };
    Color::from_hex(hex)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::poller::remaining_percentage;

    #[test]
    fn all_remaining_boundaries_select_the_expected_palette() {
        for (remaining, dark, light) in [
            (100.0, "#22D3EE", "#0E7490"),
            (70.0, "#22D3EE", "#0E7490"),
            (69.0, "#3B82F6", "#1D4ED8"),
            (40.0, "#3B82F6", "#1D4ED8"),
            (39.0, "#F59E0B", "#92400E"),
            (20.0, "#F59E0B", "#92400E"),
            (19.0, "#EF4444", "#B91C1C"),
            (0.0, "#EF4444", "#B91C1C"),
        ] {
            for (is_dark, expected) in [(true, dark), (false, light)] {
                assert_eq!(
                    codex_quota_color_from_remaining(remaining, is_dark).to_colorref(),
                    Color::from_hex(expected).to_colorref(),
                    "remaining={remaining}, dark={is_dark}"
                );
            }
        }
    }

    #[test]
    fn used_to_remaining_conversion_is_not_inverted() {
        for (used, expected) in [
            (10.0, "#22D3EE"),
            (50.0, "#3B82F6"),
            (70.0, "#F59E0B"),
            (90.0, "#EF4444"),
        ] {
            assert_eq!(
                codex_quota_color_from_remaining(remaining_percentage(used), true).to_colorref(),
                Color::from_hex(expected).to_colorref()
            );
        }
    }

    #[test]
    fn fractional_values_and_invalid_inputs_have_defined_bands() {
        for (remaining, expected) in [
            (69.9, "#3B82F6"),
            (39.9, "#F59E0B"),
            (19.9, "#EF4444"),
            (120.0, "#22D3EE"),
            (-5.0, "#EF4444"),
            (f64::NAN, "#EF4444"),
        ] {
            assert_eq!(
                codex_quota_color_from_remaining(remaining, true).to_colorref(),
                Color::from_hex(expected).to_colorref()
            );
        }
    }
}
