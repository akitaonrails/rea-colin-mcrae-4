//! CMR4's 32-bit registry locations and resolution choices.

use crate::config::Resolution;

pub const KEY: &str = r"SOFTWARE\Codemasters\Colin McRae Rally 04";
pub const VIRTUAL_STORE_KEY: &str =
    r"Software\Classes\VirtualStore\MACHINE\SOFTWARE\WOW6432Node\Codemasters\Colin McRae Rally 04";
pub const WINEBUS_KEY: &str = r"System\CurrentControlSet\Services\winebus";
pub const ORIGINAL_DEFAULT: (u32, u32) = (1024, 768);
pub const INSTALL_PATH_VALUES: [&str; 4] = ["AUDIO_PATH", "CD_PATH", "INSTALL_PATH", "VIDEO_PATH"];
pub const DEFAULT_DWORDS: [(&str, u32); 9] = [
    ("Adapter", 0),
    ("ADAPTER_PID", 0),
    ("ADAPTER_VID", 0),
    ("BitDepth", 1),
    ("ZDepth", 1),
    ("SHADERS", 1),
    ("FSAA", 0),
    ("Gamma", 1),
    ("MOVIE_QUALITY", 0),
];

pub fn install_root(runtime: &std::path::Path) -> String {
    runtime
        .to_string_lossy()
        .trim_end_matches(['\\', '/'])
        .to_owned()
}

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
    fn install_paths_and_seed_defaults_match_the_working_recipe() {
        assert_eq!(install_root(std::path::Path::new(r"C:\CMR4\")), r"C:\CMR4");
        assert_eq!(
            INSTALL_PATH_VALUES,
            ["AUDIO_PATH", "CD_PATH", "INSTALL_PATH", "VIDEO_PATH"]
        );
        assert_eq!(DEFAULT_DWORDS.len(), 9);
        for (name, _) in DEFAULT_DWORDS {
            assert!(name.is_ascii());
        }
        assert_eq!(DEFAULT_DWORDS[8], ("MOVIE_QUALITY", 0));
    }

    #[test]
    fn paths_use_the_32_bit_game_and_virtual_store_views() {
        assert!(KEY.starts_with(r"SOFTWARE\Codemasters\"));
        assert!(VIRTUAL_STORE_KEY.contains(r"WOW6432Node\Codemasters"));
        assert_eq!(WINEBUS_KEY, r"System\CurrentControlSet\Services\winebus");
    }
}
