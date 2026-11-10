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
use rea_launcher_ui::unpack::{self, Unpacking};
use rea_launcher_ui::widgets::{self, Crop, LookCard};
use rea_launcher_ui::window::{Layout, WindowFit};

use rea_launcher::identify::Known;
use rea_launcher::installer::InstallerBuild;

use crate::config::{self, Look, OPTIONS, Resolution};
use crate::form::{self, Form, Tab};
use crate::{GAME_EXE, edition, source};

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
/// The photo strip's portrait in both windows (THIRD_PARTY.md).
pub const PORTRAIT: &[u8] = include_bytes!("../assets/portrait.jpg");
/// The credit the portrait's CC BY 2.0 licence asks for, under it in both
/// windows.
pub const PHOTO_CREDIT: &str = "Photo: Carolyn Williams, CC BY 2.0";

/// The unpack window: the settings window's title, photo strip and fit.
pub fn unpack_window(fit: WindowFit) -> unpack::Window<'static> {
    unpack::Window {
        title: TITLE,
        fit,
        hero_jpeg: Some(PORTRAIT),
        photo_width: PHOTO_W,
    }
}

/// What the unpack window shows for `installer` (`total` bytes unpacked),
/// with the portrait's credit.
pub fn unpacking(installer: &str, total: u64) -> Unpacking {
    Unpacking::new(TITLE, installer, total).with_credit(PHOTO_CREDIT)
}

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
    /// Opens a web page in the browser (`dialog::open_url`).
    pub open_url: fn(&str),
    /// The installers the launcher unpacks (`source::INSTALLERS`).
    pub installers: &'static [Known<InstallerBuild>],
}

/// What the Game folder field found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Check {
    Empty,
    /// A known build: what to say.
    Found(String),
    /// A build the launcher doesn't know: playable, without the fix. Holds
    /// its SHA-256.
    Unknown(String),
    /// The installer the launcher unpacks on the first Play: what to say.
    Installer(String),
    /// An installer the launcher doesn't know, left alone: the SDK's
    /// message (with where to report it) and the setup program's SHA-256.
    UnknownInstaller {
        message: String,
        sha256: String,
    },
    Problem(String),
}

impl Check {
    pub fn ready(&self) -> bool {
        matches!(
            self,
            Check::Found(_) | Check::Unknown(_) | Check::Installer(_)
        )
    }
}

/// The setup program's SHA-256 in the SDK's unknown-installer message
/// ("SHA-256: <64 hex digits>").
fn reported_sha256(message: &str) -> Option<String> {
    let rest = message.split("SHA-256: ").nth(1)?;
    let hex: String = rest.chars().take_while(char::is_ascii_hexdigit).collect();
    (hex.len() == 64).then_some(hex)
}

/// Looks at `folder` as a launch would: its `cmr4.exe` (any case), or
/// else one of `installers`.
pub fn check_game(
    folder: &str,
    widescreen: bool,
    installers: &'static [Known<InstallerBuild>],
) -> Check {
    let folder = folder.trim();
    if folder.is_empty() {
        return Check::Empty;
    }
    let dir = Path::new(folder);
    if !dir.is_dir() {
        return Check::Problem(
            "This folder doesn't exist. Check the path, or press Browse… to pick it.".into(),
        );
    }
    let Some(exe) = source::game_exe(dir) else {
        return match source::installer_in(dir, installers) {
            Ok(Some(found)) => Check::Installer(source::found_note(&found)),
            Ok(None) => Check::Problem(format!(
                "There is no {GAME_EXE} or supported installer in this folder. Choose the installed game's folder, or the installer's."
            )),
            Err(message) => match reported_sha256(&message) {
                Some(sha256) => Check::UnknownInstaller { message, sha256 },
                None => Check::Problem(message),
            },
        };
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
                Some(name) if source::is_launchers_copy(dir) => {
                    Check::Found(source::unpacked_note(name))
                }
                Some(name) => Check::Found(format!("Found {name}. Ready to play.")),
                None => Check::Unknown(exe.sha256),
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
    widgets::name_control(response, Some(row.label), Some(row.description));
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
            check: check_game(&form.game, widescreen, host.installers),
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
                self.error = Some(format!(
                    "Can't read the settings ({error}). Close any program that has launcher.cfg open, then press Play again."
                ));
                return;
            }
        };
        match rea_launcher::fsx::write_atomic(&self.path, self.form.save(&text)) {
            Ok(()) => self.played = true,
            Err(error) => {
                self.error = Some(format!(
                    "Can't save the settings ({error}). Check that the launcher's folder can be written to, then press Play again."
                ))
            }
        }
    }

    /// The whole window, in `ui`'s rectangle.
    pub fn draw(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        if self.portrait.is_none() {
            self.portrait = widgets::load_jpeg(&ctx, "portrait", PORTRAIT);
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
        let play = widgets::footer(ui, layout.footer, hint, self.check.ready());
        // The footer describes a disabled Play; a ready one keeps its hint.
        if self.check.ready() {
            widgets::name_control(&play, None, Some(hint));
        }
        if play.clicked() {
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
            self.check = check_game(&now.0, now.1, self.host.installers);
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
            (0.0, PHOTO_CREDIT, body(12.0), MUTED),
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
            "The folder with cmr4.exe or the game's installer, e.g. Z:\\home\\you\\Games\\CMR04"
        } else {
            "The folder with your installed cmr4.exe, or with the game's installer"
        };
        let pick = self.host.pick_folder;
        if widgets::folder_field(ui, &mut self.form.game, option("game").label, hint, || {
            pick("Choose the folder with cmr4.exe or the game's installer")
        }) {
            self.error = None;
        }
        mark_rendered(ui.ctx(), "game");
        // One row whatever the check says, so the Play tab never runs
        // under the footer: an unknown build gets a short note and the
        // Report button, whose link carries the code (the long message with
        // the code is its description and is in launcher.log).
        let (note, color) = match &self.check {
            Check::Empty => (option("game").description.to_string(), MUTED),
            Check::Found(name) => (name.clone(), INK),
            Check::Unknown(_) => (edition::UNKNOWN_SHORT.to_string(), ERROR),
            Check::Installer(note) => (note.clone(), INK),
            Check::UnknownInstaller { message, .. } => (
                format!(
                    "{} Report it, or choose an installed game.",
                    message.lines().next().unwrap_or_default()
                ),
                ERROR,
            ),
            Check::Problem(message) => (message.clone(), ERROR),
        };
        ui.horizontal(|ui| {
            let report = match &self.check {
                Check::Unknown(sha256) => Some((
                    "Report this version…",
                    sha256,
                    edition::unknown_message(sha256),
                )),
                Check::UnknownInstaller { message, sha256 } => {
                    Some(("Report this installer…", sha256, message.clone()))
                }
                _ => None,
            };
            if let Some((label, sha256, description)) = report {
                let report = widgets::outline_button(ui, label);
                widgets::name_control(&report, None, Some(&description));
                if report.clicked() {
                    (self.host.open_url)(&edition::report_url(sha256));
                }
            }
            ui.add(
                egui::Label::new(egui::RichText::new(note).font(body(13.0)).color(color)).wrap(),
            );
        });
        ui.add_space(10.0);
        self.look_cards(ui);
        ui.add_space(12.0);
        widgets::heading(ui, option("runtime").label);
        widgets::folder_field(
            ui,
            &mut self.form.runtime,
            option("runtime").label,
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
                    disabled: None,
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
        if !self.form.controller {
            widgets::note(
                ui,
                "With Gamepad off, the game reads the pad by itself: menus work, but under Wine the stick and triggers may not drive the car.",
            );
            return;
        }
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
            open_url: |_| {},
            installers: crate::source::INSTALLERS,
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

    /// The unpack window shows the settings window's portrait, with its
    /// CC BY 2.0 credit as a label inside the photo strip.
    #[test]
    fn the_unpack_window_shows_the_portrait_with_its_credit() {
        use rea_launcher_ui::egui::Role;
        use rea_launcher_ui::testing::Queryable;
        use rea_launcher_ui::unpack::draw_unpack;
        use rea_launcher_ui::window::SIZE;

        let page = unpack_window(WindowFit::new(SIZE, false, None, 1.0));
        assert_eq!(page.title, TITLE);
        assert_eq!(page.hero_jpeg, Some(PORTRAIT));
        assert_eq!(page.photo_width, PHOTO_W);

        let state = unpacking("the Magipack repack", 2_200_000_000);
        assert_eq!(state.game, TITLE);
        let mut ui = testing::harness(&PALETTE, &METRICS, state, |ui, u| {
            if u.hero.is_none() {
                u.hero = widgets::load_jpeg(ui.ctx(), "hero", PORTRAIT);
            }
            u.photo_width = PHOTO_W;
            draw_unpack(ui, u);
        });
        ui.run();
        assert!(ui.state().hero.is_some(), "the portrait decodes");
        let credit = ui.get_by_role_and_label(Role::Label, PHOTO_CREDIT);
        assert!(
            credit.rect().max.x <= PHOTO_W + 1.0,
            "inside the photo strip: {:?}",
            credit.rect()
        );
    }
}
