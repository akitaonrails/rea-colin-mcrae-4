//! CMR04's three-tab settings window, drawn with `rea_launcher_ui`.
//!
//! Everything here is portable, so the window is tested headless
//! (`egui_kittest`) on the host and under Wine; the Windows binary
//! (`src/ui.rs`) adds the native window, the pad and the folder picker
//! through [`Host`].

use std::path::{Path, PathBuf};

use rea_launcher::cfg::{OptionSpec, spec as cfg_spec};
use rea_launcher::identify::validate_pe32;
use rea_launcher_ui::egui::{self, Align2, Color32, Key, TextureHandle};
use rea_launcher_ui::pad::Nav;
use rea_launcher_ui::spec::{self as ui_spec, mark_rendered};
use rea_launcher_ui::theme::{self, Metrics, Palette, body, semibold, tint};
use rea_launcher_ui::widgets::{self, Crop, LookCard};
use rea_launcher_ui::window::Layout;

use crate::config::{self, Look, OPTIONS, Resolution};
use crate::form::{self, Form, Tab};
use crate::{GAME_EXE, edition};

// CMR04-style palette: graphite and white type with a red-orange racing
// accent. WCAG contrast: INK on GRAPHITE 16.5:1, on PANEL 14.1:1; MUTED on
// GRAPHITE 7.7:1, on PANEL 6.6:1; ON_ACCENT on ACCENT 5.8:1 (on
// ACCENT_HOVER 6.9:1); ACCENT text on GRAPHITE 5.8:1; ERROR (amber, kept
// apart from the accent's hue) on GRAPHITE 12.3:1; LINE (control borders)
// 3.4:1 on GRAPHITE, 3.0:1 on PANEL.
pub const GRAPHITE: Color32 = Color32::from_rgb(0x15, 0x17, 0x1b);
const PANEL: Color32 = Color32::from_rgb(0x22, 0x25, 0x2b);
const LINE: Color32 = Color32::from_rgb(0x6b, 0x6e, 0x75);
const ACCENT: Color32 = Color32::from_rgb(0xff, 0x5a, 0x1f);
const ACCENT_HOVER: Color32 = Color32::from_rgb(0xff, 0x7a, 0x45);
const INK: Color32 = Color32::from_rgb(0xf4, 0xf5, 0xf7);
const MUTED: Color32 = Color32::from_rgb(0xa4, 0xaa, 0xb4);
const ON_ACCENT: Color32 = Color32::from_rgb(0x15, 0x17, 0x1b);
const ERROR: Color32 = Color32::from_rgb(0xff, 0xd0, 0x4a);

/// One accent in both roles (Play and whatever is chosen).
pub const PALETTE: Palette = Palette {
    base: GRAPHITE,
    panel: PANEL,
    line: LINE,
    ink: INK,
    muted: MUTED,
    action: ACCENT,
    action_hover: ACCENT_HOVER,
    on_action: ON_ACCENT,
    select: ACCENT,
    on_select: ON_ACCENT,
    error: ERROR,
    disabled_ink: Color32::from_rgb(0x4f, 0x55, 0x60),
    disabled_line: Color32::from_rgb(0x2a, 0x2d, 0x33),
    focus: INK,
    selection: tint(ACCENT, 128),
};
/// CMR04's widget sizes: the SDK defaults, toggle labels on one line.
pub const METRICS: Metrics = Metrics {
    toggle_wrap: false,
    ..Metrics::DEFAULT
};
pub const PHOTO_W: f32 = 300.0;
pub const TITLE: &str = "Colin McRae Rally 04";

/// What the window needs from the platform.
#[derive(Debug, Clone, Copy)]
pub struct Host {
    /// The monitor the game will fill.
    pub screen: Option<(u32, u32)>,
    /// Started by the Linux helper (paths are shown as `Z:\…`).
    pub linux: bool,
    /// XInput pads connected when the window opened.
    pub pads: usize,
    /// The folder picker (`rea_launcher_win32::dialog::pick_folder`).
    pub pick_folder: fn(&str) -> Option<PathBuf>,
}

/// What the Game folder field found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Check {
    Empty,
    /// A known build: what to say.
    Found(String),
    /// A build the launcher doesn't know: playable, without the fix.
    Unknown(String),
    Problem(String),
}

impl Check {
    pub fn ready(&self) -> bool {
        matches!(self, Check::Found(_) | Check::Unknown(_))
    }
}

/// Looks at `folder`'s `cmr4.exe` (any case) as a launch would.
pub fn check_game(folder: &str, widescreen: bool) -> Check {
    let folder = folder.trim();
    if folder.is_empty() {
        return Check::Empty;
    }
    let dir = Path::new(folder);
    if !dir.is_dir() {
        return Check::Problem("There is no such folder.".into());
    }
    let Some(exe) = std::fs::read_dir(dir).ok().and_then(|entries| {
        entries.flatten().map(|entry| entry.path()).find(|path| {
            path.file_name()
                .is_some_and(|name| name.eq_ignore_ascii_case(GAME_EXE))
        })
    }) else {
        return Check::Problem(format!(
            "There is no {GAME_EXE} in this folder. Choose the folder the game is installed in."
        ));
    };
    match std::fs::read(&exe) {
        Ok(bytes) => {
            if validate_pe32(&bytes).is_err() {
                return Check::Problem(format!(
                    "This {GAME_EXE} isn't a Windows program the launcher can start."
                ));
            }
            let exe = edition::prepare(&bytes, widescreen);
            match exe.edition {
                Some(name) => Check::Found(format!("Found {name}. Ready to play.")),
                None => Check::Unknown(edition::unknown_message(&exe.sha256)),
            }
        }
        Err(error) => Check::Problem(format!("Can't read {GAME_EXE}: {error}")),
    }
}

/// The table row for `key`.
fn option(key: &str) -> &'static OptionSpec {
    cfg_spec::find(OPTIONS, key).expect("every window control has an option row")
}

/// Names a control after its option row (label and description), for
/// screen readers and GUI tests, and records it for the parity test.
fn name_control(ui: &egui::Ui, response: &egui::Response, key: &'static str) {
    let row = option(key);
    mark_rendered(ui.ctx(), row.key);
    let (label, description) = (row.label, row.description);
    ui.ctx().accesskit_node_builder(response.id, |node| {
        node.set_label(label.to_owned());
        node.set_description(description.to_owned());
    });
}

pub struct Window {
    pub form: Form,
    pub tab: Tab,
    pub check: Check,
    checked: (String, bool),
    pub host: Host,
    portrait: Option<TextureHandle>,
    path: PathBuf,
    pub error: Option<String>,
    pending: Vec<Nav>,
    /// Set when Play saved the settings; the binary then closes the window.
    pub played: bool,
}

impl Window {
    /// The window for the settings file at `path`, whose text is `text`
    /// (None: there is none yet, a first run).
    pub fn new(path: PathBuf, text: Option<&str>, host: Host) -> Self {
        let form = Form::open(text, host.screen);
        let widescreen = form.settings().widescreen();
        Self {
            check: check_game(&form.game, widescreen),
            checked: (form.game.clone(), widescreen),
            form,
            tab: Tab::Play,
            host,
            portrait: None,
            path,
            error: None,
            pending: Vec::new(),
            played: false,
        }
    }

    /// Pad actions for the next frame (`pad::PadNav::update`).
    pub fn navigate(&mut self, navs: &[Nav]) {
        self.pending.extend_from_slice(navs);
    }

    /// Saves the settings into the file as it is now (keeping comments
    /// and keys the window doesn't show) and marks the window played. A
    /// file that exists but can't be read is not replaced.
    fn play(&mut self) {
        if !self.check.ready() {
            self.tab = Tab::Play;
            return;
        }
        let text = match config::read_file(&self.path) {
            Ok(text) => text.unwrap_or_default(),
            Err(error) => {
                self.error = Some(format!("Can't read the settings: {error}"));
                return;
            }
        };
        match rea_launcher::fsx::write_atomic(&self.path, self.form.save(&text)) {
            Ok(()) => self.played = true,
            Err(error) => self.error = Some(format!("Can't save the settings: {error}")),
        }
    }

    /// The whole window, in `ui`'s rectangle.
    pub fn draw(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        if self.portrait.is_none() {
            self.portrait =
                widgets::load_jpeg(&ctx, "portrait", include_bytes!("../assets/portrait.jpg"));
        }
        let full = ui.max_rect();
        let layout = Layout::new(full, PHOTO_W);
        ui.painter().rect_filled(full, 0.0, GRAPHITE);
        self.draw_photo(ui, layout.photo);
        widgets::stripes(
            ui.painter(),
            full,
            &[(30.0, ACCENT), (16.0, INK.gamma_multiply(0.8))],
        );
        ui.scope_builder(egui::UiBuilder::new().max_rect(layout.body), |ui| {
            ui.label(
                egui::RichText::new(TITLE)
                    .font(theme::decal(42.0))
                    .color(INK),
            );
            ui.add_space(-6.0);
            self.status(ui);
            ui.add_space(6.0);
            widgets::tabs_bar(
                ui,
                &[
                    (Tab::Play, "Play"),
                    (Tab::Picture, "Picture"),
                    (Tab::Controls, "Controls"),
                ],
                &mut self.tab,
            );
            ui.add_space(14.0);
            match self.tab {
                Tab::Play => self.play_tab(ui),
                Tab::Picture => self.picture_tab(ui),
                Tab::Controls => self.controls_tab(ui),
            }
        });
        let hint = if self.check.ready() {
            "Settings are saved when you press Play."
        } else {
            "Play needs the game folder first."
        };
        if widgets::footer(ui, layout.footer, hint, self.check.ready()).clicked() {
            self.play();
        }
        if ctx.input(|input| input.key_pressed(Key::Enter)) && !ctx.egui_wants_keyboard_input() {
            self.play();
        }
        // Taken, so a second egui pass in one frame can't play twice.
        for nav in std::mem::take(&mut self.pending) {
            match nav {
                Nav::Play => self.play(),
                Nav::NextTab => self.tab = self.tab.step(1),
                Nav::PreviousTab => self.tab = self.tab.step(-1),
                _ => {}
            }
        }
        let now = (self.form.game.clone(), self.form.settings().widescreen());
        if now != self.checked {
            self.check = check_game(&now.0, now.1);
            self.checked = now;
        }
    }

    fn draw_photo(&self, ui: &egui::Ui, rect: egui::Rect) {
        let painter = ui.painter();
        widgets::hero_image(painter, rect, self.portrait.as_ref(), Crop::Start, GRAPHITE);
        // The caption sits on a solid band, never straight on the photo.
        let band =
            egui::Rect::from_min_max(egui::pos2(rect.left(), rect.bottom() - 82.0), rect.max);
        painter.rect_filled(band, 0.0, GRAPHITE.gamma_multiply(0.92));
        let x = rect.left() + 20.0;
        let y = rect.bottom() - 16.0;
        for (dy, text, font, color) in [
            (0.0, "Photo: Carolyn Williams, CC BY 2.0", body(12.0), MUTED),
            (
                17.0,
                "1968–2007, World Rally Champion 1995",
                body(13.0),
                MUTED,
            ),
            (36.0, "Colin McRae", semibold(19.0), INK),
        ] {
            painter.text(
                egui::pos2(x, y - dy),
                Align2::LEFT_BOTTOM,
                text,
                font,
                color,
            );
        }
    }

    fn status(&self, ui: &mut egui::Ui) {
        let (text, color) = match (&self.error, &self.check) {
            (Some(error), _) => (error.clone(), ERROR),
            (None, check) if check.ready() => ("Ready. Press Play or Enter.".to_string(), MUTED),
            (None, _) => (
                "First, show the launcher where the game is.".to_string(),
                MUTED,
            ),
        };
        ui.label(egui::RichText::new(text).font(body(14.0)).color(color));
    }

    fn play_tab(&mut self, ui: &mut egui::Ui) {
        widgets::heading(ui, option("game").label);
        let hint = if self.host.linux {
            "The folder with cmr4.exe, e.g. Z:\\home\\you\\Games\\CMR04"
        } else {
            "The folder with your installed cmr4.exe"
        };
        let pick = self.host.pick_folder;
        if widgets::folder_field(ui, &mut self.form.game, hint, || {
            pick("Choose the folder with your cmr4.exe")
        }) {
            self.error = None;
        }
        mark_rendered(ui.ctx(), "game");
        let (note, color) = match &self.check {
            Check::Empty => (option("game").description.to_string(), MUTED),
            Check::Found(name) => (name.clone(), INK),
            Check::Unknown(message) | Check::Problem(message) => (message.clone(), ERROR),
        };
        ui.add(egui::Label::new(egui::RichText::new(note).font(body(13.0)).color(color)).wrap());
        ui.add_space(10.0);
        self.look_cards(ui);
        ui.add_space(12.0);
        widgets::heading(ui, option("runtime").label);
        widgets::folder_field(
            ui,
            &mut self.form.runtime,
            "Leave empty for the launcher's default",
            || pick("Choose the folder for the launcher's private copy"),
        );
        mark_rendered(ui.ctx(), "runtime");
        widgets::note(ui, option("runtime").description);
    }

    fn look_cards(&mut self, ui: &mut egui::Ui) {
        widgets::heading(ui, option("look").label);
        mark_rendered(ui.ctx(), "look");
        ui.add_space(12.0);
        let screen = self.host.screen;
        let wide = match Look::Widescreen.resolution(screen) {
            Some(Resolution::Custom(w, h)) => format!("{w}×{h}"),
            _ => "Your screen".to_string(),
        };
        let (deck_w, deck_h) = form::STEAM_DECK_SCREEN;
        let cards = [
            (
                Look::Original,
                "The game's own size and view.".to_string(),
                "Runs the game as it shipped: its own resolution, no widescreen view.",
            ),
            (
                Look::Widescreen,
                format!("{wide}, the full 16:9 view."),
                "The largest 16:9 size that fits your screen (up to 3840×2160), with the widescreen view.",
            ),
            (
                Look::SteamDeck,
                format!("{deck_w}×{deck_h} at 60 fps."),
                "Fills the Steam Deck's 16:10 screen with the game's own view, capped at 60 fps.",
            ),
            (
                Look::Custom,
                "Pick each setting in Picture.".to_string(),
                "Keeps the current values and opens the Picture tab.",
            ),
        ];
        let best = config::recommended(screen);
        let gap = 10.0;
        let width = (ui.available_width() - 3.0 * gap) / 4.0;
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = gap;
            for (look, note, detail) in cards {
                let title = match look {
                    Look::Original => "Original",
                    Look::Widescreen => "Widescreen",
                    Look::SteamDeck => "Steam Deck",
                    Look::Custom => "Custom",
                };
                let card = LookCard {
                    title,
                    note: &note,
                    detail: Some(detail),
                    selected: self.form.look == look,
                    badge: (look == best).then_some("Recommended"),
                    title_size: 0.0,
                };
                if widgets::look_card(ui, egui::vec2(width, 100.0), &card).clicked() {
                    self.form.set_look(look, screen);
                    self.error = None;
                    if look == Look::Custom {
                        self.tab = Tab::Picture;
                    }
                }
            }
        });
    }

    /// One row of the Picture tab: the option's label on the left, chips
    /// and its description (or `note`) on the right.
    fn chip_row<T: PartialEq + Clone>(
        ui: &mut egui::Ui,
        key: &'static str,
        choices: &[(T, String)],
        current: &mut T,
        note: Option<String>,
    ) -> bool {
        let mut changed = false;
        ui.horizontal_top(|ui| {
            widgets::field(ui, option(key).label);
            ui.vertical(|ui| {
                let response = widgets::chips(ui, choices, current, None);
                name_control(ui, &response, key);
                changed = response.changed();
                match note {
                    Some(note) => {
                        ui.add(
                            egui::Label::new(egui::RichText::new(note).font(body(13.0)).color(INK))
                                .wrap(),
                        );
                    }
                    None => {
                        widgets::note(ui, option(key).description);
                    }
                }
            });
        });
        ui.add_space(10.0);
        changed
    }

    fn picture_tab(&mut self, ui: &mut egui::Ui) {
        let screen = self.host.screen;
        let resolutions = form::resolutions(screen, self.form.resolution);
        let note = form::picture_note(self.form.resolution);
        let mut changed = Self::chip_row(
            ui,
            "resolution",
            &resolutions,
            &mut self.form.resolution,
            Some(note),
        );
        let rates = form::frame_rates(self.form.fps_limit);
        changed |= Self::chip_row(ui, "fps_limit", &rates, &mut self.form.fps_limit, None);
        let vsyncs = form::VSYNCS.map(|(on, label)| (on, label.to_string()));
        Self::chip_row(ui, "vsync", &vsyncs, &mut self.form.vsync, None);
        if changed {
            self.form.look = self.form.look_of(screen);
            self.error = None;
        }
    }

    fn controls_tab(&mut self, ui: &mut egui::Ui) {
        let mut value = ui_spec::toggle_text(self.form.controller).to_string();
        if ui_spec::spec_row(ui, option("controller"), &mut value).changed() {
            self.form.controller = value == "on";
        }
        let pads = match self.host.pads {
            0 => "No pad is connected right now; mouse and keyboard still work.".to_string(),
            1 => "One pad is connected.".to_string(),
            count => format!("{count} pads are connected."),
        };
        widgets::note(ui, &pads);
        ui.add_space(12.0);
        widgets::heading(ui, "What the game sees");
        for (control, action) in [
            ("Left stick", "Steering"),
            ("RT / LT", "Accelerate / brake (analogue)"),
            ("D-pad, A, B", "Menus"),
            ("Other buttons", "The game's own defaults"),
        ] {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(control).font(body(15.0)).color(INK));
                ui.label(egui::RichText::new(action).font(body(14.0)).color(MUTED));
            });
        }
        ui.add_space(8.0);
        widgets::note(ui, form::CONTROLS_NOTE);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rea_launcher_ui::testing::{self, Harness};

    pub(crate) const UHD: Option<(u32, u32)> = Some((3840, 2160));

    pub(crate) fn host(screen: Option<(u32, u32)>) -> Host {
        Host {
            screen,
            linux: false,
            pads: 1,
            pick_folder: |_| None,
        }
    }

    pub(crate) fn harness(window: Window) -> Harness<'static, Window> {
        testing::harness(&PALETTE, &METRICS, window, |ui, window| window.draw(ui))
    }

    #[test]
    fn the_palette_and_text_sizes_pass_the_sdk_checks() {
        assert_eq!(
            theme::contrast_problems(&PALETTE, &METRICS),
            Vec::<String>::new()
        );
        assert_eq!(theme::text_size_problems(&METRICS), Vec::<String>::new());
    }

    #[test]
    fn the_window_and_the_command_line_have_the_same_options() {
        let path = std::env::temp_dir().join("cmr4-window-parity.cfg");
        let mut ui = harness(Window::new(path, None, host(UHD)));
        for tab in Tab::ALL {
            ui.state_mut().tab = tab;
            ui.run();
        }
        cfg_spec::assert_parity(OPTIONS, &ui_spec::rendered_keys(&ui.ctx));
    }
}
