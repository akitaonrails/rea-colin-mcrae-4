//! CMR4's 32-bit registry locations and resolution choices.

use crate::config::Resolution;

pub const KEY: &str = r"SOFTWARE\Codemasters\Colin McRae Rally 04";
pub const VIRTUAL_STORE_KEY: &str =
    r"Software\Classes\VirtualStore\MACHINE\SOFTWARE\WOW6432Node\Codemasters\Colin McRae Rally 04";
pub const WINEBUS_KEY: &str = r"System\CurrentControlSet\Services\winebus";
pub const ORIGINAL_DEFAULT: (u32, u32) = (1024, 768);

/// Custom modes are enforced every launch. Original mode keeps the current
/// game values, seeding a standard original mode only when they are absent.
pub fn dimensions(resolution: Resolution, existing: Option<(u32, u32)>) -> (u32, u32) {
    match resolution {
        Resolution::Custom(width, height) => (width, height),
        Resolution::Original => existing.unwrap_or(ORIGINAL_DEFAULT),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn custom_resolution_always_wins() {
        assert_eq!(
            dimensions(Resolution::Custom(3840, 2160), Some((800, 600))),
            (3840, 2160)
        );
    }

    #[test]
    fn original_preserves_existing_mode_and_has_a_safe_initial_value() {
        assert_eq!(
            dimensions(Resolution::Original, Some((800, 600))),
            (800, 600)
        );
        assert_eq!(dimensions(Resolution::Original, None), (1024, 768));
    }

    #[test]
    fn paths_use_the_32_bit_game_and_virtual_store_views() {
        assert!(KEY.starts_with(r"SOFTWARE\Codemasters\"));
        assert!(VIRTUAL_STORE_KEY.contains(r"WOW6432Node\Codemasters"));
        assert_eq!(WINEBUS_KEY, r"System\CurrentControlSet\Services\winebus");
    }
}
