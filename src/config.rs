//! Command-line and commented `launcher.cfg` settings.
//!
//! Every option is declared once in [`OPTIONS`] (`rea_launcher::cfg::spec`):
//! the file and flag parsing, `--help`, the first-run template and the
//! window's rows come from it. The keys, defaults, looks and what a launch
//! remembers are CMR4's.

use std::path::{Path, PathBuf};

use rea_launcher::cfg::{
    self, ArgError, KeyStyle, Kind, OptionSpec, Rejected, Value, on_off, spec,
};
pub use rea_launcher::cfg::{clear_key, read_file, set_key};
use rea_launcher::display;
use rea_launcher_win32::report::{Audience, Run};

pub const FILE_NAME: &str = "launcher.cfg";
pub const DEFAULT_RESOLUTION: Resolution = Resolution::Custom(1920, 1080);
/// The Steam Deck's own screen, landscape.
pub const STEAM_DECK_SCREEN: (u32, u32) = display::STEAM_DECK_SCREEN;

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

/// Picture presets: the window's look cards and `--look`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Look {
    /// The game's own resolution and view.
    Original,
    /// The largest exact 16:9 that fits the screen (up to 3840x2160), so
    /// the widescreen fix applies.
    Widescreen,
    /// Native 1280x800 (16:10: the game's own view, correct proportions)
    /// at a 60 fps cap.
    SteamDeck,
    /// Whatever the Picture tab says (the window only; never a value).
    Custom,
}

impl Look {
    pub const ALL: [Look; 4] = [
        Look::Original,
        Look::Widescreen,
        Look::SteamDeck,
        Look::Custom,
    ];

    /// The `--look`/`look =` value.
    pub fn value(self) -> &'static str {
        match self {
            Look::Original => "original",
            Look::Widescreen => "widescreen",
            Look::SteamDeck => "steam_deck",
            Look::Custom => "custom",
        }
    }

    fn from_value(value: &str) -> Option<Look> {
        Look::ALL.into_iter().find(|look| look.value() == value)
    }

    /// The resolution this look sets on `screen` (None: Custom).
    pub fn resolution(self, screen: Option<(u32, u32)>) -> Option<Resolution> {
        match self {
            Look::Original => Some(Resolution::Original),
            Look::Widescreen => Some(default_resolution(screen)),
            Look::SteamDeck => Some(Resolution::Custom(STEAM_DECK_SCREEN.0, STEAM_DECK_SCREEN.1)),
            Look::Custom => None,
        }
    }

    /// The frame-rate cap this look sets (None: left as it is).
    pub fn fps_limit(self) -> Option<u32> {
        (self == Look::SteamDeck).then_some(60)
    }
}

/// The look to recommend: Steam Deck on a 1280x800 screen (either
/// orientation), otherwise Widescreen.
pub fn recommended(screen: Option<(u32, u32)>) -> Look {
    if display::is_steam_deck(screen) {
        Look::SteamDeck
    } else {
        Look::Widescreen
    }
}

const LOOKS: &[(&str, &str)] = &[
    ("original", "Original"),
    ("widescreen", "Widescreen"),
    ("steam_deck", "Steam Deck"),
];

/// Every option: `launcher.cfg` key, flag (`fps_limit` ↔ `--fps-limit`),
/// values, the window's label and a one-line description.
pub const OPTIONS: &[OptionSpec] = &[
    OptionSpec::new(
        "game",
        Kind::Path,
        "Game folder",
        "The folder of your installed game, with cmr4.exe. It is only read, never changed.",
    ),
    OptionSpec::new(
        "look",
        Kind::Choice(LOOKS),
        "Look",
        "Sets the picture in one go: Original as the game shipped, Widescreen fitted to your screen, or Steam Deck (1280x800 at 60 fps).",
    ),
    OptionSpec::new(
        "resolution",
        Kind::Size {
            min: (320, 240),
            max: (7680, 4320),
            aliases: cfg::ORIGINAL,
        },
        "Resolution",
        "The picture size. A 16:9 size shows the full widescreen view; other shapes keep the game's own view. Original keeps the game's own mode.",
    )
    .with_example("1920x1080"),
    OptionSpec::new(
        "fps_limit",
        Kind::Range { min: 0, max: 1000 },
        "Frame-rate cap",
        "The most frames per second the game draws; 0 or off for no cap.",
    )
    // What the window and older launchers wrote for no cap.
    .with_value_aliases(&[("off", "0")])
    .with_example("60"),
    OptionSpec::new(
        "vsync",
        Kind::Toggle,
        "VSync",
        "Waits for the screen before showing each frame: no tearing, slightly more input delay.",
    ),
    OptionSpec::new(
        "controller",
        Kind::Toggle,
        "Gamepad",
        "Lets the game use an Xbox-style pad: the left stick steers and the triggers are analogue pedals. Off: the game reads the pad on its own (under Wine only its buttons work).",
    ),
    OptionSpec::new(
        "runtime",
        Kind::Path,
        "Private copy folder",
        "Where the launcher keeps its own copy of the game, with your saves and settings. Empty: %LOCALAPPDATA%\\CMR4Launcher\\CMR4. It must be outside the game folder.",
    ),
    // `widescreen = on|off` and `--widescreen on|off` from launchers before
    // 0.3.0: accepted and ignored, commented out on save.
    OptionSpec::removed(
        "widescreen",
        "It only ever worked with a 16:9 resolution, so the resolution decides now.",
    ),
];

/// The launcher's own flags that take no value, with their `--help` text.
pub const SWITCHES: [(&str, &str); 4] = [
    ("--check", "check the game and settings only"),
    ("--no-window", "play with saved settings and flags"),
    ("--settings", "only the settings window (Linux helper)"),
    ("--linux", "started by the Linux helper (cmr4-play)"),
];

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Settings {
    pub game: Option<PathBuf>,
    pub runtime: Option<PathBuf>,
    pub look: Option<Look>,
    pub resolution: Option<Resolution>,
    /// 0 is no cap.
    pub fps_limit: Option<u32>,
    pub vsync: Option<bool>,
    pub controller: Option<bool>,
}

impl Settings {
    pub fn resolution(&self) -> Resolution {
        self.resolution.unwrap_or(DEFAULT_RESOLUTION)
    }

    /// Whether the widescreen fix is wanted: exactly when the resolution
    /// is 16:9 (it used to be its own `widescreen` setting, which only
    /// ever worked together with a 16:9 resolution).
    pub fn widescreen(&self) -> bool {
        self.resolution().is_exact_16_by_9()
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

    /// Fills the values a `look` sets on `screen`, where this same source
    /// (the file, or the command line) didn't set them itself.
    pub fn resolve_look(mut self, screen: Option<(u32, u32)>) -> Settings {
        if let Some(look) = self.look {
            self.resolution = self.resolution.or(look.resolution(screen));
            self.fps_limit = self.fps_limit.or(look.fps_limit());
        }
        self
    }

    pub fn merged(self, over: Settings) -> Settings {
        Settings {
            game: over.game.or(self.game),
            runtime: over.runtime.or(self.runtime),
            look: over.look.or(self.look),
            resolution: over.resolution.or(self.resolution),
            fps_limit: over.fps_limit.or(self.fps_limit),
            vsync: over.vsync.or(self.vsync),
            controller: over.controller.or(self.controller),
        }
    }

    /// Takes one checked value from the file or a flag.
    fn apply(&mut self, key: &str, value: Value) -> Result<(), Rejected> {
        match (key, value) {
            ("game", Value::Text(path)) => self.game = Some(path.into()),
            ("runtime", Value::Text(path)) => self.runtime = Some(path.into()),
            ("look", Value::Choice(look)) => self.look = Look::from_value(look),
            ("resolution", Value::Size(cfg::Resolution::Size(w, h))) => {
                self.resolution = Some(Resolution::Custom(w, h));
            }
            ("resolution", Value::Size(_)) => self.resolution = Some(Resolution::Original),
            ("fps_limit", Value::Number(fps)) => self.fps_limit = Some(fps),
            ("vsync", Value::Toggle(on)) => self.vsync = Some(on),
            ("controller", Value::Toggle(on)) => self.controller = Some(on),
            _ => return Err(Rejected::Unknown),
        }
        Ok(())
    }

    /// The settings in `text` and a warning per line that was refused.
    pub fn parse_file(text: &str) -> (Settings, Vec<String>) {
        let mut settings = Settings::default();
        let warnings = spec::parse_file(text, FILE_NAME, OPTIONS, |key, value| {
            settings.apply(key, value)
        });
        (settings, warnings)
    }

    /// The text of each option's value, as the file and the flags spell
    /// it (`None` when unset).
    pub fn value_of(&self, key: &str) -> Option<String> {
        let path = |path: &Option<PathBuf>| path.as_ref().map(|path| path.display().to_string());
        match key {
            "game" => path(&self.game),
            "runtime" => path(&self.runtime),
            "look" => self.look.map(|look| look.value().into()),
            "resolution" => self.resolution.map(Resolution::value),
            "fps_limit" => self
                .fps_limit
                .map(|n| if n == 0 { "off".into() } else { n.to_string() }),
            "vsync" => self.vsync.map(|v| on_off(v).into()),
            "controller" => self.controller.map(|v| on_off(v).into()),
            _ => None,
        }
    }

    /// The file `text` with these values written in. Every other line, the
    /// player's comments and keys this launcher doesn't know included, is
    /// kept; a value that is `None` is commented out so its default applies
    /// (so is a removed key). An empty or missing file starts from
    /// [`template`].
    pub fn save_into(&self, text: &str) -> String {
        let mut out = if text.trim().is_empty() {
            template()
        } else {
            refresh_header(text)
        };
        for option in spec::active(OPTIONS) {
            out = match self.value_of(option.key) {
                Some(value) => spec::set_key(&out, option, &value),
                None => clear_key(&out, option.key),
            };
        }
        spec::retire_removed(&out, OPTIONS)
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
///   neither the file nor the command line (`--resolution`, `--look`) set
///   one. The caller passes None inside gamescope, whose output is only
///   `cmr4-play`'s 1024x768 fallback and would be saved for good. A flag's
///   value is never saved.
pub fn remember(
    text: &str,
    saved: &Settings,
    cli: &Settings,
    game: &Path,
    picked: Option<Resolution>,
) -> Option<String> {
    let game_changed = should_remember_game(saved.game.as_deref(), cli.game.as_deref());
    let set = |s: &Settings| s.resolution.is_some() || s.look.is_some();
    let auto_resolution = picked.filter(|_| !set(saved) && !set(cli));
    if !game_changed && auto_resolution.is_none() {
        return None;
    }
    let mut out = if text.trim().is_empty() {
        template()
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

const HEADING: &str = "\
Colin McRae Rally 04 launcher settings.
One setting per line as key = value; # starts a comment. An unset value uses the default. Command-line options override these values for one run; only --game is remembered here. An unset resolution is filled in with the one the launcher picks from the screen.";

/// A new `launcher.cfg`: every option with its description, unset.
pub fn template() -> String {
    spec::template(HEADING, OPTIONS)
}

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

impl Cli {
    /// The kind of run these flags ask for, for the SDK's
    /// `report::Audience`.
    pub fn run(&self) -> Run {
        Run {
            command: self.help || self.check,
            no_window: self.no_window,
            settings_only: self.settings_window,
            linux: self.linux,
        }
    }

    /// Who may see this run's messages (`wine`: running under Wine). The
    /// SDK decides it; never `!no_window` or `!linux` here: `cmr4-play`,
    /// the AppImage and Proton start the game run with `--no-window` for a
    /// player who is watching (STANDARDS "Messages reach the player").
    pub fn audience(&self, wine: bool) -> Audience {
        Audience::of(self.run(), wine)
    }
}

/// `--help`: the options from [`OPTIONS`], then the switches.
pub fn usage() -> String {
    let mut out = format!(
        "CMR4Launcher {}\n\nCMR4Launcher [options]\n\n",
        env!("CARGO_PKG_VERSION")
    );
    let mut switches = SWITCHES.to_vec();
    switches.push(("--help", "show this help"));
    out.push_str(&spec::usage_with(OPTIONS, KeyStyle::Underscore, &switches));
    out.push_str(
        "\nWith no options the settings window opens. Options override launcher.cfg\n\
         for one run; only --game is remembered there. The game folder is never\n\
         modified.\n",
    );
    out
}

pub fn parse_args<I: IntoIterator<Item = String>>(args: I) -> Result<Cli, String> {
    let mut cli = Cli::default();
    let mut settings = Settings::default();
    let switches: Vec<&str> = SWITCHES.iter().map(|(flag, _)| *flag).collect();
    match spec::parse_args(
        args,
        KeyStyle::Underscore,
        OPTIONS,
        &switches,
        |key, value| settings.apply(key, value),
    ) {
        Ok(seen) => {
            for switch in seen {
                match switch.as_str() {
                    "--check" => cli.check = true,
                    "--no-window" => cli.no_window = true,
                    "--settings" => cli.settings_window = true,
                    _ => cli.linux = true,
                }
            }
        }
        Err(ArgError::Help(_)) => cli.help = true,
        Err(error) => return Err(format!("{error}\n\n{}", usage())),
    }
    cli.settings = settings;
    Ok(cli)
}

#[cfg(test)]
mod audience_tests {
    //! Who sees a message (the SDK's `report::Audience`), for the command
    //! lines `cmr4-play`, the AppImage, Proton and Windows players use.
    use super::*;
    use rea_launcher_win32::report::{Audience, Mode, Route, Terminal, route};

    fn audience(values: &[&str], wine: bool) -> Audience {
        let cli = parse_args(values.iter().map(|value| (*value).to_string())).unwrap();
        cli.audience(wine)
    }

    /// `cmr4-play`, the AppImage and Proton: no console, stdout to a log.
    const UNDER_HELPER: Terminal = Terminal {
        attached: false,
        redirected: true,
        wine: true,
    };

    #[test]
    fn the_run_comes_from_the_parsed_flags() {
        let cli = parse_args(["--check", "--no-window", "--linux"].map(String::from)).unwrap();
        assert_eq!(
            cli.run(),
            rea_launcher_win32::report::Run {
                command: true,
                no_window: true,
                settings_only: false,
                linux: true,
            }
        );
        let cli = parse_args(["--settings", "--linux"].map(String::from)).unwrap();
        assert!(cli.run().settings_only && !cli.run().command);
        // A value spelled like a switch is a value, not the switch.
        let cli = parse_args(["--game", "--check"].map(String::from)).unwrap();
        assert!(!cli.run().command);
        let help = parse_args(["--help"].map(String::from)).unwrap();
        assert!(help.run().command);
    }

    #[test]
    fn a_play_error_under_the_linux_helper_is_a_dialog() {
        // cmr4-play's game run (`--no-window`), a plain start (Proton, the
        // AppImage), and a game run the helper marks with --linux.
        for run in [&["--no-window"][..], &[], &["--no-window", "--linux"]] {
            let audience = audience(run, true);
            assert_eq!(audience.mode(), Mode::Play { interactive: true }, "{run:?}");
            assert_eq!(
                route(audience.mode(), UNDER_HELPER),
                Route {
                    print: true,
                    dialog: true
                },
                "{run:?}"
            );
        }
    }

    #[test]
    fn the_helpers_settings_window_and_check_show_no_dialog() {
        // The window reports itself; when it can't open, the helper reads
        // exit code 4 and the game run reports.
        let window = audience(&["--settings", "--linux"], true);
        assert!(!route(window.mode(), UNDER_HELPER).dialog);
        // The helper reads --check's output and exit code itself.
        let check = audience(&["--check", "--linux"], true);
        assert_eq!(check.mode(), Mode::Command { interactive: false });
    }

    #[test]
    fn check_and_help_print_and_show_a_dialog_only_when_unseen() {
        // Under Wine without the helper, stdout is a log: a dialog too.
        let check = audience(&["--check", "--no-window"], true);
        assert!(route(check.mode(), UNDER_HELPER).dialog);
        // From a Windows terminal: printed, no dialog.
        let terminal = Terminal {
            attached: true,
            redirected: false,
            wine: false,
        };
        assert_eq!(
            route(audience(&["--check"], false).mode(), terminal),
            Route {
                print: true,
                dialog: false
            }
        );
        assert!(audience(&["--help"], false).command);
    }

    #[test]
    fn a_windows_script_with_no_window_never_gets_a_dialog() {
        let script = audience(&["--no-window", "--game", "C:\\CMR4"], false);
        assert_eq!(script.mode(), Mode::Play { interactive: false });
        let redirected = Terminal {
            attached: false,
            redirected: true,
            wine: false,
        };
        assert!(!route(script.mode(), redirected).dialog);
        // A double-clicked shortcut is a person.
        assert!(audience(&[], false).interactive);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(values: &[&str]) -> Result<Cli, String> {
        parse_args(values.iter().map(|value| (*value).to_string()))
    }

    const DECK: Option<(u32, u32)> = Some((1280, 800));
    const UHD: Option<(u32, u32)> = Some((3840, 2160));

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
    fn widescreen_follows_the_resolution() {
        for (resolution, wide) in [
            (Resolution::Custom(3840, 2160), true),
            (Resolution::Custom(1360, 765), true),
            (Resolution::Custom(2560, 1080), false),
            (Resolution::Custom(1600, 1200), false),
            (Resolution::Custom(1280, 800), false),
            (Resolution::Original, false),
        ] {
            let settings = Settings {
                resolution: Some(resolution),
                ..Settings::default()
            };
            assert_eq!(settings.widescreen(), wide, "{resolution:?}");
        }
    }

    #[test]
    fn a_removed_widescreen_line_or_flag_is_ignored_and_commented_out_on_save() {
        // What v0.2.0 wrote: widescreen = off next to a 16:9 resolution.
        let old = "resolution = 1920x1080\r\nwidescreen = off\r\nvsync = on\r\n";
        let (settings, warnings) = Settings::parse_file(old);
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(settings.resolution, Some(Resolution::Custom(1920, 1080)));
        assert!(settings.widescreen(), "the resolution decides now");
        assert_eq!(settings.vsync, Some(true));
        let saved = settings.save_into(old);
        assert!(saved.contains("#widescreen =\r\n"), "{saved}");
        assert!(!saved.contains("widescreen = off"));
        assert_eq!(Settings::parse_file(&saved).0, settings);
        // Steam launch options with --widescreen still work.
        let cli = args(&["--widescreen", "off", "--vsync", "on", "--widescreen", "on"]).unwrap();
        assert_eq!(cli.settings.vsync, Some(true));
        assert!(args(&["--widescreen", "perhaps"]).is_ok());
    }

    #[test]
    fn parses_all_cli_options_and_control_flags() {
        let cli = args(&[
            "--game",
            r"D:\Games\CMR4",
            "--runtime",
            r"D:\Runtime",
            "--look",
            "steam_deck",
            "--resolution",
            "1920X1080",
            "--fps-limit",
            "120",
            "--vsync",
            "on",
            "--controller",
            "off",
            "--check",
            "--no-window",
            "--settings",
            "--linux",
        ])
        .unwrap();
        let s = &cli.settings;
        assert_eq!(s.game.as_deref(), Some(Path::new(r"D:\Games\CMR4")));
        assert_eq!(s.runtime.as_deref(), Some(Path::new(r"D:\Runtime")));
        assert_eq!(s.look, Some(Look::SteamDeck));
        assert_eq!(s.resolution, Some(Resolution::Custom(1920, 1080)));
        assert_eq!(s.fps_limit, Some(120));
        assert_eq!(s.vsync, Some(true));
        assert_eq!(s.controller, Some(false));
        assert!(cli.check && cli.no_window && cli.settings_window && cli.linux && !cli.help);
    }

    #[test]
    fn bad_values_and_unknown_flags_say_what_they_need() {
        for (argv, says) in [
            (&["--resolution", "1x2"][..], "--resolution"),
            (&["--resolution", "huge"], "WIDTHxHEIGHT or original"),
            (&["--fps-limit", "1001"], "0 to 1000"),
            (&["--fps-limit", "fast"], "0 to 1000"),
            (&["--vsync", "perhaps"], "on or off"),
            (&["--controller", "2"], "on or off"),
            (&["--look", "4k"], "original, widescreen or steam_deck"),
            (&["--game"], "--game"),
            (&["--bogus"], "--bogus"),
        ] {
            let error = args(argv).unwrap_err();
            assert!(error.contains(says), "{argv:?}: {error}");
            assert!(
                error.contains("CMR4Launcher [options]"),
                "{argv:?}: no usage"
            );
        }
    }

    #[test]
    fn help_is_generated_from_the_table_and_lists_every_flag() {
        for help in ["--help", "-h", "/?"] {
            assert!(args(&[help]).unwrap().help, "{help}");
        }
        let text = usage();
        for option in spec::active(OPTIONS) {
            let flag = option.flag_name(KeyStyle::Underscore);
            assert!(text.contains(&flag), "{flag} missing from --help");
            assert!(text.contains(option.label), "{} missing", option.label);
        }
        for (flag, _) in SWITCHES {
            assert!(text.contains(flag), "{flag} missing from --help");
        }
        assert!(text.contains("--look original|widescreen|steam_deck"));
        assert!(text.contains("--fps-limit <0-1000>"));
        assert!(!text.contains("--widescreen"));
    }

    #[test]
    fn looks_set_their_values_and_explicit_values_win() {
        let deck = args(&["--look", "steam_deck"])
            .unwrap()
            .settings
            .resolve_look(UHD);
        assert_eq!(deck.resolution, Some(Resolution::Custom(1280, 800)));
        assert_eq!(deck.fps_limit, Some(60));
        let wide = args(&["--look", "widescreen"])
            .unwrap()
            .settings
            .resolve_look(DECK);
        assert_eq!(wide.resolution, Some(Resolution::Custom(1280, 720)));
        assert_eq!(wide.fps_limit, None);
        let original = args(&["--look", "original", "--fps-limit", "30"])
            .unwrap()
            .settings
            .resolve_look(UHD);
        assert_eq!(original.resolution, Some(Resolution::Original));
        assert_eq!(original.fps_limit, Some(30));
        let mixed = args(&["--look", "steam_deck", "--resolution", "1920x1080"])
            .unwrap()
            .settings
            .resolve_look(UHD);
        assert_eq!(mixed.resolution, Some(Resolution::Custom(1920, 1080)));
        // A look on the command line beats the file's own values.
        let (file, _) = Settings::parse_file("resolution = 1600x1200\nfps_limit = 144\n");
        let merged = file.resolve_look(UHD).merged(deck);
        assert_eq!(merged.resolution, Some(Resolution::Custom(1280, 800)));
        assert_eq!(merged.fps_limit, Some(60));
        // In the file, the explicit values win over its own look line.
        let (file, _) = Settings::parse_file("resolution = 1600x1200\nlook = steam_deck\n");
        let resolved = file.resolve_look(UHD);
        assert_eq!(resolved.resolution, Some(Resolution::Custom(1600, 1200)));
        assert_eq!(resolved.fps_limit, Some(60));
        assert_eq!(recommended(DECK), Look::SteamDeck);
        assert_eq!(recommended(Some((800, 1280))), Look::SteamDeck);
        assert_eq!(recommended(UHD), Look::Widescreen);
        assert_eq!(recommended(None), Look::Widescreen);
    }

    #[test]
    fn command_line_overrides_file_and_original_is_not_empty() {
        let (saved, warnings) =
            Settings::parse_file("resolution = 1920x1080\ncontroller = off\nfps_limit = 60\n");
        assert!(warnings.is_empty());
        let over = args(&["--resolution", "original"]).unwrap().settings;
        let merged = saved.merged(over);
        assert_eq!(merged.resolution(), Resolution::Original);
        assert!(!merged.widescreen());
        assert!(!merged.controller());
        assert_eq!(merged.fps_limit(), Some(60));
    }

    #[test]
    fn cli_off_values_override_saved_on_values() {
        let (saved, warnings) =
            Settings::parse_file("fps_limit = 60\nvsync = on\ncontroller = on\n");
        assert!(warnings.is_empty());
        let over = args(&[
            "--fps-limit",
            "off",
            "--vsync",
            "off",
            "--controller",
            "off",
        ])
        .unwrap()
        .settings;
        let merged = saved.clone().merged(over);
        assert_eq!(merged.fps_limit(), None);
        assert!(!merged.vsync() && !merged.controller());
        // Absent CLI values keep the saved ones.
        assert_eq!(saved.clone().merged(Settings::default()), saved);
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
        for (raw, expected) in [("off", 0), ("OFF", 0), ("0", 0), ("1000", 1000)] {
            let (file, warnings) = Settings::parse_file(&format!("fps_limit = {raw}\n"));
            assert!(warnings.is_empty(), "{raw}");
            assert_eq!(file.fps_limit, Some(expected), "{raw}");
            assert_eq!(
                args(&["--fps-limit", raw]).unwrap().settings.fps_limit,
                Some(expected)
            );
        }
        for raw in ["1001", "-5", "fast"] {
            assert_eq!(
                Settings::parse_file(&format!("fps_limit = {raw}\n"))
                    .1
                    .len(),
                1
            );
            assert!(args(&["--fps-limit", raw]).is_err(), "{raw}");
        }
        for raw in ["319x240", "320x239", "7681x4320", "1920*1080", "x1080"] {
            assert_eq!(
                Settings::parse_file(&format!("resolution = {raw}\n"))
                    .1
                    .len(),
                1
            );
            assert!(args(&["--resolution", raw]).is_err(), "{raw}");
        }
        for raw in ["1920X1080", "1920×1080", "1920x1080"] {
            let (file, _) = Settings::parse_file(&format!("resolution = {raw}\n"));
            assert_eq!(
                file.resolution,
                Some(Resolution::Custom(1920, 1080)),
                "{raw}"
            );
        }
        for raw in ["Steam_Deck", "ORIGINAL", "widescreen"] {
            let (file, warnings) = Settings::parse_file(&format!("look = {raw}\n"));
            assert!(warnings.is_empty() && file.look.is_some(), "{raw}");
        }
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
            look: Some(Look::Original),
            resolution: Some(Resolution::Original),
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
        assert!(
            warnings[0].contains("WIDTHxHEIGHT or original"),
            "{warnings:?}"
        );
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
        // --resolution and --look: one-run overrides, never saved.
        for flag in [&["--resolution", "1920x1080"][..], &["--look", "original"]] {
            let flag = args(flag).unwrap().settings;
            assert_eq!(remember(file, &saved, &flag, game, picked), None);
        }
        // A first launch with a flag remembers only the game folder.
        let flag = args(&["--resolution", "1920x1080"]).unwrap().settings;
        let fresh = remember("", &Settings::default(), &flag, game, picked).unwrap();
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
        // Notepad ANSI (Windows-1252): "C:\Jogos d'Ação\Edição".
        let mut ansi = b"game = C:\\Jogos d'A".to_vec();
        ansi.extend([0xE7, 0xE3]);
        ansi.extend(b"o\\Edi");
        ansi.extend([0xE7, 0xE3]);
        ansi.extend(b"o\r\n");
        std::fs::write(&path, &ansi).unwrap();
        let text = read_file(&path).unwrap().unwrap();
        assert_eq!(
            Settings::parse_file(&text).0.game,
            Some(PathBuf::from("C:\\Jogos d'Ação\\Edição"))
        );
        std::fs::write(&path, "\u{feff}game = D:\\José\\Edição\n").unwrap();
        let text = read_file(&path).unwrap().unwrap();
        assert_eq!(
            Settings::parse_file(&text).0.game,
            Some(PathBuf::from("D:\\José\\Edição"))
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
        assert_eq!(refresh_header("# mine\n"), "# mine\n");
    }

    #[test]
    fn a_v0_2_0_template_still_reads_and_is_filled_in_place() {
        // The hand-written template v0.2.0 created on first run.
        let old = "# Colin McRae Rally 04 launcher settings.\n\n# The installed game folder.\ngame =\n\nresolution =\n\n# Apply the tested 16:9 Low aspect patch to the private cmr4.exe (on/off).\nwidescreen =\n\nfps_limit =\nvsync =\ncontroller =\n";
        let (settings, warnings) = Settings::parse_file(old);
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(settings, Settings::default());
        let saved = Settings {
            game: Some(PathBuf::from(r"C:\CMR4")),
            vsync: Some(true),
            ..Settings::default()
        }
        .save_into(old);
        assert!(saved.contains("\ngame = C:\\CMR4\n"), "{saved}");
        assert!(saved.contains("\nvsync = on\n"), "{saved}");
        assert_eq!(saved.matches("game =").count(), 1);
    }

    #[test]
    fn template_comes_from_the_table_and_parses_clean_to_the_defaults() {
        let text = template();
        let (settings, warnings) = Settings::parse_file(&text);
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(settings, Settings::default());
        for option in spec::active(OPTIONS) {
            assert!(
                text.contains(&format!("\n#{} ", option.key))
                    || text.contains(&format!("\n#{} =", option.key)),
                "{}",
                option.key
            );
        }
        assert!(!text.contains("widescreen ="));
        let filled = Settings {
            vsync: Some(true),
            ..Settings::default()
        }
        .save_into("");
        assert!(filled.contains("\nvsync = on\n"));
        assert_eq!(filled.matches("vsync =").count(), 1);
        // Placeholders with an example are filled in place too.
        let filled = Settings {
            resolution: Some(Resolution::Custom(2560, 1440)),
            fps_limit: Some(30),
            ..Settings::default()
        }
        .save_into("");
        assert_eq!(filled.matches("resolution =").count(), 1, "{filled}");
        assert_eq!(filled.matches("fps_limit =").count(), 1, "{filled}");
        assert!(filled.contains("\nresolution = 2560x1440\n"));
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
    fn the_table_is_consistent() {
        // Duplicate keys or flags, empty choices, bad examples: the GUI
        // parity test also checks the window's side.
        let keys: Vec<&str> = spec::active(OPTIONS).map(|o| o.key).collect();
        let problems = spec::parity_problems(OPTIONS, KeyStyle::Underscore, &keys);
        assert!(problems.is_empty(), "{problems:?}");
        for option in spec::active(OPTIONS) {
            assert!(!option.description.is_empty(), "{}", option.key);
            assert!(option.description.len() < 200, "{} too long", option.key);
        }
    }
}
