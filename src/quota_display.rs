use serde::{Deserialize, Deserializer, Serialize};

use crate::poller::remaining_percentage;

/// One setting controls both Codex rows, independently of their language/colors.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum QuotaDisplayMode {
    #[default]
    Remaining,
    Used,
}

impl<'de> Deserialize<'de> for QuotaDisplayMode {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = serde_json::Value::deserialize(deserializer)?;
        Ok(match value.as_str() {
            Some("used") => Self::Used,
            _ => Self::Remaining,
        })
    }
}

/// Input is always quota used; only the value/length changes with this setting.
pub fn display_percentage(used_percentage: f64, mode: QuotaDisplayMode) -> f64 {
    let used = if used_percentage.is_finite() {
        used_percentage.clamp(0.0, 100.0)
    } else {
        100.0
    };
    match mode {
        QuotaDisplayMode::Remaining => remaining_percentage(used),
        QuotaDisplayMode::Used => used,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_modes_convert_used_quota_consistently() {
        for (used, remaining) in [
            (0.0, 100.0),
            (20.0, 80.0),
            (50.0, 50.0),
            (80.0, 20.0),
            (100.0, 0.0),
        ] {
            assert_eq!(
                display_percentage(used, QuotaDisplayMode::Remaining),
                remaining
            );
            assert_eq!(display_percentage(used, QuotaDisplayMode::Used), used);
        }
        assert_eq!(display_percentage(30.1, QuotaDisplayMode::Remaining), 69.9);
        assert_eq!(display_percentage(-10.0, QuotaDisplayMode::Used), 0.0);
        assert_eq!(display_percentage(120.0, QuotaDisplayMode::Remaining), 0.0);
        assert_eq!(
            display_percentage(f64::NAN, QuotaDisplayMode::Remaining),
            0.0
        );
    }
}
