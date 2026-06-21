/// Clamps a percentage value to the range 0..=100.
pub fn clamp_percentage(value: i32) -> u8 {
    if value < 0 {
        0
    } else if value > 100 {
        100
    } else {
        value as u8
    }
}

/// Converts a percentage (0..=100) to a Windows alpha value (0..=255).
pub fn percentage_to_alpha(percentage: u8) -> u8 {
    let pct = percentage.min(100);
    ((pct as u16 * 255 + 50) / 100) as u8
}

/// Checks if the opacity percentage is below the warning threshold (15%).
pub fn is_below_warning_threshold(percentage: u8) -> bool {
    percentage < 15
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clamp_percentage() {
        assert_eq!(clamp_percentage(50), 50);
        assert_eq!(clamp_percentage(-10), 0);
        assert_eq!(clamp_percentage(150), 100);
        assert_eq!(clamp_percentage(0), 0);
        assert_eq!(clamp_percentage(100), 100);
    }

    #[test]
    fn test_percentage_to_alpha() {
        assert_eq!(percentage_to_alpha(0), 0);
        assert_eq!(percentage_to_alpha(100), 255);
        assert_eq!(percentage_to_alpha(50), 128);
        assert_eq!(percentage_to_alpha(15), 38);
        assert_eq!(percentage_to_alpha(120), 255); // Clamped behavior
    }

    #[test]
    fn test_is_below_warning_threshold() {
        assert_eq!(is_below_warning_threshold(0), true);
        assert_eq!(is_below_warning_threshold(14), true);
        assert_eq!(is_below_warning_threshold(15), false);
        assert_eq!(is_below_warning_threshold(50), false);
    }
}
