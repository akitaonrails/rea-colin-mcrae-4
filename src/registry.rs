//! CMR4's registry: the values the game reads, where the launcher keeps
//! them, the redirect that hands them to the game, and resolution choices.
//!
//! `cmr4.exe` opens `HKLM\SOFTWARE\CODEMASTERS\Colin McRae Rally 04`
//! (`RegCreateKeyExA`, read-write, then read-only when that fails) and reads
//! INSTALL_PATH, WIDTH/HEIGHT, LANGUAGE, ADAPTER_VID/ADAPTER_PID and
//! SPEC_SELECTION1–3, writing the last five back. Nothing else: the exe
//! has no AUDIO_PATH, CD_PATH, VIDEO_PATH, BitDepth, ZDepth, FSAA or
//! MOVIE_QUALITY string, so older launchers' writes of those were dead.
//!
//! An unelevated launcher can't write `HKLM`, and the VirtualStore copy
//! Windows makes instead reaches the game only while UAC registry
//! virtualization is on for it, which Windows doesn't promise
//! (akitaonrails/rea-colin-mcrae#6). So the launcher writes [`USER_KEY`]
//! under `HKCU`, and the close hook DLL in the runtime exe's imports sends
//! the game's opens of its `HKLM` key there ([`REDIRECT`]).

use rea_close_hook::registry::{Config, ConfigError};
use rea_launcher_win32::reg::{Data, Mismatch, Value, When};

use crate::config::Resolution;

/// Where the game looks (`HKLM`, 32-bit view) and where launchers before
/// 0.5.1 wrote (or its VirtualStore copy): read once to carry the game's
/// values over, never written again.
pub const MACHINE_KEY: &str = r"SOFTWARE\Codemasters\Colin McRae Rally 04";
/// Where the launcher keeps the game's values (`HKCU`).
pub const USER_KEY: &str = r"Software\Codemasters\Colin McRae Rally 04";
/// The close hook's `REA_REGISTRY` rule: the game's opens of its `HKLM` key
/// go to [`USER_KEY`].
pub const REDIRECT: &str = r"HKLM\SOFTWARE\Codemasters\Colin McRae Rally 04 -> HKCU\Software\Codemasters\Colin McRae Rally 04";
pub const ORIGINAL_DEFAULT: (u32, u32) = (1024, 768);
/// The game's own values, seeded on a first run: the menus' language and
/// the display adapter the game then records.
pub const FIRST_RUN_DWORDS: [(&str, u32); 2] = [("ADAPTER_VID", 0), ("ADAPTER_PID", 0)];
pub const FIRST_RUN_LANGUAGE: &str = "E";
/// The game's own values carried over from [`MACHINE_KEY`] (besides
/// Width/Height): LANGUAGE (a string) and these DWORDs, which the game
/// writes itself (its graphics detail choices and the adapter).
pub const CARRIED_DWORDS: [&str; 5] = [
    "ADAPTER_VID",
    "ADAPTER_PID",
    "SPEC_SELECTION1",
    "SPEC_SELECTION2",
    "SPEC_SELECTION3",
];
const BUG_REPORT: &str =
    "https://github.com/akitaonrails/rea-colin-mcrae-4/issues/new?template=bug-report.yml";

/// Wine's bus driver (`rea_launcher::wine::WINEBUS_KEY`): present pads to
/// the game as XInput controllers, which Xidi then reads. Seeded once; a
/// value the player set (for instance `Map Controllers = 0` to hide the pad
/// from XInput) is kept.
pub const WINEBUS_DEFAULTS: [(&str, u32); 1] = [(rea_launcher::wine::MAP_CONTROLLERS, 1)];

/// The redirect's rules, checked with the close hook's own parser.
pub fn redirect() -> Result<Config, ConfigError> {
    Config::parse(REDIRECT)
}

pub fn install_root(runtime: &std::path::Path) -> String {
    runtime
        .to_string_lossy()
        .trim_end_matches(['\\', '/'])
        .to_owned()
}

/// What the game had before this launch.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Previous {
    /// Its Width/Height ([`USER_KEY`], else its old key).
    pub size: Option<(u32, u32)>,
    /// Its own values from the old key, while [`USER_KEY`] doesn't exist
    /// yet: seeded into it in place of the first-run defaults.
    pub carried: Vec<(&'static str, Data)>,
}

/// `name = string:"…"` / `name = dword:N`, plus `(if missing)`.
pub fn describe(value: &Value) -> String {
    let data = match &value.data {
        Data::String(text) => format!("string:{text:?}"),
        Data::Dword(n) => format!("dword:{n}"),
    };
    let when = match value.when {
        When::Always => "",
        When::IfMissing => " (if missing)",
    };
    format!("{} = {data}{when}", value.name)
}

/// Every value a launch writes under [`USER_KEY`]: the install path and the
/// game's mode on every launch; the game's own values only when missing,
/// from `previous` when it carried them, else the first-run defaults.
pub fn values(game_mode: Resolution, runtime: &std::path::Path, previous: &Previous) -> Vec<Value> {
    let (width, height) = dimensions(game_mode, previous.size);
    let mut out = vec![
        Value::new(
            "INSTALL_PATH",
            Data::String(install_root(runtime)),
            When::Always,
        ),
        Value::new("Width", Data::Dword(width), When::Always),
        Value::new("Height", Data::Dword(height), When::Always),
    ];
    let first_run = std::iter::once(("LANGUAGE", Data::String(FIRST_RUN_LANGUAGE.into())))
        .chain(FIRST_RUN_DWORDS.map(|(name, n)| (name, Data::Dword(n))));
    let carried = |name: &str| {
        previous
            .carried
            .iter()
            .find(|(carried, _)| *carried == name)
            .map(|(_, data)| data.clone())
    };
    for (name, default) in first_run {
        out.push(Value::new(
            name,
            carried(name).unwrap_or(default),
            When::IfMissing,
        ));
    }
    for (name, data) in &previous.carried {
        if !out.iter().any(|value| value.name == *name) {
            out.push(Value::new(*name, data.clone(), When::IfMissing));
        }
    }
    out
}

/// The player's message when the game would not see its values
/// (`problems`: the close hook missing or not in the exe's imports, the
/// rules refused, values not read back), for Play and `--check`.
pub fn not_ready(problems: &[String]) -> String {
    let mut text =
        String::from("Not ready to play: Colin McRae Rally 04 would not find its settings.\n");
    for problem in problems {
        text.push_str(&format!("\n- {problem}"));
    }
    text.push_str(&format!(
        "\n\nThe launcher keeps the game's settings in HKCU\\{USER_KEY} \
         (no administrator rights needed) and points the game there with \
         rea_close_hook.dll. Extract the launcher's release zip again, \
         components folder included. If this keeps happening, report it \
         with launcher.log attached: {BUG_REPORT}"
    ));
    text
}

/// One problem line per value `write_verified` found wrong.
pub fn mismatch_lines(mismatches: &[Mismatch]) -> Vec<String> {
    mismatches
        .iter()
        .map(|mismatch| format!("HKCU\\{USER_KEY}: {mismatch}"))
        .collect()
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
    use rea_close_hook::registry::{Key, Root};
    use rea_launcher_win32::reg::{Error, Problem};
    use std::path::Path;

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

    fn rendered(values: &[Value]) -> Vec<String> {
        values.iter().map(describe).collect()
    }

    #[test]
    fn a_first_run_writes_only_what_the_game_reads() {
        let values = values(
            Resolution::Custom(1440, 1080),
            Path::new(r"C:\CMR4\"),
            &Previous::default(),
        );
        assert_eq!(
            rendered(&values),
            [
                r#"INSTALL_PATH = string:"C:\\CMR4""#,
                "Width = dword:1440",
                "Height = dword:1080",
                r#"LANGUAGE = string:"E" (if missing)"#,
                "ADAPTER_VID = dword:0 (if missing)",
                "ADAPTER_PID = dword:0 (if missing)",
            ]
        );
        // No dead values: cmr4.exe has none of these names.
        for dead in [
            "AUDIO_PATH",
            "CD_PATH",
            "VIDEO_PATH",
            "BitDepth",
            "MOVIE_QUALITY",
        ] {
            assert!(values.iter().all(|value| value.name != dead), "{dead}");
        }
    }

    #[test]
    fn the_players_old_values_are_carried_over_as_seeds() {
        let previous = Previous {
            size: Some((800, 600)),
            carried: vec![
                ("LANGUAGE", Data::String("F".into())),
                ("ADAPTER_VID", Data::Dword(0x10de)),
                ("SPEC_SELECTION2", Data::Dword(3)),
            ],
        };
        let values = values(Resolution::Original, Path::new(r"C:\CMR4"), &previous);
        assert_eq!(
            rendered(&values),
            [
                r#"INSTALL_PATH = string:"C:\\CMR4""#,
                "Width = dword:800",
                "Height = dword:600",
                r#"LANGUAGE = string:"F" (if missing)"#,
                "ADAPTER_VID = dword:4318 (if missing)",
                "ADAPTER_PID = dword:0 (if missing)",
                "SPEC_SELECTION2 = dword:3 (if missing)",
            ]
        );
        // A custom resolution still wins over the carried size.
        let custom = super::values(Resolution::Custom(1280, 960), Path::new("C:"), &previous);
        assert_eq!(custom[1].data, Data::Dword(1280));
    }

    #[test]
    fn the_redirect_sends_the_games_key_to_hkcu() {
        let config = redirect().unwrap();
        // The variable carries the rule as written, and it parses back.
        assert_eq!(config.env_value(), REDIRECT);
        // cmr4.exe spells it CODEMASTERS; the rule matches any case.
        let game = Key::parse(r"HKLM\SOFTWARE\CODEMASTERS\Colin McRae Rally 04").unwrap();
        let target = config.map(&game).unwrap();
        assert_eq!(target.key, Key::new(Root::CurrentUser, USER_KEY));
        assert!(!target.walk);
        assert_eq!(REDIRECT, format!(r"HKLM\{MACHINE_KEY} -> HKCU\{USER_KEY}"));
        let other = Key::parse(r"HKLM\SOFTWARE\Microsoft\DirectX").unwrap();
        assert_eq!(config.map(&other), None);
    }

    #[test]
    fn not_ready_names_each_problem_and_what_to_do() {
        let mismatches = [
            Mismatch {
                name: "INSTALL_PATH".into(),
                problem: Problem::Differs {
                    wanted: Data::String(r"C:\CMR4".into()),
                    found: Data::String(r"D:\old".into()),
                },
            },
            Mismatch {
                name: "Width".into(),
                problem: Problem::NotWritten(Error {
                    what: "cannot write registry value Width".into(),
                    code: 5,
                }),
            },
        ];
        let mut problems = vec!["rea_close_hook.dll is not in the components folder".to_owned()];
        problems.extend(mismatch_lines(&mismatches));
        let text = not_ready(&problems);
        assert!(text.starts_with("Not ready to play"), "{text}");
        assert!(text.contains("\n- rea_close_hook.dll is not in the components folder"));
        assert!(text.contains(
            r#"- HKCU\Software\Codemasters\Colin McRae Rally 04: INSTALL_PATH is "D:\old", not "C:\CMR4""#
        ));
        assert!(
            text.contains("Width can't be written: cannot write registry value Width (error 5)")
        );
        assert!(text.contains("no administrator rights needed"));
        assert!(text.contains(BUG_REPORT));
    }

    #[test]
    fn paths_use_the_32_bit_game_and_virtual_store_views() {
        use rea_launcher_win32::reg::{uses_32_bit_view, virtual_store_path};
        assert!(MACHINE_KEY.starts_with(r"SOFTWARE\Codemasters\"));
        assert!(uses_32_bit_view(true, MACHINE_KEY));
        // The VirtualStore key older launchers wrote on 64-bit Windows,
        // read once for the carry-over.
        assert_eq!(
            virtual_store_path(MACHINE_KEY, true),
            r"Software\Classes\VirtualStore\MACHINE\SOFTWARE\WOW6432Node\Codemasters\Colin McRae Rally 04"
        );
        // Wine's bus key opens without a view flag, as before.
        assert!(!uses_32_bit_view(true, rea_launcher::wine::WINEBUS_KEY));
        assert_eq!(WINEBUS_DEFAULTS, [("Map Controllers", 1)]);
    }
}
