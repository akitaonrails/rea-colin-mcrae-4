//! Command-line and commented `launcher.cfg` settings.

use std::path::PathBuf;

pub const FILE_NAME: &str = "launcher.cfg";
pub const DEFAULT_RESOLUTION: Resolution = Resolution::Custom(3840, 2160);

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
             # Xidi modern controller to keyboard mapping.\n\
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
    "  --resolution WxH      picture size, or original (default 3840x2160)\n",
    "  --widescreen on|off   tested 16:9 Low patch (default on)\n",
    "  --fps-limit N|off     dgVoodoo frame-rate cap\n",
    "  --vsync on|off        force vertical sync (default off)\n",
    "  --controller on|off   Xidi modern controller mapping (default on)\n",
    "  --check               validate game and patch only; copy nothing\n",
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
        assert_eq!(settings.resolution(), Resolution::Custom(3840, 2160));
        assert!(settings.widescreen());
        assert!(settings.controller());
        assert_eq!(settings.fps_limit(), None);
        assert!(!settings.vsync());
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
    fn bad_file_values_warn_without_losing_other_settings() {
        let (settings, warnings) = Settings::parse_file(
            "game = C:\\CMR4\nresolution = huge\nvsync = perhaps\nfuture = ignored\n",
        );
        assert!(settings.game.is_some());
        assert_eq!(settings.resolution, None);
        assert_eq!(warnings.len(), 2);
    }
}
