//! Portable state for the CMR04 settings window and its gamepad navigation.
//!
//! The Play tab offers a look preset first. The preset is never stored:
//! `launcher.cfg` keeps the individual picture settings and the look is
//! worked out from them (`look_of`) when the window opens.

use std::path::PathBuf;

pub use rea_launcher::display::STEAM_DECK_SCREEN;
use rea_launcher::display::is_steam_deck;

use crate::config::{Resolution, Settings, default_resolution};

/// Picture presets offered on the Play tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Look {
    /// The game's own resolution, widescreen off.
    Original,
    /// The largest exact 16:9 that fits the screen (up to 3840x2160),
    /// with the 16:9 Low widescreen patch.
    FourK,
    /// Native 1280x800 with widescreen off (the patch needs exact 16:9;
    /// the unpatched game keeps correct proportions at 16:10), 60 fps.
    SteamDeck,
    /// Whatever the Picture tab says.
    Custom,
}

impl Look {
    pub const ALL: [Look; 4] = [Look::Original, Look::FourK, Look::SteamDeck, Look::Custom];
}

/// The look to recommend: Steam Deck on a 1280x800 screen (either
/// orientation), otherwise 4K.
pub fn recommended(screen: Option<(u32, u32)>) -> Look {
    if is_steam_deck(screen) {
        Look::SteamDeck
    } else {
        Look::FourK
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Form {
    pub game: String,
    pub runtime: String,
    pub resolution: Resolution,
    pub widescreen: bool,
    pub fps_limit: Option<u32>,
    pub vsync: bool,
    pub controller: bool,
    /// Remembered so Custom stays picked while editing; otherwise always
    /// `look_of` the picture values.
    pub look: Look,
}

impl Form {
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
            widescreen: settings.widescreen(),
            fps_limit: settings.fps_limit(),
            vsync: settings.vsync(),
            controller: settings.controller(),
            look: Look::Custom,
        };
        form.look = form.look_of(screen);
        form
    }

    /// The form for a real first run (no `launcher.cfg` yet): the defaults
    /// with the recommended look for this screen (Steam Deck on a Deck, 4K
    /// elsewhere), as the CMR3 launcher does.
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
            Some(text) => Self::new(&Settings::parse_file(text).0, screen),
        }
    }

    pub fn original_picture(&mut self) {
        self.resolution = Resolution::Original;
        self.widescreen = false;
    }

    /// The look the current picture values make on `screen`. Original and
    /// 4K leave the frame-rate cap alone, so they ignore it; Steam Deck
    /// sets it to 60.
    pub fn look_of(&self, screen: Option<(u32, u32)>) -> Look {
        let (w, h) = STEAM_DECK_SCREEN;
        if self.resolution == Resolution::Original && !self.widescreen {
            Look::Original
        } else if self.resolution == Resolution::Custom(w, h)
            && !self.widescreen
            && self.fps_limit == Some(60)
        {
            Look::SteamDeck
        } else if self.resolution == default_resolution(screen) && self.widescreen {
            Look::FourK
        } else {
            Look::Custom
        }
    }

    /// Picks a look. Presets change only picture fields (resolution,
    /// widescreen and, for Steam Deck, the frame-rate cap); Custom keeps
    /// the current values to edit.
    pub fn set_look(&mut self, look: Look, screen: Option<(u32, u32)>) {
        self.look = look;
        match look {
            Look::Original => self.original_picture(),
            Look::FourK => {
                self.resolution = default_resolution(screen);
                self.widescreen = true;
            }
            Look::SteamDeck => {
                let (w, h) = STEAM_DECK_SCREEN;
                self.resolution = Resolution::Custom(w, h);
                self.widescreen = false;
                self.fps_limit = Some(60);
            }
            Look::Custom => {}
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
            resolution: Some(self.resolution),
            widescreen: Some(self.widescreen),
            fps_limit: Some(self.fps_limit.unwrap_or(0)),
            vsync: Some(self.vsync),
            controller: Some(self.controller),
        }
    }

    /// The settings file `text` with this form's values written in; every
    /// other line (comments, keys the window doesn't show) is kept.
    pub fn save(&self, text: &str) -> String {
        self.settings().save_into(text)
    }
}

pub const FRAME_RATES: [(Option<u32>, &str); 5] = [
    (None, "Uncapped"),
    (Some(30), "30"),
    (Some(60), "60"),
    (Some(120), "120"),
    (Some(144), "144"),
];
pub const VSYNCS: [(bool, &str); 2] = [(false, "Off"), (true, "On")];

pub fn resolutions(screen: Option<(u32, u32)>) -> Vec<(Resolution, String)> {
    let common = [(1920, 1080), (2560, 1440), (3840, 2160)];
    let mut result = vec![(Resolution::Original, "Original".to_string())];
    if let Resolution::Custom(width, height) = default_resolution(screen)
        && !common.contains(&(width, height))
    {
        result.push((
            Resolution::Custom(width, height),
            format!("Screen fit {width}×{height}"),
        ));
    }
    for (width, height) in common {
        let label = match height {
            1080 => "1080p",
            1440 => "1440p",
            _ => "4K",
        };
        result.push((Resolution::Custom(width, height), label.to_string()));
    }
    result
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
pub const CONTROLS_NOTE: &str = "For a profile whose controls were never set up, the launcher selects the pad and analogue pedals before the game starts; a profile you set up yourself is left as it is. In this window: D-pad moves, A activates, LB/RB change tabs, Start plays.";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_run_uses_cmr04_modern_defaults_not_a_device_preset() {
        let form = Form::first_run(Some((3840, 2160)));
        assert_eq!(form.resolution, Resolution::Custom(3840, 2160));
        assert!(form.widescreen);
        assert!(form.controller);
        assert_eq!(form.fps_limit, None);
        assert!(!form.vsync);
        assert_eq!(form.look, Look::FourK);
    }

    #[test]
    fn the_recommended_look_is_applied_on_the_real_first_run_only() {
        let deck = Form::open(None, DECK);
        assert_eq!(deck.look, Look::SteamDeck);
        assert_eq!(deck.resolution, Resolution::Custom(1280, 800));
        assert_eq!(deck.fps_limit, Some(60));
        assert_eq!(Form::open(None, UHD).look, Look::FourK);
        assert_eq!(Form::open(None, UHD), Form::first_run(UHD));
        // An existing file opens on its values, whatever the screen.
        assert_eq!(Form::open(Some(""), DECK).look, Look::FourK);
        let saved = Form::open(Some("resolution = original\nwidescreen = off\n"), DECK);
        assert_eq!(saved.look, Look::Original);
        let mut four_k = Form::first_run(DECK);
        four_k.set_look(Look::FourK, DECK);
        let reopened = Form::open(Some(&four_k.save("")), DECK);
        assert_eq!(reopened.look, Look::FourK);
        assert_eq!(reopened, four_k);
    }

    #[test]
    fn every_public_setting_round_trips() {
        let mut form = Form::first_run(Some((3840, 2160)));
        form.game = r"D:\Games\CMR04".into();
        form.runtime = r"D:\CMR04 Runtime".into();
        form.resolution = Resolution::Custom(1920, 1080);
        form.widescreen = false;
        form.fps_limit = Some(120);
        form.vsync = true;
        form.controller = false;
        form.look = form.look_of(Some((3840, 2160)));
        assert_eq!(form.look, Look::Custom);
        let text = form.save("");
        let (settings, warnings) = Settings::parse_file(&text);
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(Form::new(&settings, Some((3840, 2160))), form);
        assert!(!text.contains("scaler"));
        assert!(!text.contains("buttons"));
        assert!(!text.contains("\nsaves ="));
    }

    #[test]
    fn original_preset_only_changes_resolution_and_widescreen() {
        let mut form = Form::first_run(Some((3840, 2160)));
        form.fps_limit = Some(60);
        form.original_picture();
        assert_eq!(form.resolution, Resolution::Original);
        assert!(!form.widescreen);
        assert_eq!(form.fps_limit, Some(60));
        assert!(form.controller);
    }

    #[test]
    fn screen_resolution_is_offered_once_and_original_is_available() {
        let choices = resolutions(Some((1280, 800)));
        assert_eq!(choices[0].0, Resolution::Original);
        assert_eq!(choices[1].0, Resolution::Custom(1280, 720));
        let listed = resolutions(Some((1920, 1080)));
        assert_eq!(
            listed
                .iter()
                .filter(|(resolution, _)| *resolution == Resolution::Custom(1920, 1080))
                .count(),
            1
        );
        assert_eq!(resolutions(None).len(), 4);
    }

    #[test]
    fn shoulders_wrap_the_three_tabs() {
        assert_eq!(Tab::Play.step(-1), Tab::Controls);
        assert_eq!(Tab::Controls.step(1), Tab::Play);
        assert_eq!(Tab::Play.step(1), Tab::Picture);
    }

    #[test]
    fn controls_note_does_not_send_players_to_the_game_options() {
        // The launcher picks the pad itself (`profile`); the old note told
        // players to pick it in Options, which can freeze the game under Wine.
        assert!(!CONTROLS_NOTE.contains("Options"));
        assert!(CONTROLS_NOTE.contains("never set up"));
        assert!(CONTROLS_NOTE.contains("Start plays"));
    }

    const DECK: Option<(u32, u32)> = Some((1280, 800));
    const UHD: Option<(u32, u32)> = Some((3840, 2160));

    fn custom_form() -> Form {
        let mut form = Form::first_run(UHD);
        form.game = r"D:\Games\CMR04".into();
        form.runtime = r"D:\CMR04 Runtime".into();
        form.resolution = Resolution::Custom(1600, 1200);
        form.widescreen = false;
        form.fps_limit = Some(144);
        form.vsync = true;
        form.controller = false;
        form
    }

    #[test]
    fn each_look_sets_its_picture_values() {
        let mut form = custom_form();
        form.set_look(Look::Original, UHD);
        assert_eq!(form.resolution, Resolution::Original);
        assert!(!form.widescreen);
        assert_eq!(form.fps_limit, Some(144));

        form.set_look(Look::FourK, UHD);
        assert_eq!(form.resolution, Resolution::Custom(3840, 2160));
        assert!(form.widescreen);
        form.set_look(Look::FourK, Some((2560, 1600)));
        assert_eq!(form.resolution, Resolution::Custom(2560, 1440));
        form.set_look(Look::FourK, Some((7680, 4320)));
        assert_eq!(form.resolution, Resolution::Custom(3840, 2160));

        form.set_look(Look::SteamDeck, UHD);
        assert_eq!(form.resolution, Resolution::Custom(1280, 800));
        assert!(!form.widescreen);
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
    fn every_look_passes_picture_validation() {
        for screen in [UHD, DECK, Some((800, 1280)), Some((2560, 1080)), None] {
            for look in Look::ALL {
                let mut form = custom_form();
                form.set_look(look, screen);
                crate::config::validate_picture(&form.settings())
                    .unwrap_or_else(|error| panic!("{look:?} on {screen:?}: {error}"));
            }
        }
    }

    #[test]
    fn the_look_is_worked_out_from_saved_values() {
        for screen in [UHD, DECK, Some((1920, 1200)), None] {
            for look in [Look::Original, Look::FourK, Look::SteamDeck] {
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
    fn steam_deck_is_recommended_only_on_a_deck_screen() {
        assert_eq!(recommended(DECK), Look::SteamDeck);
        assert_eq!(recommended(Some((800, 1280))), Look::SteamDeck);
        assert_eq!(recommended(UHD), Look::FourK);
        assert_eq!(recommended(Some((1280, 720))), Look::FourK);
        assert_eq!(recommended(None), Look::FourK);
    }

    #[test]
    fn every_picture_choice_round_trips_through_launcher_cfg() {
        for screen in [UHD, DECK, None] {
            for (resolution, _) in resolutions(screen) {
                for (fps, _) in FRAME_RATES {
                    for (vsync, _) in VSYNCS {
                        let mut form = custom_form();
                        form.resolution = resolution;
                        form.widescreen = resolution.is_exact_16_by_9();
                        form.fps_limit = fps;
                        form.vsync = vsync;
                        form.look = form.look_of(screen);
                        crate::config::validate_picture(&form.settings()).unwrap();
                        let (settings, warnings) = Settings::parse_file(&form.save(""));
                        assert!(warnings.is_empty(), "{warnings:?}");
                        assert_eq!(Form::new(&settings, screen), form);
                    }
                }
            }
        }
    }

    #[test]
    fn unlisted_saved_values_survive_the_window() {
        let (settings, _) =
            Settings::parse_file("resolution = 1600x1200\nwidescreen = off\nfps_limit = 90\n");
        let form = Form::new(&settings, UHD);
        assert_eq!(form.resolution, Resolution::Custom(1600, 1200));
        assert_eq!(form.fps_limit, Some(90));
        assert_eq!(form.look, Look::Custom);
        assert_eq!(form.settings().resolution, settings.resolution);
        assert_eq!(form.settings().fps_limit, settings.fps_limit);
    }

    #[test]
    fn window_and_command_line_share_the_screen_fit_default() {
        // A saved file without a resolution opens on the same mode the
        // launcher picks when it starts without the window.
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
}
