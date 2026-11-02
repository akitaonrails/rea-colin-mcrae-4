//! CMR04's three-tab settings window. Portable state lives in `form`; the
//! theme, widgets, window sizing and pad navigation are `rea_launcher_ui`'s.

use std::path::{Path, PathBuf};

use cmr4_launcher::config::Resolution;
use cmr4_launcher::form::{self, Form, Look, Tab};
use cmr4_launcher::{GAME_EXE, patch};
use rea_launcher::identify::validate_pe32;
use rea_launcher_ui::eframe;
use rea_launcher_ui::egui::{self, Align2, Color32, Key, TextureHandle};
use rea_launcher_ui::pad::{self, Nav, PadNav};
use rea_launcher_ui::theme::{self, Metrics, Palette, body, semibold, tint};
use rea_launcher_ui::widgets::{self, Crop, LookCard};
use rea_launcher_ui::window::{Layout, SIZE, WindowFit};
use rea_launcher_win32::{dialog, xinput};

use crate::win32;

// CMR04-style palette: graphite and white type with a red-orange racing
// accent. WCAG contrast: INK on GRAPHITE 16.5:1, on PANEL 14.1:1; MUTED on
// GRAPHITE 7.7:1, on PANEL 6.6:1; ON_ACCENT on ACCENT 5.8:1 (on
// ACCENT_HOVER 6.9:1); ACCENT text on GRAPHITE 5.8:1; ERROR (amber, kept
// apart from the accent's hue) on GRAPHITE 12.3:1.
const GRAPHITE: Color32 = Color32::from_rgb(0x15, 0x17, 0x1b);
const PANEL: Color32 = Color32::from_rgb(0x22, 0x25, 0x2b);
// Control borders: 3:1 against GRAPHITE and PANEL (WCAG non-text contrast).
const LINE: Color32 = Color32::from_rgb(0x6b, 0x6e, 0x75);
const ACCENT: Color32 = Color32::from_rgb(0xff, 0x5a, 0x1f);
const ACCENT_HOVER: Color32 = Color32::from_rgb(0xff, 0x7a, 0x45);
const INK: Color32 = Color32::from_rgb(0xf4, 0xf5, 0xf7);
const MUTED: Color32 = Color32::from_rgb(0xa4, 0xaa, 0xb4);
const ON_ACCENT: Color32 = Color32::from_rgb(0x15, 0x17, 0x1b);
const ERROR: Color32 = Color32::from_rgb(0xff, 0xd0, 0x4a);

/// One accent in both roles (Play and whatever is chosen).
const PALETTE: Palette = Palette {
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
const METRICS: Metrics = Metrics {
    toggle_wrap: false,
    ..Metrics::DEFAULT
};
const PHOTO_W: f32 = 300.0;
const TITLE: &str = "Colin McRae Rally 04";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Play,
    Closed,
}

pub fn show(settings_path: &Path, text: Option<&str>, linux: bool) -> Result<Outcome, String> {
    let mut outcome = Outcome::Closed;
    let wine = win32::is_wine();
    // XWayland fractional scaling can lose a post-creation resize. Under Wine
    // create the final surface now; the first frame only changes egui's zoom.
    // Wine then scales the window by its own DPI setting, so that is divided
    // out. On Windows the first frame fits it to the DPI-aware scale.
    // Wine's scale first: it makes the process DPI aware, so the screen
    // size below is in pixels.
    let wine_scale = win32::wine_dpi_scale();
    let screen = win32::screen_size();
    let fit = WindowFit::new(SIZE, wine, screen, wine_scale).with_native_fit(true);
    let options = fit.native_options(TITLE);
    let path = settings_path.to_path_buf();
    let text = text.map(str::to_string);
    eframe::run_native(
        TITLE,
        options,
        Box::new(|cc| {
            theme::install_with(&cc.egui_ctx, &PALETTE, &METRICS);
            Ok(Box::new(Window::new(
                &cc.egui_ctx,
                path,
                text,
                linux,
                screen,
                fit,
                &mut outcome,
            )))
        }),
    )
    .map_err(|error| error.to_string())?;
    Ok(outcome)
}

enum Check {
    Empty,
    Found(String),
    Problem(String),
}

fn check_game(folder: &str, widescreen: bool) -> Check {
    let folder = folder.trim();
    if folder.is_empty() {
        return Check::Empty;
    }
    let dir = Path::new(folder);
    if !dir.is_dir() {
        return Check::Problem("No such folder.".into());
    }
    let Some(exe) = std::fs::read_dir(dir).ok().and_then(|entries| {
        entries.flatten().map(|entry| entry.path()).find(|path| {
            path.file_name()
                .is_some_and(|name| name.eq_ignore_ascii_case(GAME_EXE))
        })
    }) else {
        return Check::Problem(format!("There is no {GAME_EXE} in this folder."));
    };
    match std::fs::read(&exe) {
        Ok(bytes) => {
            if let Err(error) = validate_pe32(&bytes) {
                return Check::Problem(format!("{GAME_EXE}: {error}"));
            }
            let mut patched = bytes.clone();
            match patch::widescreen(&mut patched, widescreen) {
                Ok(outcome) => Check::Found(format!("PE32 found; widescreen check: {outcome:?}")),
                Err(error) => Check::Problem(error.to_string()),
            }
        }
        Err(error) => Check::Problem(format!("Can't read {GAME_EXE}: {error}")),
    }
}

struct Window<'a> {
    form: Form,
    tab: Tab,
    check: Check,
    checked: String,
    checked_widescreen: bool,
    screen: Option<(u32, u32)>,
    resolutions: Vec<(Resolution, String)>,
    pads: usize,
    linux: bool,
    fit: WindowFit,
    portrait: Option<TextureHandle>,
    path: PathBuf,
    error: Option<String>,
    pad: PadNav,
    pending: Vec<Nav>,
    outcome: &'a mut Outcome,
}

impl<'a> Window<'a> {
    fn new(
        ctx: &egui::Context,
        path: PathBuf,
        text: Option<String>,
        linux: bool,
        screen: Option<(u32, u32)>,
        fit: WindowFit,
        outcome: &'a mut Outcome,
    ) -> Self {
        let form = Form::open(text.as_deref(), screen);
        let check = check_game(&form.game, form.widescreen);
        Self {
            checked: form.game.clone(),
            checked_widescreen: form.widescreen,
            form,
            tab: Tab::Play,
            check,
            screen,
            resolutions: form::resolutions(screen),
            pads: xinput::connected(),
            linux,
            fit,
            portrait: widgets::load_jpeg(ctx, "portrait", include_bytes!("../assets/portrait.jpg")),
            path,
            error: None,
            pad: PadNav::default(),
            pending: Vec::new(),
            outcome,
        }
    }

    fn ready(&self) -> bool {
        matches!(self.check, Check::Found(_))
    }

    fn play(&mut self, ctx: &egui::Context) {
        if !self.ready() {
            self.tab = Tab::Play;
            return;
        }
        let settings = self.form.settings();
        if let Err(error) = cmr4_launcher::config::validate_picture(&settings) {
            self.error = Some(error);
            self.tab = Tab::Picture;
            return;
        }
        // Written into the file as it is now, keeping comments and keys the
        // window doesn't show. A file that exists but can't be read is not
        // replaced.
        let text = match cmr4_launcher::config::read_file(&self.path) {
            Ok(text) => text.unwrap_or_default(),
            Err(error) => {
                self.error = Some(format!("Can't read the settings: {error}"));
                return;
            }
        };
        match rea_launcher::fsx::write_atomic(&self.path, settings.save_into(&text)) {
            Ok(()) => {
                *self.outcome = Outcome::Play;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            Err(error) => self.error = Some(format!("Can't save the settings: {error}")),
        }
    }
}

impl eframe::App for Window<'_> {
    fn raw_input_hook(&mut self, ctx: &egui::Context, raw: &mut egui::RawInput) {
        let navs = self.pad.update(xinput::buttons());
        pad::inject(ctx, raw, &navs);
        self.pending.extend(navs);
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.fit.first_frame(&ctx);
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
        let ready = self.ready();
        if widgets::footer(
            ui,
            layout.footer,
            "Settings are saved when you press Play.",
            ready,
        )
        .clicked()
        {
            self.play(&ctx);
        }
        if ctx.input(|input| input.key_pressed(Key::Enter)) && !ctx.egui_wants_keyboard_input() {
            self.play(&ctx);
        }
        ctx.request_repaint_after(std::time::Duration::from_millis(50));
        // Taken, so a second egui pass in one frame can't play twice.
        for nav in std::mem::take(&mut self.pending) {
            match nav {
                Nav::Play => self.play(&ctx),
                Nav::NextTab => self.tab = self.tab.step(1),
                Nav::PreviousTab => self.tab = self.tab.step(-1),
                _ => {}
            }
        }
        if self.form.game != self.checked || self.form.widescreen != self.checked_widescreen {
            self.checked = self.form.game.clone();
            self.checked_widescreen = self.form.widescreen;
            self.check = check_game(&self.form.game, self.form.widescreen);
        }
    }
}

impl Window<'_> {
    fn draw_photo(&self, ui: &egui::Ui, rect: egui::Rect) {
        let painter = ui.painter();
        widgets::hero_image(painter, rect, self.portrait.as_ref(), Crop::Start, GRAPHITE);
        let x = rect.left() + 20.0;
        let y = rect.bottom() - 20.0;
        for (dy, text, font, color) in [
            (0.0, "Photo: Carolyn Williams, CC BY 2.0", body(11.0), MUTED),
            (
                16.0,
                "1968–2007, World Rally Champion 1995",
                body(13.0),
                MUTED,
            ),
            (34.0, "Colin McRae", semibold(19.0), INK),
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
            (None, Check::Found(_)) => ("Ready. Press Play or Enter.".to_string(), MUTED),
            (None, _) => (
                "First, show the launcher where the game is.".to_string(),
                MUTED,
            ),
        };
        ui.label(egui::RichText::new(text).font(body(14.0)).color(color));
    }

    fn play_tab(&mut self, ui: &mut egui::Ui) {
        widgets::heading(ui, "Game folder");
        let hint = if self.linux {
            "The folder with cmr4.exe, e.g. Z:\\home\\you\\Games\\CMR04"
        } else {
            "The folder with your installed cmr4.exe"
        };
        if widgets::folder_field(ui, &mut self.form.game, hint, || {
            dialog::pick_folder("Choose the folder with your cmr4.exe")
        }) {
            self.error = None;
        }
        let (note, color) = match &self.check {
            Check::Empty => (
                "The installed game folder is only read, never changed.".to_string(),
                MUTED,
            ),
            Check::Found(name) => (name.clone(), INK),
            Check::Problem(problem) => (problem.clone(), ERROR),
        };
        ui.label(egui::RichText::new(note).font(body(13.0)).color(color));
        ui.add_space(10.0);
        self.look_cards(ui);
        ui.add_space(12.0);
        widgets::heading(ui, "Private runtime folder");
        widgets::folder_field(
            ui,
            &mut self.form.runtime,
            "Leave empty for the launcher default",
            || dialog::pick_folder("Choose the private runtime folder"),
        );
        widgets::note(
            ui,
            "The writable copy holds the patch, wrappers, saves and settings. It must be outside the game folder.",
        );
    }

    fn look_cards(&mut self, ui: &mut egui::Ui) {
        widgets::heading(ui, "Look");
        ui.add_space(12.0);
        let fit = match cmr4_launcher::config::default_resolution(self.screen) {
            Resolution::Custom(w, h) => format!("{w}×{h}"),
            Resolution::Original => "the game's own size".to_string(),
        };
        let (deck_w, deck_h) = form::STEAM_DECK_SCREEN;
        let cards = [
            (
                Look::Original,
                "Original",
                "The game's own resolution and view.".to_string(),
                "Original resolution, widescreen off.".to_string(),
            ),
            (
                Look::FourK,
                "4K",
                "Screen-fit 16:9 up to 4K, widescreen view.".to_string(),
                format!("{fit}, 16:9 widescreen patch on."),
            ),
            (
                Look::SteamDeck,
                "Steam Deck",
                format!("{deck_w}×{deck_h} at 60 fps, fills the 16:10 screen."),
                "Native 16:10, widescreen off (the patch needs exact 16:9), 60 fps cap."
                    .to_string(),
            ),
            (
                Look::Custom,
                "Custom",
                "Pick each setting in Picture.".to_string(),
                "Keeps the current values and opens the Picture tab.".to_string(),
            ),
        ];
        let best = form::recommended(self.screen);
        let gap = 10.0;
        let width = (ui.available_width() - 3.0 * gap) / 4.0;
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = gap;
            for (look, title, note, detail) in cards {
                let card = LookCard {
                    title,
                    note: &note,
                    detail: Some(&detail),
                    selected: self.form.look == look,
                    badge: (look == best).then_some("Recommended"),
                    title_size: 0.0,
                };
                if widgets::look_card(ui, egui::vec2(width, 100.0), &card).clicked() {
                    self.form.set_look(look, self.screen);
                    self.error = None;
                    if look == Look::Custom {
                        self.tab = Tab::Picture;
                    }
                }
            }
        });
    }

    fn picture_tab(&mut self, ui: &mut egui::Ui) {
        let before = (
            self.form.resolution,
            self.form.widescreen,
            self.form.fps_limit,
        );
        // A preset (e.g. Steam Deck's 1280x800) may not be a listed size.
        let mut listed = self.resolutions.clone();
        if let Resolution::Custom(w, h) = self.form.resolution
            && !listed
                .iter()
                .any(|(value, _)| *value == self.form.resolution)
        {
            listed.push((self.form.resolution, format!("{w}×{h}")));
        }
        let resolutions = &listed;
        let row = |ui: &mut egui::Ui, name: &str, add: &mut dyn FnMut(&mut egui::Ui)| {
            ui.horizontal_top(|ui| {
                widgets::field(ui, name);
                ui.vertical(|ui| add(ui));
            });
        };
        row(ui, "Resolution", &mut |ui| {
            widgets::chips(ui, resolutions, &mut self.form.resolution, None);
        });
        if self.form.resolution == Resolution::Original {
            self.form.widescreen = false;
        }
        row(ui, "Frame-rate cap", &mut |ui| {
            widgets::chips(ui, &form::FRAME_RATES, &mut self.form.fps_limit, None);
        });
        row(ui, "VSync", &mut |ui| {
            widgets::chips(ui, &form::VSYNCS, &mut self.form.vsync, None);
        });
        ui.add_space(8.0);
        widgets::toggle(
            ui,
            &mut self.form.widescreen,
            "Widescreen: tested 16:9 Low aspect patch",
            None,
        );
        widgets::note(
            ui,
            "Widescreen requires exact 16:9. Original or another aspect (4:3, 16:10, ultrawide) needs widescreen off; custom modes use 60 Hz enumeration.",
        );
        if (
            self.form.resolution,
            self.form.widescreen,
            self.form.fps_limit,
        ) != before
        {
            self.form.look = self.form.look_of(self.screen);
        }
    }

    fn controls_tab(&mut self, ui: &mut egui::Ui) {
        widgets::toggle(
            ui,
            &mut self.form.controller,
            "Gamepad through Xidi (recommended)",
            None,
        );
        let pads = match self.pads {
            0 => "No XInput pad connected now; mouse and keyboard still work.".to_string(),
            1 => "One XInput pad connected.".to_string(),
            count => format!("{count} XInput pads connected."),
        };
        widgets::note(ui, &pads);
        ui.add_space(12.0);
        widgets::heading(ui, "What the game sees");
        for (control, action) in [
            ("Left stick", "Steering (X axis)"),
            ("RT / LT", "Accelerate / brake (Y axis, analogue)"),
            ("D-pad, A, B", "Menus (hat, buttons 1 and 2)"),
            ("Other buttons", "Their standard gamepad numbers"),
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

    #[test]
    fn the_palette_and_text_sizes_pass_the_sdk_checks() {
        assert_eq!(
            theme::contrast_problems(&PALETTE, &METRICS),
            Vec::<String>::new()
        );
        assert_eq!(theme::text_size_problems(&METRICS), Vec::<String>::new());
    }
}
