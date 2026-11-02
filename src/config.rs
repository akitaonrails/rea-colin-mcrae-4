//! Command-line and commented `launcher.cfg` settings.
//!
//! Reading, line editing and value parsing are `rea_launcher::cfg`; the
//! keys, defaults, template and what a launch remembers are CMR4's.

use std::path::{Path, PathBuf};

use rea_launcher::cfg::{self, ArgError, Rejected, need, on_off, parse_bool};
pub use rea_launcher::cfg::{clear_key, read_file, set_key};
use rea_launcher::display;

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
    let (width, height) = display::largest_16_9(screen, (3840, 2160), (1920, 1080));
    Resolution::Custom(width, height)
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

    /// Takes one `key = value` from the file or a `--key value` flag.
    fn apply(&mut self, key: &str, value: &str) -> Result<(), Rejected> {
        match key {
            "game" => self.game = Some(value.into()),
            "runtime" => self.runtime = Some(value.into()),
            "resolution" => {
                self.resolution = Some(need(parse_resolution(value), RESOLUTION_NEEDS)?);
            }
            "fps_limit" => self.fps_limit = Some(need(parse_fps(value), FPS_NEEDS)?),
            "widescreen" => self.widescreen = Some(need(parse_bool(value), cfg::ON_OFF)?),
            "vsync" => self.vsync = Some(need(parse_bool(value), cfg::ON_OFF)?),
            "controller" => self.controller = Some(need(parse_bool(value), cfg::ON_OFF)?),
            _ => return Err(Rejected::Unknown),
        }
        Ok(())
    }

    pub fn parse_file(text: &str) -> (Settings, Vec<String>) {
        let mut settings = Settings::default();
        let warnings = cfg::parse(text, FILE_NAME, |key, value| settings.apply(key, value));
        (settings, warnings)
    }

    /// The file `text` with these values written in. Every other line, the
    /// player's comments and keys this launcher doesn't know included, is
    /// kept; a value that is `None` is commented out so its default applies.
    /// An empty or missing file starts from the commented `TEMPLATE`.
    pub fn save_into(&self, text: &str) -> String {
        let path = |path: &Option<PathBuf>| path.as_ref().map(|path| path.display().to_string());
        let mut out = if text.trim().is_empty() {
            TEMPLATE.to_string()
        } else {
            refresh_header(text)
        };
        for (key, value) in [
            ("game", path(&self.game)),
            ("runtime", path(&self.runtime)),
            ("resolution", self.resolution.map(Resolution::value)),
            ("widescreen", self.widescreen.map(|v| on_off(v).into())),
            (
                "fps_limit",
                self.fps_limit
                    .map(|n| if n == 0 { "off".into() } else { n.to_string() }),
            ),
            ("vsync", self.vsync.map(|v| on_off(v).into())),
            ("controller", self.controller.map(|v| on_off(v).into())),
        ] {
            out = match value {
                Some(value) => set_key(&out, key, &value),
                None => clear_key(&out, key),
            };
        }
        out
    }
}

/// Whether a launch should write its game folder into the settings file:
/// when the file has none yet, or `--game` named a different one. No other
/// command-line value is ever saved (the family rule: flags are one-run
/// overrides).
pub fn should_remember_game(saved: Option<&Path>, cli: Option<&Path>) -> bool {
    saved.is_none() || cli.is_some_and(|cli| saved != Some(cli))
}

/// What a launch writes back into the settings file `text`, keeping every
/// other line; None when nothing changes. Two values are remembered:
/// - the game folder, as `should_remember_game` decides;
/// - `picked`, the resolution the launcher picked from the screen, when
///   neither the file nor a `--resolution` flag set one. The caller passes
///   None inside gamescope, whose output is only `cmr4-play`'s 1024x768
///   fallback and would be saved for good. A flag's value is never saved.
pub fn remember(
    text: &str,
    saved: &Settings,
    cli: &Settings,
    game: &Path,
    picked: Option<Resolution>,
) -> Option<String> {
    let game_changed = should_remember_game(saved.game.as_deref(), cli.game.as_deref());
    let auto_resolution = picked.filter(|_| saved.resolution.is_none() && cli.resolution.is_none());
    if !game_changed && auto_resolution.is_none() {
        return None;
    }
    let mut out = if text.trim().is_empty() {
        TEMPLATE.to_string()
    } else {
        refresh_header(text)
    };
    if game_changed {
        out = set_key(&out, "game", &game.display().to_string());
    }
    if let Some(picked) = auto_resolution {
        out = set_key(&out, "resolution", &picked.value());
    }
    Some(out)
}

/// A new `launcher.cfg`: every key with its comment, unset (defaults apply).
pub const TEMPLATE: &str = "\
# Colin McRae Rally 04 launcher settings.
# One setting per line as key = value; # starts a comment. An empty value
# uses the default. Command-line options override these values for one run;
# only --game is remembered here. An unset resolution is filled in with the
# one the launcher picks from the screen.

# The installed game folder. It is only read, never modified.
game =

# The private writable runtime, including saves and settings.
# Default: %LOCALAPPDATA%\\CMR4Launcher\\CMR4
runtime =

# Picture size: WIDTHxHEIGHT, or original to keep the game's mode.
# Default: the largest exact 16:9 that fits the screen.
resolution =

# Apply the tested 16:9 Low aspect patch to the private cmr4.exe (on/off).
widescreen =

# dgVoodoo frame-rate cap: 1 to 1000, or off.
fps_limit =

# Force vertical sync in dgVoodoo (on/off).
vsync =

# Xidi gamepad: stick steers, RT/LT drive the pedal (Y) axis (on/off).
controller =
";

/// v0.1.0 and v0.2.0 wrote this header line; it is no longer true.
const STALE_HEADER: &str = "# Command-line options override these values and are saved.";
const HEADER: &str =
    "# Command-line options override these values for one run; only --game is remembered.";

/// Replaces the stale v0.2.0 header claim, if present; nothing else changes.
fn refresh_header(text: &str) -> String {
    text.replacen(STALE_HEADER, HEADER, 1)
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
    "  --check               validate the game and patch, remember --game; copy nothing\n",
    "  --no-window           run without a settings window\n",
    "  --help                show this help\n\n",
    "The original game folder is never modified. Options override launcher.cfg for one\nrun; only --game is remembered there."
);

/// What `resolution` takes, for messages.
const RESOLUTION_NEEDS: &str = "WIDTHxHEIGHT or original";
/// What `fps_limit` takes, for messages.
const FPS_NEEDS: &str = "1 to 1000 or off";
/// The settings keys, which are also the `--flag` names.
const KEYS: [&str; 7] = [
    "game",
    "runtime",
    "resolution",
    "widescreen",
    "fps_limit",
    "vsync",
    "controller",
];
/// The launcher's own flags that take no value.
const SWITCHES: [&str; 4] = ["--check", "--no-window", "--settings", "--linux"];

fn parse_resolution(value: &str) -> Option<Resolution> {
    match cfg::parse_resolution(value, (320, 240), (7680, 4320), cfg::ORIGINAL)? {
        cfg::Resolution::Size(width, height) => Some(Resolution::Custom(width, height)),
        _ => Some(Resolution::Original),
    }
}

fn parse_fps(value: &str) -> Option<u32> {
    if value.trim().eq_ignore_ascii_case("off") {
        return Some(0);
    }
    cfg::parse_range(value, 0..=1000)
}

pub fn parse_args<I: IntoIterator<Item = String>>(args: I) -> Result<Cli, String> {
    let mut cli = Cli::default();
    let mut settings = Settings::default();
    match cfg::parse_args(args, &KEYS, &SWITCHES, |key, value| {
        settings.apply(key, value)
    }) {
        Ok(switches) => {
            for switch in switches {
                match switch.as_str() {
                    "--check" => cli.check = true,
                    "--no-window" => cli.no_window = true,
                    "--settings" => cli.settings_window = true,
                    _ => cli.linux = true,
                }
            }
        }
        Err(ArgError::Help) => cli.help = true,
        Err(error) => return Err(format!("{error}\n\n{USAGE}")),
    }
    cli.settings = settings;
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
        let text = settings.save_into("");
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
        let (read, warnings) = Settings::parse_file(&settings.save_into(""));
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(read, settings);
        let uncapped = Settings {
            fps_limit: Some(0),
            ..Settings::default()
        };
        let text = uncapped.save_into("");
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
        assert_eq!(warnings.len(), 3);
        assert!(warnings[2].contains("line 4: unknown setting \"future\""));
    }

    #[test]
    fn a_command_line_run_remembers_only_the_game_folder() {
        let file = "# mine\nresolution = 1920x1080\nvsync = on\n";
        let cli = args(&[
            "--game",
            r"D:\CMR4",
            "--resolution",
            "original",
            "--vsync",
            "off",
        ])
        .unwrap()
        .settings;
        let (saved, _) = Settings::parse_file(file);
        let game = Path::new(r"D:\CMR4");
        let text = remember(file, &saved, &cli, game, Some(Resolution::Original)).unwrap();
        assert_eq!(
            text,
            "# mine\nresolution = 1920x1080\nvsync = on\ngame = D:\\CMR4\n"
        );
        let (read, _) = Settings::parse_file(&text);
        assert_eq!(read.resolution, Some(Resolution::Custom(1920, 1080)));
        assert_eq!(read.vsync, Some(true));
        // Remembered already: no rewrite, with or without the same --game.
        let same = Settings {
            game: read.game.clone(),
            ..Settings::default()
        };
        let picked = Some(Resolution::Custom(3840, 2160));
        assert_eq!(
            remember(&text, &read, &Settings::default(), game, picked),
            None
        );
        assert_eq!(remember(&text, &read, &same, game, picked), None);
        assert!(should_remember_game(
            read.game.as_deref(),
            Some(Path::new(r"E:\CMR4"))
        ));
    }

    #[test]
    fn a_resolution_picked_from_the_screen_is_saved_but_a_flag_is_not() {
        let game = Path::new(r"D:\CMR4");
        let file = "game = D:\\CMR4\n# my comment\n";
        let (saved, _) = Settings::parse_file(file);
        let picked = Some(Resolution::Custom(2560, 1440));
        // Auto-picked: the file had none and no flag set one.
        let text = remember(file, &saved, &Settings::default(), game, picked).unwrap();
        assert_eq!(
            text,
            "game = D:\\CMR4\n# my comment\nresolution = 2560x1440\n"
        );
        // --resolution: a one-run override, never saved.
        let flag = args(&["--resolution", "1920x1080"]).unwrap().settings;
        assert_eq!(
            remember(
                file,
                &saved,
                &flag,
                game,
                Some(Resolution::Custom(1920, 1080))
            ),
            None
        );
        // A first launch with a flag remembers only the game folder.
        let fresh = remember(
            "",
            &Settings::default(),
            &flag,
            game,
            Some(Resolution::Custom(1920, 1080)),
        )
        .unwrap();
        assert!(fresh.starts_with("# Colin McRae Rally 04 launcher settings."));
        let (read, _) = Settings::parse_file(&fresh);
        assert_eq!(read.game.as_deref(), Some(game));
        assert_eq!(read.resolution, None);
        // A first launch without flags also keeps the picked resolution.
        let auto = remember("", &Settings::default(), &Settings::default(), game, picked).unwrap();
        assert_eq!(Settings::parse_file(&auto).0.resolution, picked);
        // Inside gamescope (picked = None) only the game folder is saved:
        // its output is cmr4-play's fallback, not the player's screen.
        assert_eq!(
            remember(file, &saved, &Settings::default(), game, None),
            None
        );
        let nested = remember("", &Settings::default(), &Settings::default(), game, None).unwrap();
        assert_eq!(Settings::parse_file(&nested).0.resolution, None);
    }

    #[test]
    fn settings_files_in_utf8_or_ansi_read_and_only_missing_is_a_first_run() {
        let dir = std::env::temp_dir().join(format!("cmr4-config-read-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(FILE_NAME);
        assert!(read_file(&path).unwrap().is_none());
        // Notepad ANSI (Windows-1252): "C:\Jogos\Colin McRae Rally 04 Edição".
        let mut ansi = b"game = C:\\Jogos\\Edi".to_vec();
        ansi.extend([0xE7, 0xE3]);
        ansi.extend(b"o\r\n");
        std::fs::write(&path, &ansi).unwrap();
        let text = read_file(&path).unwrap().unwrap();
        assert_eq!(
            Settings::parse_file(&text).0.game,
            Some(PathBuf::from("C:\\Jogos\\Edição"))
        );
        std::fs::write(&path, "game = D:\\Edição\n").unwrap();
        let text = read_file(&path).unwrap().unwrap();
        assert_eq!(
            Settings::parse_file(&text).0.game,
            Some(PathBuf::from("D:\\Edição"))
        );
        // Not a file: a real error, so the caller won't overwrite it.
        assert!(read_file(&dir).is_err());
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn saving_a_v0_2_0_file_drops_its_stale_header_claim() {
        let old = "# Colin McRae Rally 04 launcher settings.\r\n# One setting per line as key = value; # starts a comment.\r\n# Command-line options override these values and are saved.\r\n\r\ngame = C:\\CMR4\r\nvsync = on\r\n";
        let window = Settings {
            vsync: Some(false),
            ..Settings::default()
        }
        .save_into(old);
        assert_eq!(
            window,
            "# Colin McRae Rally 04 launcher settings.\r\n# One setting per line as key = value; # starts a comment.\r\n# Command-line options override these values for one run; only --game is remembered.\r\n\r\n#game =\r\nvsync = off\r\n"
        );
        let (saved, _) = Settings::parse_file(old);
        let cli = Settings {
            game: Some(PathBuf::from(r"D:\CMR4")),
            ..Settings::default()
        };
        let remembered = remember(old, &saved, &cli, Path::new(r"D:\CMR4"), None).unwrap();
        assert!(!remembered.contains(STALE_HEADER));
        assert!(remembered.contains(HEADER));
        // A file without the old line is left alone.
        assert_eq!(refresh_header("# mine\n"), "# mine\n");
    }

    #[test]
    fn template_parses_clean_to_the_defaults() {
        let (settings, warnings) = Settings::parse_file(TEMPLATE);
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(settings, Settings::default());
        for key in [
            "game",
            "runtime",
            "resolution",
            "widescreen",
            "fps_limit",
            "vsync",
            "controller",
        ] {
            assert!(TEMPLATE.contains(&format!("\n{key} =\n")), "{key}");
        }
    }

    #[test]
    fn saving_keeps_comments_unknown_keys_and_line_order() {
        let mine = "# my notes\r\nvsync = on # tear-free\r\nmod_path = C:\\mods\r\n# resolution = 640x480\r\nresolution = 1920x1080\r\n";
        let saved = Settings {
            resolution: Some(Resolution::Custom(3840, 2160)),
            vsync: Some(false),
            ..Settings::default()
        }
        .save_into(mine);
        assert_eq!(
            saved,
            // Unset keys absent from the file stay absent.
            "# my notes\r\nvsync = off\r\nmod_path = C:\\mods\r\n# resolution = 640x480\r\nresolution = 3840x2160\r\n"
        );
        let (read, warnings) = Settings::parse_file(&saved);
        assert_eq!(warnings.len(), 1, "{warnings:?}");
        assert_eq!(read.resolution, Some(Resolution::Custom(3840, 2160)));
        assert_eq!(read.vsync, Some(false));
    }

    #[test]
    fn set_key_replaces_every_active_line_and_clear_key_comments_it_out() {
        let filled = set_key(TEMPLATE, "game", r"D:\CMR4");
        assert!(filled.contains("\ngame = D:\\CMR4\n"));
        assert!(filled.contains("\nruntime =\n"));
        assert_eq!(
            set_key(&filled, "game", r"E:\CMR4")
                .matches("game =")
                .count(),
            1
        );
        // Duplicates all take the new value, so the last one read agrees.
        let twice = set_key("vsync = on\nvsync = on\n", "vsync", "off");
        assert_eq!(twice, "vsync = off\nvsync = off\n");
        assert_eq!(set_key("a = 1", "b", "2"), "a = 1\nb = 2\n");
        let cleared = clear_key(&filled, "game");
        assert!(cleared.contains("\n#game =\n"));
        assert_eq!(Settings::parse_file(&cleared).0.game, None);
    }
}
