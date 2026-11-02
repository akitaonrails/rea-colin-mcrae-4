//! CMR4's 32-bit registry locations and resolution choices.

use crate::config::Resolution;

pub const KEY: &str = r"SOFTWARE\Codemasters\Colin McRae Rally 04";
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

/// Wine's bus driver (`rea_launcher::wine::WINEBUS_KEY`): present pads to
/// the game as XInput controllers, which Xidi then reads. Seeded once; a
/// value the player set (for instance `Map Controllers = 0` to hide the pad
/// from XInput) is kept.
pub const WINEBUS_DEFAULTS: [(&str, u32); 1] = [(rea_launcher::wine::MAP_CONTROLLERS, 1)];

pub fn install_root(runtime: &std::path::Path) -> String {
    runtime
        .to_string_lossy()
        .trim_end_matches(['\\', '/'])
        .to_owned()
}

/// One value the launcher writes under [`KEY`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegWrite {
    pub name: &'static str,
    pub value: RegValue,
    /// Written only when the value is absent (first-run defaults).
    pub if_missing: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegValue {
    String(String),
    Dword(u32),
}

impl RegWrite {
    /// `name = string:"…"` / `name = dword:N`, plus `(if missing)`.
    pub fn describe(&self) -> String {
        let value = match &self.value {
            RegValue::String(text) => format!("string:{text:?}"),
            RegValue::Dword(n) => format!("dword:{n}"),
        };
        let when = if self.if_missing { " (if missing)" } else { "" };
        format!("{} = {value}{when}", self.name)
    }
}

/// Every value a launch writes: the install paths and the game's mode on
/// every launch, the language and first-run graphics defaults only when
/// missing. `existing`: the Width/Height already there.
pub fn writes(
    game_mode: Resolution,
    runtime: &std::path::Path,
    existing: Option<(u32, u32)>,
) -> Vec<RegWrite> {
    let root = install_root(runtime);
    let always = |name, value| RegWrite {
        name,
        value,
        if_missing: false,
    };
    let mut out: Vec<RegWrite> = INSTALL_PATH_VALUES
        .iter()
        .map(|name| always(name, RegValue::String(root.clone())))
        .collect();
    let (width, height) = dimensions(game_mode, existing);
    out.push(always("Width", RegValue::Dword(width)));
    out.push(always("Height", RegValue::Dword(height)));
    out.push(RegWrite {
        name: "LANGUAGE",
        value: RegValue::String("E".into()),
        if_missing: true,
    });
    out.extend(DEFAULT_DWORDS.iter().map(|&(name, value)| RegWrite {
        name,
        value: RegValue::Dword(value),
        if_missing: true,
    }));
    out
}

/// Custom modes are enforced every launch. Original mode keeps the current
/// game values, seeding a standard original mode only when they are absent.
pub fn dimensions(resolution: Resolution, existing: Option<(u32, u32)>) -> (u32, u32) {
    match resolution {
        Resolution::Custom(width, height) => (width, height),
        Resolution::Original => existing.unwrap_or(ORIGINAL_DEFAULT),
    }
}

/// The mode CMR04 itself renders. The 16:9 Low patch narrows the camera by
/// 0.75, which is only correct when the game renders 4:3 and dgVoodoo
/// stretches that frame to 16:9. Rendering 16:9 natively applies the
/// correction twice and squeezes the 3D view horizontally.
pub fn game_mode(resolution: Resolution, widescreen: bool) -> Resolution {
    match resolution {
        Resolution::Custom(width, height)
            if widescreen && u64::from(width) * 9 == u64::from(height) * 16 =>
        {
            Resolution::Custom(height / 3 * 4, height)
        }
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn widescreen_renders_4_by_3_for_the_anamorphic_patch() {
        assert_eq!(
            game_mode(Resolution::Custom(3840, 2160), true),
            Resolution::Custom(2880, 2160)
        );
        assert_eq!(
            game_mode(Resolution::Custom(1920, 1080), true),
            Resolution::Custom(1440, 1080)
        );
        assert_eq!(
            game_mode(Resolution::Custom(3840, 2160), false),
            Resolution::Custom(3840, 2160)
        );
        assert_eq!(game_mode(Resolution::Original, true), Resolution::Original);
    }

    #[test]
    fn only_exact_16_by_9_with_widescreen_is_narrowed() {
        for (resolution, widescreen) in [
            (Resolution::Custom(2560, 1080), true),
            (Resolution::Custom(1280, 800), true),
            (Resolution::Custom(1280, 800), false),
            (Resolution::Custom(1600, 1200), false),
        ] {
            assert_eq!(game_mode(resolution, widescreen), resolution);
        }
        // Every exact 16:9 height is a multiple of 9, so the 4:3 width is exact.
        assert_eq!(
            game_mode(Resolution::Custom(1360, 765), true),
            Resolution::Custom(1020, 765)
        );
        assert_eq!(
            game_mode(Resolution::Custom(1280, 720), true),
            Resolution::Custom(960, 720)
        );
    }

    #[test]
    fn registry_receives_the_game_mode_not_the_output_mode() {
        let game = game_mode(Resolution::Custom(3840, 2160), true);
        assert_eq!(dimensions(game, Some((1024, 768))), (2880, 2160));
        let game = game_mode(Resolution::Original, false);
        assert_eq!(dimensions(game, Some((800, 600))), (800, 600));
    }

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
        use rea_launcher_win32::reg::{uses_32_bit_view, virtual_store_path};
        assert!(KEY.starts_with(r"SOFTWARE\Codemasters\"));
        assert!(uses_32_bit_view(true, KEY));
        // The VirtualStore key older launchers wrote on 64-bit Windows.
        assert_eq!(
            virtual_store_path(KEY, true),
            r"Software\Classes\VirtualStore\MACHINE\SOFTWARE\WOW6432Node\Codemasters\Colin McRae Rally 04"
        );
        // Wine's bus key opens without a view flag, as before.
        assert!(!uses_32_bit_view(true, rea_launcher::wine::WINEBUS_KEY));
        assert_eq!(WINEBUS_DEFAULTS, [("Map Controllers", 1)]);
    }
}
