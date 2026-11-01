//! Portable state for the CMR04 settings window and its gamepad navigation.

use std::path::PathBuf;

use crate::config::{Resolution, Settings};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Form {
    pub game: String,
    pub runtime: String,
    pub resolution: Resolution,
    pub widescreen: bool,
    pub fps_limit: Option<u32>,
    pub vsync: bool,
    pub controller: bool,
}

impl Form {
    pub fn new(settings: &Settings) -> Self {
        let path = |path: &Option<PathBuf>| {
            path.as_ref()
                .map(|path| path.display().to_string())
                .unwrap_or_default()
        };
        Self {
            game: path(&settings.game),
            runtime: path(&settings.runtime),
            resolution: settings.resolution(),
            widescreen: settings.widescreen(),
            fps_limit: settings.fps_limit(),
            vsync: settings.vsync(),
            controller: settings.controller(),
        }
    }

    pub fn first_run() -> Self {
        Self::new(&Settings::default())
    }

    pub fn original_picture(&mut self) {
        self.resolution = Resolution::Original;
        self.widescreen = false;
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

    pub fn save(&self) -> String {
        self.settings().render()
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

fn landscape((a, b): (u32, u32)) -> Option<(u32, u32)> {
    let (width, height) = (a.max(b) & !1, a.min(b) & !1);
    (width > 640 && height > 480 && width <= 7680 && height <= 4320).then_some((width, height))
}

pub fn resolutions(screen: Option<(u32, u32)>) -> Vec<(Resolution, String)> {
    let common = [(1920, 1080), (2560, 1440), (3840, 2160)];
    let mut result = vec![(Resolution::Original, "Original".to_string())];
    if let Some(size) = screen.and_then(landscape)
        && !common.contains(&size)
    {
        result.push((
            Resolution::Custom(size.0, size.1),
            format!("Screen {}×{}", size.0, size.1),
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PadAction {
    Play,
    Next,
    Previous,
    Press,
    NextTab,
    PreviousTab,
}

pub const PAD_UP: u16 = 0x0001;
pub const PAD_DOWN: u16 = 0x0002;
pub const PAD_LEFT: u16 = 0x0004;
pub const PAD_RIGHT: u16 = 0x0008;
pub const PAD_START: u16 = 0x0010;
pub const PAD_LB: u16 = 0x0100;
pub const PAD_RB: u16 = 0x0200;
pub const PAD_A: u16 = 0x1000;

/// Edge-triggered XInput actions: d-pad focus, A activate, shoulders tabs,
/// Start play. Held buttons do not repeat.
pub fn pad_actions(before: u16, now: u16) -> Vec<PadAction> {
    let pressed = now & !before;
    [
        (PAD_START, PadAction::Play),
        (PAD_DOWN, PadAction::Next),
        (PAD_RIGHT, PadAction::Next),
        (PAD_UP, PadAction::Previous),
        (PAD_LEFT, PadAction::Previous),
        (PAD_A, PadAction::Press),
        (PAD_RB, PadAction::NextTab),
        (PAD_LB, PadAction::PreviousTab),
    ]
    .into_iter()
    .filter(|(bit, _)| pressed & bit != 0)
    .map(|(_, action)| action)
    .collect()
}

pub const WINDOW: (f32, f32) = (880.0, 640.0);

pub fn window_zoom(screen: Option<(u32, u32)>, native: f32) -> f32 {
    let native = if native > 0.0 { native } else { 1.0 };
    let Some((width, height)) = screen else {
        return 1.0;
    };
    let (width, height) = (width as f32, height as f32);
    let fit = 0.9 * (width / WINDOW.0).min(height / WINDOW.1);
    let target = (width.min(height) / 1440.0).max(1.0).min(fit);
    let zoom = (target / native).max(1.0);
    (zoom * 20.0).round() / 20.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_run_uses_cmr04_modern_defaults_not_a_device_preset() {
        let form = Form::first_run();
        assert_eq!(form.resolution, Resolution::Custom(3840, 2160));
        assert!(form.widescreen);
        assert!(form.controller);
        assert_eq!(form.fps_limit, None);
        assert!(!form.vsync);
    }

    #[test]
    fn every_public_setting_round_trips() {
        let mut form = Form::first_run();
        form.game = r"D:\Games\CMR04".into();
        form.runtime = r"D:\CMR04 Runtime".into();
        form.resolution = Resolution::Custom(1920, 1080);
        form.widescreen = false;
        form.fps_limit = Some(120);
        form.vsync = true;
        form.controller = false;
        let text = form.save();
        let (settings, warnings) = Settings::parse_file(&text);
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(Form::new(&settings), form);
        assert!(!text.contains("scaler"));
        assert!(!text.contains("buttons"));
        assert!(!text.contains("\nsaves ="));
    }

    #[test]
    fn original_preset_only_changes_resolution_and_widescreen() {
        let mut form = Form::first_run();
        form.fps_limit = Some(60);
        form.original_picture();
        assert_eq!(form.resolution, Resolution::Original);
        assert!(!form.widescreen);
        assert_eq!(form.fps_limit, Some(60));
        assert!(form.controller);
    }

    #[test]
    fn screen_resolution_is_offered_once_and_original_is_available() {
        let choices = resolutions(Some((2560, 1600)));
        assert_eq!(choices[0].0, Resolution::Original);
        assert_eq!(choices[1].0, Resolution::Custom(2560, 1600));
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
    fn gamepad_buttons_act_once_and_shoulders_wrap_three_tabs() {
        assert_eq!(pad_actions(0, PAD_START), [PadAction::Play]);
        assert_eq!(pad_actions(PAD_START, PAD_START), []);
        assert_eq!(
            pad_actions(0, PAD_DOWN | PAD_A),
            [PadAction::Next, PadAction::Press]
        );
        assert_eq!(pad_actions(0, PAD_RB), [PadAction::NextTab]);
        assert_eq!(pad_actions(0, PAD_LB), [PadAction::PreviousTab]);
        assert_eq!(Tab::Play.step(-1), Tab::Controls);
        assert_eq!(Tab::Controls.step(1), Tab::Play);
        assert_eq!(Tab::Play.step(1), Tab::Picture);
    }

    #[test]
    fn window_grows_to_fit_large_screens() {
        assert_eq!(window_zoom(Some((1920, 1080)), 1.0), 1.0);
        assert_eq!(window_zoom(Some((3840, 2160)), 1.0), 1.5);
        assert_eq!(window_zoom(Some((3840, 2160)), 1.5), 1.0);
        assert_eq!(window_zoom(Some((1280, 800)), 1.0), 1.0);
        assert_eq!(window_zoom(None, 1.0), 1.0);
    }
}
