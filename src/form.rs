//! Portable state for the CMR04 settings window and its gamepad navigation.
//!
//! The Play tab offers a look first. The look is never stored:
//! `launcher.cfg` keeps the individual picture settings and the look is
//! worked out from them (`look_of`) when the window opens.

use std::path::PathBuf;

pub use crate::config::{Look, STEAM_DECK_SCREEN, recommended};
use crate::config::{Resolution, Settings, default_resolution};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Form {
    pub game: String,
    pub runtime: String,
    pub resolution: Resolution,
    /// None: no cap.
    pub fps_limit: Option<u32>,
    pub vsync: bool,
    pub controller: bool,
    /// Remembered so Custom stays picked while editing; otherwise always
    /// `look_of` the picture values.
    pub look: Look,
}

impl Form {
    /// The form for `settings` (with any `look` already resolved).
    pub fn new(settings: &Settings, screen: Option<(u32, u32)>) -> Self {
        let path = |path: &Option<PathBuf>| {
            path.as_ref()
                .map(|path| path.display().to_string())
                .unwrap_or_default()
        };
        let mut form = Self {
            game: path(&settings.game),
            runtime: path(&settings.runtime),
            resolution: settings
                .resolution
                .unwrap_or_else(|| default_resolution(screen)),
            fps_limit: settings.fps_limit(),
            vsync: settings.vsync(),
            controller: settings.controller(),
            look: Look::Custom,
        };
        form.look = form.look_of(screen);
        form
    }

    /// The form for a real first run (no `launcher.cfg` yet): the defaults
    /// with the recommended look for this screen (Steam Deck on a Deck,
    /// Widescreen elsewhere), as the CMR3 launcher does.
    pub fn first_run(screen: Option<(u32, u32)>) -> Self {
        let mut form = Self::new(&Settings::default(), screen);
        form.set_look(recommended(screen), screen);
        form
    }

    /// The form the window opens with. Only a missing settings file is a
    /// first run; any existing file, even an empty or hand-trimmed one,
    /// opens on its own values so a saved choice is never replaced.
    pub fn open(text: Option<&str>, screen: Option<(u32, u32)>) -> Self {
        match text {
            None => Self::first_run(screen),
            Some(text) => Self::new(&Settings::parse_file(text).0.resolve_look(screen), screen),
        }
    }

    /// The look the current picture values make on `screen`. Original and
    /// Widescreen leave the frame-rate cap alone, so they ignore it; Steam
    /// Deck sets it to 60.
    pub fn look_of(&self, screen: Option<(u32, u32)>) -> Look {
        [Look::Original, Look::Widescreen, Look::SteamDeck]
            .into_iter()
            .find(|look| {
                Some(self.resolution) == look.resolution(screen)
                    && look
                        .fps_limit()
                        .is_none_or(|fps| self.fps_limit == Some(fps))
            })
            .unwrap_or(Look::Custom)
    }

    /// Picks a look. Presets change only picture fields (the resolution
    /// and, for Steam Deck, the frame-rate cap); Custom keeps the current
    /// values to edit.
    pub fn set_look(&mut self, look: Look, screen: Option<(u32, u32)>) {
        self.look = look;
        if let Some(resolution) = look.resolution(screen) {
            self.resolution = resolution;
        }
        if let Some(fps) = look.fps_limit() {
            self.fps_limit = Some(fps);
        }
    }

    pub fn settings(&self) -> Settings {
        let path = |value: &str| {
            let value = value.trim();
            (!value.is_empty()).then(|| PathBuf::from(value))
        };
        Settings {
            game: path(&self.game),
            runtime: path(&self.runtime),
            look: None,
            resolution: Some(self.resolution),
            fps_limit: Some(self.fps_limit.unwrap_or(0)),
            vsync: Some(self.vsync),
            controller: Some(self.controller),
        }
    }

    /// The settings file `text` with this form's values written in; every
    /// other line (comments, keys the window doesn't show) is kept. A
    /// `look` line is commented out: the window saves the values it set.
    pub fn save(&self, text: &str) -> String {
        self.settings().save_into(text)
    }
}

/// The frame-rate caps offered, plus a value saved from the file or the
/// command line that isn't one of them.
pub fn frame_rates(current: Option<u32>) -> Vec<(Option<u32>, String)> {
    let mut rates: Vec<(Option<u32>, String)> = vec![(None, "No cap".into())];
    rates.extend([30, 60, 120, 144].map(|fps| (Some(fps), fps.to_string())));
    if !rates.iter().any(|(fps, _)| *fps == current) {
        rates.push((current, format!("{} (saved)", current.unwrap_or(0))));
    }
    rates
}

pub const VSYNCS: [(bool, &str); 2] = [(false, "Off"), (true, "On")];

/// The common 16:9 sizes; the screen's own 16:9 fit is added when it isn't
/// one of them.
const SIXTEEN_NINE: [(u32, u32); 3] = [(1920, 1080), (2560, 1440), (3840, 2160)];
/// Other shapes, played with the game's own view: 4:3, 16:10, ultrawide.
const OTHER_SHAPES: [(u32, u32); 3] = [(1600, 1200), (1920, 1200), (2560, 1080)];

/// The resolutions offered on `screen`, plus `current` when it isn't one of
/// them (a Steam Deck look, a value from the file or the command line).
pub fn resolutions(screen: Option<(u32, u32)>, current: Resolution) -> Vec<(Resolution, String)> {
    let label = |(w, h): (u32, u32)| (Resolution::Custom(w, h), format!("{w}×{h}"));
    let mut result = vec![(Resolution::Original, "Original".to_string())];
    if let Resolution::Custom(w, h) = default_resolution(screen)
        && !SIXTEEN_NINE.contains(&(w, h))
    {
        result.push(label((w, h)));
    }
    result.extend(SIXTEEN_NINE.map(label));
    result.extend(OTHER_SHAPES.map(label));
    if !result.iter().any(|(value, _)| *value == current) {
        result.push((
            current,
            format!("{} (saved)", current.value().replace('x', "×")),
        ));
    }
    result
}

/// The shape of a size, in words.
fn shape(width: u32, height: u32) -> Option<&'static str> {
    let wide = |w: u32, h: u32| u64::from(width) * u64::from(h) == u64::from(height) * u64::from(w);
    if wide(4, 3) {
        Some("4:3")
    } else if wide(16, 10) {
        Some("16:10")
    } else if wide(21, 9) || wide(64, 27) || wide(43, 18) {
        Some("Ultrawide")
    } else {
        None
    }
}

/// What the player will see with `resolution`: the line under the
/// Resolution row (the widescreen fix follows the resolution).
pub fn picture_note(resolution: Resolution) -> String {
    match resolution {
        Resolution::Original => "You'll see the game's own mode and view, as it shipped.".into(),
        r if r.is_exact_16_by_9() => "You'll see the full 16:9 widescreen view. The game draws a 4:3 picture that is stretched to fill the screen, which keeps the cars' shape.".into(),
        Resolution::Custom(w, h) => match shape(w, h) {
            Some(shape) => format!(
                "{shape}: you'll see the game's own view with its shape kept. Pick a 16:9 size for the widescreen view."
            ),
            None => "You'll see the game's own view with its shape kept. Pick a 16:9 size for the widescreen view.".into(),
        },
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Play,
    Picture,
    Controls,
}

impl Tab {
    pub const ALL: [Tab; 3] = [Tab::Play, Tab::Picture, Tab::Controls];

    pub fn step(self, by: isize) -> Self {
        let at = Self::ALL.iter().position(|&tab| tab == self).unwrap_or(0) as isize;
        Self::ALL[(at + by).rem_euclid(Self::ALL.len() as isize) as usize]
    }
}

/// The Controls tab's note. `profile::choose_gamepad` selects the pad for a
/// profile whose controls were never set up, so players need not visit the
/// game's Options (choosing the controller there can freeze it under Wine).
pub const CONTROLS_NOTE: &str = "For a profile whose controls were never set up, the launcher selects the pad and analogue pedals before the game starts; a profile you set up yourself is left as it is. In this window: D-pad moves, A activates, LB/RB change tabs, Start plays; on a keyboard, Ctrl+Tab changes tabs and Enter plays.";

#[cfg(test)]
mod tests {
    use super::*;

    const DECK: Option<(u32, u32)> = Some((1280, 800));
    const UHD: Option<(u32, u32)> = Some((3840, 2160));

    fn custom_form() -> Form {
        let mut form = Form::first_run(UHD);
        form.game = r"D:\Games\CMR04".into();
        form.runtime = r"D:\CMR04 Runtime".into();
        form.resolution = Resolution::Custom(1600, 1200);
        form.fps_limit = Some(144);
        form.vsync = true;
        form.controller = false;
        form
    }

    #[test]
    fn first_run_uses_cmr04_modern_defaults_not_a_device_preset() {
        let form = Form::first_run(UHD);
        assert_eq!(form.resolution, Resolution::Custom(3840, 2160));
        assert!(form.settings().widescreen());
        assert!(form.controller);
        assert_eq!(form.fps_limit, None);
        assert!(!form.vsync);
        assert_eq!(form.look, Look::Widescreen);
    }

    #[test]
    fn the_recommended_look_is_applied_on_the_real_first_run_only() {
        let deck = Form::open(None, DECK);
        assert_eq!(deck.look, Look::SteamDeck);
        assert_eq!(deck.resolution, Resolution::Custom(1280, 800));
        assert_eq!(deck.fps_limit, Some(60));
        assert_eq!(Form::open(None, UHD).look, Look::Widescreen);
        assert_eq!(Form::open(None, UHD), Form::first_run(UHD));
        // An existing file opens on its values, whatever the screen.
        assert_eq!(Form::open(Some(""), DECK).look, Look::Widescreen);
        let saved = Form::open(Some("resolution = original\n"), DECK);
        assert_eq!(saved.look, Look::Original);
        let mut wide = Form::first_run(DECK);
        wide.set_look(Look::Widescreen, DECK);
        let reopened = Form::open(Some(&wide.save("")), DECK);
        assert_eq!(reopened.look, Look::Widescreen);
        assert_eq!(reopened, wide);
        // A look line from the file opens as that look.
        assert_eq!(
            Form::open(Some("look = steam_deck\n"), UHD).look,
            Look::SteamDeck
        );
    }

    #[test]
    fn every_public_setting_round_trips() {
        let mut form = Form::first_run(UHD);
        form.game = r"C:\Users\José\Jogos d'Ação\CMR04".into();
        form.runtime = r"D:\CMR04 Runtime".into();
        form.resolution = Resolution::Custom(1920, 1080);
        form.fps_limit = Some(120);
        form.vsync = true;
        form.controller = false;
        form.look = form.look_of(UHD);
        assert_eq!(form.look, Look::Custom);
        let text = form.save("look = original\n");
        let (settings, warnings) = Settings::parse_file(&text);
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(settings.look, None, "the window saves values, not a look");
        assert_eq!(Form::new(&settings, UHD), form);
        assert!(!text.contains("widescreen = "));
    }

    #[test]
    fn each_look_sets_its_picture_values() {
        let mut form = custom_form();
        form.set_look(Look::Original, UHD);
        assert_eq!(form.resolution, Resolution::Original);
        assert_eq!(form.fps_limit, Some(144));

        form.set_look(Look::Widescreen, UHD);
        assert_eq!(form.resolution, Resolution::Custom(3840, 2160));
        form.set_look(Look::Widescreen, Some((2560, 1600)));
        assert_eq!(form.resolution, Resolution::Custom(2560, 1440));
        form.set_look(Look::Widescreen, Some((7680, 4320)));
        assert_eq!(form.resolution, Resolution::Custom(3840, 2160));

        form.set_look(Look::SteamDeck, UHD);
        assert_eq!(form.resolution, Resolution::Custom(1280, 800));
        assert!(!form.settings().widescreen());
        assert_eq!(form.fps_limit, Some(60));

        let before = form.clone();
        form.set_look(Look::Custom, UHD);
        assert_eq!(form.look, Look::Custom);
        assert_eq!(
            Form {
                look: before.look,
                ..form
            },
            before
        );
    }

    #[test]
    fn the_look_is_worked_out_from_saved_values() {
        for screen in [UHD, DECK, Some((1920, 1200)), None] {
            for look in [Look::Original, Look::Widescreen, Look::SteamDeck] {
                let mut form = custom_form();
                form.set_look(look, screen);
                assert_eq!(form.look_of(screen), look, "{look:?} on {screen:?}");
                let (settings, warnings) = Settings::parse_file(&form.save(""));
                assert!(warnings.is_empty(), "{warnings:?}");
                let back = Form::new(&settings, screen);
                assert_eq!(back.look, look, "{look:?} on {screen:?}");
                assert_eq!(back, form);
            }
        }
        let mut tweaked = custom_form();
        tweaked.set_look(Look::SteamDeck, DECK);
        tweaked.fps_limit = Some(30);
        assert_eq!(tweaked.look_of(DECK), Look::Custom);
        assert_eq!(custom_form().look_of(UHD), Look::Custom);
    }

    #[test]
    fn every_picture_choice_round_trips_through_launcher_cfg() {
        for screen in [UHD, DECK, None] {
            for (resolution, _) in resolutions(screen, Resolution::Original) {
                for (fps, _) in frame_rates(None) {
                    for (vsync, _) in VSYNCS {
                        let mut form = custom_form();
                        form.resolution = resolution;
                        form.fps_limit = fps;
                        form.vsync = vsync;
                        form.look = form.look_of(screen);
                        let (settings, warnings) = Settings::parse_file(&form.save(""));
                        assert!(warnings.is_empty(), "{warnings:?}");
                        assert_eq!(Form::new(&settings, screen), form);
                    }
                }
            }
        }
    }

    #[test]
    fn the_window_offers_every_shape_and_keeps_unlisted_saved_values() {
        let listed = resolutions(UHD, Resolution::Custom(3840, 2160));
        let values: Vec<String> = listed.iter().map(|(r, _)| r.value()).collect();
        assert_eq!(
            values,
            [
                "original",
                "1920x1080",
                "2560x1440",
                "3840x2160",
                "1600x1200",
                "1920x1200",
                "2560x1080"
            ]
        );
        // The screen's own fit, once; a saved odd size as a "(saved)" chip.
        let deck = resolutions(DECK, Resolution::Custom(1280, 800));
        assert_eq!(deck[1], (Resolution::Custom(1280, 720), "1280×720".into()));
        assert_eq!(
            deck.last().unwrap(),
            &(Resolution::Custom(1280, 800), "1280×800 (saved)".into())
        );
        assert_eq!(
            frame_rates(Some(90)).last().unwrap(),
            &(Some(90), "90 (saved)".into())
        );
        assert_eq!(frame_rates(Some(60)).len(), 5);
        let (settings, _) = Settings::parse_file("resolution = 1366x768\nfps_limit = 90\n");
        let form = Form::new(&settings, UHD);
        assert_eq!(form.settings().resolution, settings.resolution);
        assert_eq!(form.settings().fps_limit, settings.fps_limit);
    }

    #[test]
    fn the_picture_note_says_what_the_player_will_see() {
        assert!(picture_note(Resolution::Custom(3840, 2160)).contains("widescreen view"));
        assert!(picture_note(Resolution::Custom(1600, 1200)).starts_with("4:3"));
        assert!(picture_note(Resolution::Custom(1280, 800)).starts_with("16:10"));
        assert!(picture_note(Resolution::Custom(2560, 1080)).starts_with("Ultrawide"));
        assert!(picture_note(Resolution::Custom(3440, 1440)).starts_with("Ultrawide"));
        assert!(picture_note(Resolution::Original).contains("as it shipped"));
        for note in [
            picture_note(Resolution::Custom(1000, 999)),
            picture_note(Resolution::Original),
        ] {
            assert!(!note.contains("patch") && !note.contains("Hz"), "{note}");
        }
    }

    #[test]
    fn window_and_command_line_share_the_screen_fit_default() {
        let (settings, _) = Settings::parse_file("game = C:\\CMR4\n");
        for screen in [UHD, DECK, Some((2560, 1440)), None] {
            assert_eq!(
                Form::new(&settings, screen).resolution,
                default_resolution(screen)
            );
        }
        let saved_uncapped = Form::new(
            &Settings {
                fps_limit: Some(0),
                ..Settings::default()
            },
            UHD,
        );
        assert_eq!(saved_uncapped.fps_limit, None);
    }

    #[test]
    fn looks_never_touch_folders_vsync_or_the_controller() {
        for look in Look::ALL {
            let mut form = custom_form();
            form.set_look(look, UHD);
            assert_eq!(form.game, r"D:\Games\CMR04");
            assert_eq!(form.runtime, r"D:\CMR04 Runtime");
            assert!(!form.controller);
            assert!(form.vsync);
        }
    }

    #[test]
    fn shoulders_wrap_the_three_tabs() {
        assert_eq!(Tab::Play.step(-1), Tab::Controls);
        assert_eq!(Tab::Controls.step(1), Tab::Play);
        assert_eq!(Tab::Play.step(1), Tab::Picture);
    }

    #[test]
    fn controls_note_does_not_send_players_to_the_game_options() {
        assert!(!CONTROLS_NOTE.contains("Options"));
        assert!(CONTROLS_NOTE.contains("never set up"));
        assert!(CONTROLS_NOTE.contains("Start plays"));
    }
}
