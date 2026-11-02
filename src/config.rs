//! Command-line and commented `launcher.cfg` settings.

use std::path::PathBuf;

pub const FILE_NAME: &str = "launcher.cfg";
pub const DEFAULT_RESOLUTION: Resolution = Resolution::Custom(1920, 1080);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Resolution {
    Original,
    Custom(u32, u32),
}

impl Resolution {
    pub fn value(self) -> String {
        match self {
            Resolution::Original => "original".into(),
            Resolution::Custom(w, h) => format!("{w}x{h}"),
        }
    }

    pub fn is_exact_16_by_9(self) -> bool {
        match self {
            Resolution::Custom(width, height) => {
                height > 0 && u64::from(width) * 9 == u64::from(height) * 16
            }
            Resolution::Original => false,
        }
    }
}

/// Largest exact 16:9 mode that fits the active screen in landscape, capped
/// at 4K and with a 640x360 minimum. Unknown screens use 1920x1080.
pub fn default_resolution(screen: Option<(u32, u32)>) -> Resolution {
    let Some((a, b)) = screen else {
        return DEFAULT_RESOLUTION;
    };
    let (screen_w, screen_h) = (a.max(b).min(3840), a.min(b).min(2160));
    let mut height = screen_h.min(2160);
    while height >= 360 {
        if height % 9 == 0 {
            let width = height / 9 * 16;
            if width <= screen_w && width >= 640 {
                return Resolution::Custom(width, height);
            }
        }
        height -= 1;
    }
    Resolution::Custom(640, 360)
}

pub fn validate_picture(settings: &Settings) -> Result<(), String> {
    if settings.widescreen() && !settings.resolution().is_exact_16_by_9() {
        return Err(format!(
            "the 16:9 Low widescreen patch requires an exact 16:9 custom resolution (current: {}); choose 16:9 or turn widescreen off",
            settings.resolution().value()
        ));
    }
    Ok(())
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Settings {
    pub game: Option<PathBuf>,
    pub runtime: Option<PathBuf>,
    pub resolution: Option<Resolution>,
    pub widescreen: Option<bool>,
    pub fps_limit: Option<u32>,
    pub vsync: Option<bool>,
    pub controller: Option<bool>,
}

impl Settings {
    pub fn resolution(&self) -> Resolution {
        self.resolution.unwrap_or(DEFAULT_RESOLUTION)
    }

    pub fn widescreen(&self) -> bool {
        self.widescreen.unwrap_or(true)
    }

    pub fn fps_limit(&self) -> Option<u32> {
        self.fps_limit.filter(|&fps| fps > 0)
    }

    pub fn vsync(&self) -> bool {
        self.vsync.unwrap_or(false)
    }

    pub fn controller(&self) -> bool {
        self.controller.unwrap_or(true)
    }

    pub fn merged(self, over: Settings) -> Settings {
        Settings {
            game: over.game.or(self.game),
            runtime: over.runtime.or(self.runtime),
            resolution: over.resolution.or(self.resolution),
            widescreen: over.widescreen.or(self.widescreen),
            fps_limit: over.fps_limit.or(self.fps_limit),
            vsync: over.vsync.or(self.vsync),
            controller: over.controller.or(self.controller),
        }
    }

    pub fn parse_file(text: &str) -> (Settings, Vec<String>) {
        let mut settings = Settings::default();
        let mut warnings = Vec::new();
        for (line_number, line) in text.lines().enumerate() {
            let line = line.trim();
            if line.starts_with('#') {
                continue;
            }
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            let key = key.trim();
            let value = value.trim();
            if value.is_empty() {
                continue;
            }
            let mut bad = |expected: &str| {
                warnings.push(format!(
                    "{FILE_NAME} line {}: expected {expected}, not {value:?}",
                    line_number + 1
                ));
            };
            match key {
                "game" => settings.game = Some(value.into()),
                "runtime" => settings.runtime = Some(value.into()),
                "resolution" => match parse_resolution(value) {
                    Some(value) => settings.resolution = Some(value),
                    None => bad("WIDTHxHEIGHT or original"),
                },
                "fps_limit" => match parse_fps(value) {
                    Some(value) => settings.fps_limit = Some(value),
                    None => bad("1 to 1000 or off"),
                },
                "widescreen" | "vsync" | "controller" => match parse_bool(value) {
                    Some(value) => match key {
                        "widescreen" => settings.widescreen = Some(value),
                        "vsync" => settings.vsync = Some(value),
                        _ => settings.controller = Some(value),
                    },
                    None => bad("on or off"),
                },
                _ => {}
            }
        }
        (settings, warnings)
    }

    /// Writes every launcher-owned value, with comments suitable for hand editing.
    pub fn render(&self) -> String {
        let game = self
            .game
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_default();
        let runtime = self
            .runtime
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_default();
        let fps = self
            .fps_limit()
            .map_or_else(|| "off".into(), |n| n.to_string());
        format!(
            "# Colin McRae Rally 04 launcher settings.\n\
             # One setting per line as key = value; # starts a comment.\n\
             # Command-line options override these values and are saved.\n\n\
             # The installed game folder. It is only read, never modified.\n\
             game = {game}\n\n\
             # The private writable runtime, including saves and settings.\n\
             runtime = {runtime}\n\n\
             # Picture size: WIDTHxHEIGHT, or original to keep the game's mode.\n\
             resolution = {}\n\n\
             # Apply the tested 16:9 Low aspect patch to the private cmr4.exe.\n\
             widescreen = {}\n\n\
             # dgVoodoo frame-rate cap: 1 to 1000, or off.\n\
             fps_limit = {fps}\n\n\
             # Force vertical sync in dgVoodoo.\n\
             vsync = {}\n\n\
             # Xidi gamepad: stick steers, RT/LT drive the pedal (Y) axis.\n\
             controller = {}\n",
            self.resolution().value(),
            on_off(self.widescreen()),
            on_off(self.vsync()),
            on_off(self.controller()),
        )
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Cli {
    pub settings: Settings,
    pub check: bool,
    pub no_window: bool,
    pub help: bool,
    pub settings_window: bool,
    pub linux: bool,
}

pub const USAGE: &str = concat!(
    "CMR4Launcher ",
    env!("CARGO_PKG_VERSION"),
    "\n\n",
    "CMR4Launcher [options]\n\n",
    "  --game <folder>       folder containing the installed cmr4.exe\n",
    "  --runtime <folder>    private writable runtime folder\n",
    "  --resolution WxH      exact 16:9 for widescreen, or original (default: screen fit)\n",
    "  --widescreen on|off   tested 16:9 Low patch (default on; off for other aspects)\n",
    "  --fps-limit N|off     dgVoodoo frame-rate cap\n",
    "  --vsync on|off        force vertical sync (default off)\n",
    "  --controller on|off   Xidi gamepad: stick steers, RT/LT are the pedals (default on)\n",
    "  --check               validate the game and patch and save settings; copy nothing\n",
    "  --no-window           run without a settings window\n",
    "  --help                show this help\n\n",
    "The original game folder is never modified. Settings are saved in launcher.cfg."
);

fn on_off(value: bool) -> &'static str {
    if value { "on" } else { "off" }
}

fn parse_bool(value: &str) -> Option<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "on" | "true" | "yes" | "1" => Some(true),
        "off" | "false" | "no" | "0" => Some(false),
        _ => None,
    }
}

fn parse_resolution(value: &str) -> Option<Resolution> {
    let value = value.trim().to_ascii_lowercase();
    if value == "original" {
        return Some(Resolution::Original);
    }
    let (width, height) = value.split_once('x')?;
    let width: u32 = width.trim().parse().ok()?;
    let height: u32 = height.trim().parse().ok()?;
    ((320..=7680).contains(&width) && (240..=4320).contains(&height))
        .then_some(Resolution::Custom(width, height))
}

fn parse_fps(value: &str) -> Option<u32> {
    let value = value.trim();
    if value.eq_ignore_ascii_case("off") || value == "0" {
        return Some(0);
    }
    let fps: u32 = value.parse().ok()?;
    (1..=1000).contains(&fps).then_some(fps)
}

pub fn parse_args<I: IntoIterator<Item = String>>(args: I) -> Result<Cli, String> {
    let mut cli = Cli::default();
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        let mut value = |name: &str| {
            args.next()
                .ok_or_else(|| format!("{name} needs a value\n\n{USAGE}"))
        };
        match arg.as_str() {
            "--game" => cli.settings.game = Some(value("--game")?.into()),
            "--runtime" => cli.settings.runtime = Some(value("--runtime")?.into()),
            "--resolution" => {
                let raw = value("--resolution")?;
                cli.settings.resolution = Some(parse_resolution(&raw).ok_or_else(|| {
                    format!("--resolution needs WIDTHxHEIGHT or original, not {raw}\n\n{USAGE}")
                })?);
            }
            "--fps-limit" => {
                let raw = value("--fps-limit")?;
                cli.settings.fps_limit = Some(parse_fps(&raw).ok_or_else(|| {
                    format!("--fps-limit needs 1 to 1000 or off, not {raw}\n\n{USAGE}")
                })?);
            }
            "--check" => cli.check = true,
            "--no-window" => cli.no_window = true,
            "--settings" => cli.settings_window = true,
            "--linux" => cli.linux = true,
            "--help" | "-h" | "/?" => cli.help = true,
            flag @ ("--widescreen" | "--vsync" | "--controller") => {
                let raw = value(flag)?;
                let enabled = parse_bool(&raw)
                    .ok_or_else(|| format!("{flag} needs on or off, not {raw}\n\n{USAGE}"))?;
                match flag {
                    "--widescreen" => cli.settings.widescreen = Some(enabled),
                    "--vsync" => cli.settings.vsync = Some(enabled),
                    _ => cli.settings.controller = Some(enabled),
                }
            }
            other => return Err(format!("unknown argument {other}\n\n{USAGE}")),
        }
    }
    Ok(cli)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(values: &[&str]) -> Result<Cli, String> {
        parse_args(values.iter().map(|value| (*value).to_string()))
    }

    #[test]
    fn defaults_match_the_tested_modern_setup() {
        let settings = Settings::default();
        assert_eq!(settings.resolution(), Resolution::Custom(1920, 1080));
        assert!(settings.widescreen());
        assert!(settings.controller());
        assert_eq!(settings.fps_limit(), None);
        assert!(!settings.vsync());
    }

    #[test]
    fn default_resolution_fits_exact_16_by_9_to_the_screen() {
        for (screen, expected) in [
            (Some((3840, 2160)), (3840, 2160)),
            (Some((1920, 1080)), (1920, 1080)),
            (Some((2560, 1080)), (1920, 1080)),
            (Some((1280, 800)), (1280, 720)),
            (Some((2160, 3840)), (3840, 2160)),
            (Some((1366, 768)), (1360, 765)),
            (Some((640, 480)), (640, 360)),
        ] {
            assert_eq!(
                default_resolution(screen),
                Resolution::Custom(expected.0, expected.1),
                "{screen:?}"
            );
        }
        assert_eq!(default_resolution(None), Resolution::Custom(1920, 1080));
    }

    #[test]
    fn widescreen_rejects_non_16_by_9_but_4_by_3_can_be_unpatched() {
        let wide = Settings {
            resolution: Some(Resolution::Custom(2560, 1080)),
            ..Settings::default()
        };
        assert!(validate_picture(&wide).unwrap_err().contains("exact 16:9"));
        let original = Settings {
            resolution: Some(Resolution::Original),
            ..Settings::default()
        };
        assert!(validate_picture(&original).is_err());
        let four_three = Settings {
            resolution: Some(Resolution::Custom(1600, 1200)),
            widescreen: Some(false),
            ..Settings::default()
        };
        assert_eq!(validate_picture(&four_three), Ok(()));
    }

    #[test]
    fn parses_all_cli_options_and_control_flags() {
        let cli = args(&[
            "--game",
            r"D:\Games\CMR4",
            "--runtime",
            r"D:\Runtime",
            "--resolution",
            "1920X1080",
            "--widescreen",
            "off",
            "--fps-limit",
            "120",
            "--vsync",
            "on",
            "--controller",
            "off",
            "--check",
            "--no-window",
        ])
        .unwrap();
        assert_eq!(
            cli.settings.resolution,
            Some(Resolution::Custom(1920, 1080))
        );
        assert_eq!(cli.settings.widescreen, Some(false));
        assert_eq!(cli.settings.fps_limit, Some(120));
        assert_eq!(cli.settings.vsync, Some(true));
        assert_eq!(cli.settings.controller, Some(false));
        assert!(cli.check && cli.no_window && !cli.help);
        assert!(args(&["--help"]).unwrap().help);
        assert!(args(&["--resolution", "original"]).is_ok());
        assert!(args(&["--fps-limit", "off"]).is_ok());
        assert!(args(&["--resolution", "1x2"]).is_err());
        assert!(args(&["--widescreen", "perhaps"]).is_err());
        assert!(args(&["--game"]).is_err());
        assert!(args(&["--bogus"]).is_err());
    }

    #[test]
    fn command_line_overrides_file_and_original_is_not_empty() {
        let (saved, warnings) = Settings::parse_file(
            "resolution = 1920x1080\nwidescreen = off\ncontroller = off\nfps_limit = 60\n",
        );
        assert!(warnings.is_empty());
        let over = args(&["--resolution", "original", "--widescreen", "on"])
            .unwrap()
            .settings;
        let merged = saved.merged(over);
        assert_eq!(merged.resolution(), Resolution::Original);
        assert!(merged.widescreen());
        assert!(!merged.controller());
        assert_eq!(merged.fps_limit(), Some(60));
    }

    #[test]
    fn rendered_file_is_commented_and_round_trips() {
        let settings = Settings {
            game: Some(PathBuf::from(r"C:\Games\CMR 04")),
            runtime: Some(PathBuf::from(r"C:\Runtime")),
            ..Settings::default()
        };
        let text = settings.render();
        assert!(text.contains("# The installed game folder"));
        let (read, warnings) = Settings::parse_file(&text);
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(read.game, settings.game);
        assert_eq!(read.runtime, settings.runtime);
        assert_eq!(read.resolution(), DEFAULT_RESOLUTION);
        assert!(read.widescreen());
        assert!(read.controller());
    }

    #[test]
    fn hidden_helper_flags_and_help_aliases_parse() {
        let cli = args(&["--settings", "--linux"]).unwrap();
        assert!(cli.settings_window && cli.linux && !cli.no_window && !cli.check);
        assert_eq!(cli.settings, Settings::default());
        for help in ["--help", "-h", "/?"] {
            assert!(args(&[help]).unwrap().help, "{help}");
        }
        for flag in [
            "--game",
            "--runtime",
            "--resolution",
            "--widescreen",
            "--fps-limit",
            "--vsync",
            "--controller",
            "--check",
            "--no-window",
            "--help",
        ] {
            assert!(USAGE.contains(flag), "{flag} missing from USAGE");
        }
    }

    #[test]
    fn cli_off_values_override_saved_on_values() {
        let (saved, warnings) =
            Settings::parse_file("fps_limit = 60\nvsync = on\ncontroller = on\nwidescreen = on\n");
        assert!(warnings.is_empty());
        let over = args(&[
            "--fps-limit",
            "off",
            "--vsync",
            "off",
            "--controller",
            "off",
            "--widescreen",
            "off",
        ])
        .unwrap()
        .settings;
        let merged = saved.clone().merged(over);
        assert_eq!(merged.fps_limit(), None);
        assert!(!merged.vsync() && !merged.controller() && !merged.widescreen());
        // Absent CLI values keep the saved ones.
        let kept = saved.clone().merged(Settings::default());
        assert_eq!(kept, saved);
    }

    #[test]
    fn file_and_cli_accept_the_same_value_spellings() {
        for (raw, expected) in [
            ("on", true),
            ("TRUE", true),
            ("yes", true),
            ("1", true),
            ("off", false),
            ("False", false),
            ("no", false),
            ("0", false),
        ] {
            let (file, warnings) = Settings::parse_file(&format!("vsync = {raw}\n"));
            assert!(warnings.is_empty(), "{raw}");
            assert_eq!(file.vsync, Some(expected), "{raw}");
            assert_eq!(args(&["--vsync", raw]).unwrap().settings.vsync, file.vsync);
        }
        for (raw, expected) in [("off", Some(0)), ("0", Some(0)), ("1000", Some(1000))] {
            let (file, _) = Settings::parse_file(&format!("fps_limit = {raw}\n"));
            assert_eq!(file.fps_limit, expected, "{raw}");
            assert_eq!(
                args(&["--fps-limit", raw]).unwrap().settings.fps_limit,
                expected
            );
        }
        for raw in ["1001", "-5", "fast"] {
            assert!(
                Settings::parse_file(&format!("fps_limit = {raw}\n"))
                    .1
                    .len()
                    == 1
            );
            assert!(args(&["--fps-limit", raw]).is_err(), "{raw}");
        }
        for raw in ["319x240", "320x239", "7681x4320", "1920*1080", "x1080"] {
            assert!(
                Settings::parse_file(&format!("resolution = {raw}\n"))
                    .1
                    .len()
                    == 1
            );
            assert!(args(&["--resolution", raw]).is_err(), "{raw}");
        }
        let (file, _) = Settings::parse_file("resolution = 1920X1080\n");
        assert_eq!(file.resolution, Some(Resolution::Custom(1920, 1080)));
    }

    #[test]
    fn file_ignores_comments_blank_values_and_keeps_the_last_value() {
        let (settings, warnings) = Settings::parse_file(
            "# vsync = on\n  # resolution = huge\nvsync =\nfps_limit = 30\nfps_limit = 120\n",
        );
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(settings.vsync, None);
        assert_eq!(settings.fps_limit, Some(120));
    }

    #[test]
    fn every_non_default_value_renders_and_reads_back() {
        let settings = Settings {
            game: Some(PathBuf::from(r"C:\Games\CMR 04")),
            runtime: Some(PathBuf::from(r"D:\Runtime")),
            resolution: Some(Resolution::Original),
            widescreen: Some(false),
            fps_limit: Some(144),
            vsync: Some(true),
            controller: Some(false),
        };
        let (read, warnings) = Settings::parse_file(&settings.render());
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(read, settings);
        let uncapped = Settings {
            fps_limit: Some(0),
            ..Settings::default()
        };
        let text = uncapped.render();
        assert!(text.contains("fps_limit = off\n"));
        assert_eq!(Settings::parse_file(&text).0.fps_limit(), None);
    }

    #[test]
    fn bad_file_values_warn_without_losing_other_settings() {
        let (settings, warnings) = Settings::parse_file(
            "game = C:\\CMR4\nresolution = huge\nvsync = perhaps\nfuture = ignored\n",
        );
        assert!(settings.game.is_some());
        assert_eq!(settings.resolution, None);
        assert_eq!(warnings.len(), 2);
    }
}
