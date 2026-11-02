//! CMR04's three-tab settings window. Portable state and pad logic live in `form`.

use std::path::{Path, PathBuf};

use cmr4_launcher::config::Resolution;
use cmr4_launcher::form::{self, Form, Look, PadAction, Tab};
use cmr4_launcher::{GAME_EXE, patch, pe};
use eframe::egui::{
    self, Align2, Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Key, Mesh,
    Rect, Sense, Shape, Stroke, StrokeKind, TextureHandle, Vec2, pos2, vec2,
};

use crate::win32;

// CMR04-style palette: graphite and white type with a red-orange racing
// accent. WCAG contrast: INK on GRAPHITE 16.5:1, on PANEL 14.1:1; MUTED on
// GRAPHITE 7.7:1, on PANEL 6.6:1; ON_ACCENT on ACCENT 5.8:1 (on
// ACCENT_HOVER 6.9:1); ACCENT text on GRAPHITE 5.8:1; ERROR (amber, kept
// apart from the accent's hue) on GRAPHITE 12.3:1.
const GRAPHITE: Color32 = Color32::from_rgb(0x15, 0x17, 0x1b);
const PANEL: Color32 = Color32::from_rgb(0x22, 0x25, 0x2b);
const LINE: Color32 = Color32::from_rgb(0x3a, 0x3f, 0x48);
const ACCENT: Color32 = Color32::from_rgb(0xff, 0x5a, 0x1f);
const ACCENT_HOVER: Color32 = Color32::from_rgb(0xff, 0x7a, 0x45);
const INK: Color32 = Color32::from_rgb(0xf4, 0xf5, 0xf7);
const MUTED: Color32 = Color32::from_rgb(0xa4, 0xaa, 0xb4);
const ON_ACCENT: Color32 = Color32::from_rgb(0x15, 0x17, 0x1b);
const ERROR: Color32 = Color32::from_rgb(0xff, 0xd0, 0x4a);
const SIZE: Vec2 = vec2(form::WINDOW.0, form::WINDOW.1);
const PHOTO_W: f32 = 300.0;

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
    let zoom = if wine {
        form::window_zoom(win32::screen_size(), 1.0)
    } else {
        1.0
    };
    let size = SIZE * zoom;
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Colin McRae Rally 04")
            .with_inner_size(size)
            .with_min_inner_size(size)
            .with_max_inner_size(size)
            .with_resizable(false)
            .with_maximize_button(false),
        centered: !wine,
        ..Default::default()
    };
    let path = settings_path.to_path_buf();
    let text = text.map(str::to_string);
    eframe::run_native(
        "Colin McRae Rally 04",
        options,
        Box::new(|cc| {
            set_style(&cc.egui_ctx);
            Ok(Box::new(Window::new(
                &cc.egui_ctx,
                path,
                text,
                linux,
                zoom,
                &mut outcome,
            )))
        }),
    )
    .map_err(|error| error.to_string())?;
    Ok(outcome)
}

fn set_style(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    let font = |bytes: &'static [u8]| std::sync::Arc::new(FontData::from_static(bytes));
    fonts.font_data.insert(
        "barlow".into(),
        font(include_bytes!("../assets/fonts/Barlow-Regular.ttf")),
    );
    fonts.font_data.insert(
        "barlow-semibold".into(),
        font(include_bytes!("../assets/fonts/Barlow-SemiBold.ttf")),
    );
    fonts.font_data.insert(
        "condensed".into(),
        font(include_bytes!(
            "../assets/fonts/BarlowCondensed-SemiBold.ttf"
        )),
    );
    fonts.font_data.insert(
        "decal".into(),
        font(include_bytes!(
            "../assets/fonts/BarlowCondensed-BlackItalic.ttf"
        )),
    );
    for family in [FontFamily::Proportional, FontFamily::Monospace] {
        fonts.families.insert(family, vec!["barlow".into()]);
    }
    for name in ["barlow-semibold", "condensed", "decal"] {
        fonts.families.insert(
            FontFamily::Name(name.into()),
            vec![name.into(), "barlow".into()],
        );
    }
    ctx.set_fonts(fonts);
    ctx.set_theme(egui::Theme::Dark);
    ctx.all_styles_mut(|style| {
        let visuals = &mut style.visuals;
        *visuals = egui::Visuals::dark();
        visuals.panel_fill = GRAPHITE;
        visuals.extreme_bg_color = PANEL;
        visuals.text_edit_bg_color = Some(PANEL);
        visuals.override_text_color = Some(INK);
        visuals.selection.bg_fill = ACCENT.gamma_multiply(0.5);
        visuals.selection.stroke = Stroke::new(1.0, ACCENT);
        visuals.text_cursor.stroke = Stroke::new(2.0, ACCENT);
        for widget in [
            &mut visuals.widgets.inactive,
            &mut visuals.widgets.hovered,
            &mut visuals.widgets.active,
            &mut visuals.widgets.open,
        ] {
            widget.bg_stroke = Stroke::new(1.0, LINE);
            widget.corner_radius = CornerRadius::same(6);
        }
        visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, MUTED);
        visuals.widgets.active.bg_stroke = Stroke::new(1.0, ACCENT);
        style.spacing.item_spacing = vec2(8.0, 8.0);
    });
}

fn font(size: f32) -> FontId {
    FontId::new(size, FontFamily::Proportional)
}

fn named(name: &str, size: f32) -> FontId {
    FontId::new(size, FontFamily::Name(name.into()))
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
            if let Err(error) = pe::validate_pe32(&bytes) {
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
    zoom: f32,
    portrait: Option<TextureHandle>,
    path: PathBuf,
    error: Option<String>,
    sized: bool,
    pad_held: u16,
    pad_actions: Vec<PadAction>,
    outcome: &'a mut Outcome,
}

impl<'a> Window<'a> {
    fn new(
        ctx: &egui::Context,
        path: PathBuf,
        text: Option<String>,
        linux: bool,
        zoom: f32,
        outcome: &'a mut Outcome,
    ) -> Self {
        let screen = win32::screen_size();
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
            pads: win32::xinput_pads().len(),
            linux,
            zoom,
            portrait: load_portrait(ctx),
            path,
            error: None,
            sized: false,
            pad_held: 0,
            pad_actions: Vec::new(),
            outcome,
        }
    }

    fn fit_screen(&mut self, ctx: &egui::Context) {
        self.sized = true;
        let wine = win32::is_wine();
        let zoom = if wine {
            self.zoom
        } else {
            form::window_zoom(self.screen, ctx.native_pixels_per_point().unwrap_or(1.0))
        };
        self.zoom = zoom;
        if zoom != 1.0 {
            ctx.set_zoom_factor(zoom);
        }
        if wine || zoom == 1.0 {
            return;
        }
        let size = SIZE * zoom;
        for command in [
            egui::ViewportCommand::MinInnerSize(size),
            egui::ViewportCommand::MaxInnerSize(size),
            egui::ViewportCommand::InnerSize(size),
        ] {
            ctx.send_viewport_cmd(command);
        }
        if let Some(monitor) = ctx.input(|input| input.viewport().monitor_size) {
            let at = ((monitor - size) / 2.0).max(Vec2::ZERO);
            ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(at.to_pos2()));
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
        match cmr4_launcher::runtime::write_atomic(&self.path, settings.save_into(&text).as_bytes())
        {
            Ok(()) => {
                *self.outcome = Outcome::Play;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            Err(error) => self.error = Some(format!("Can't save the settings: {error}")),
        }
    }
}

fn load_portrait(ctx: &egui::Context) -> Option<TextureHandle> {
    let mut decoder = jpeg_decoder::Decoder::new(&include_bytes!("../assets/portrait.jpg")[..]);
    let rgb = decoder.decode().ok()?;
    let info = decoder.info()?;
    let size = [usize::from(info.width), usize::from(info.height)];
    (rgb.len() == size[0] * size[1] * 3).then(|| {
        ctx.load_texture(
            "portrait",
            egui::ColorImage::from_rgb(size, &rgb),
            egui::TextureOptions::LINEAR,
        )
    })
}

impl eframe::App for Window<'_> {
    fn raw_input_hook(&mut self, _ctx: &egui::Context, raw: &mut egui::RawInput) {
        let now = win32::xinput_buttons();
        self.pad_actions = form::pad_actions(self.pad_held, now);
        self.pad_held = now;
        for action in &self.pad_actions {
            let (key, modifiers) = match action {
                PadAction::Next => (Key::Tab, egui::Modifiers::NONE),
                PadAction::Previous => (Key::Tab, egui::Modifiers::SHIFT),
                PadAction::Press => (Key::Space, egui::Modifiers::NONE),
                _ => continue,
            };
            for pressed in [true, false] {
                raw.events.push(egui::Event::Key {
                    key,
                    physical_key: None,
                    pressed,
                    repeat: false,
                    modifiers,
                });
            }
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        if !self.sized {
            self.fit_screen(&ctx);
        }
        let full = ui.max_rect();
        ui.painter().rect_filled(full, 0.0, GRAPHITE);
        self.draw_photo(ui, full);
        draw_decal(ui, full);
        let content = Rect::from_min_max(
            pos2(full.left() + PHOTO_W + 12.0, full.top() + 24.0),
            pos2(full.right() - 32.0, full.bottom() - 24.0),
        );
        let footer_h = 52.0;
        let body = Rect::from_min_max(
            content.min,
            pos2(content.right(), content.bottom() - footer_h - 16.0),
        );
        ui.scope_builder(egui::UiBuilder::new().max_rect(body), |ui| {
            ui.label(
                egui::RichText::new("Colin McRae Rally 04")
                    .font(named("decal", 42.0))
                    .color(INK),
            );
            ui.add_space(-6.0);
            self.status(ui);
            ui.add_space(6.0);
            self.tabs(ui);
            ui.add_space(14.0);
            match self.tab {
                Tab::Play => self.play_tab(ui),
                Tab::Picture => self.picture_tab(ui),
                Tab::Controls => self.controls_tab(ui),
            }
        });
        let footer = Rect::from_min_max(
            pos2(content.left(), content.bottom() - footer_h),
            content.max,
        );
        self.footer(ui, footer, &ctx);
        if ctx.input(|input| input.key_pressed(Key::Enter)) && !ctx.egui_wants_keyboard_input() {
            self.play(&ctx);
        }
        ctx.request_repaint_after(std::time::Duration::from_millis(50));
        for action in std::mem::take(&mut self.pad_actions) {
            match action {
                PadAction::Play => self.play(&ctx),
                PadAction::NextTab => self.tab = self.tab.step(1),
                PadAction::PreviousTab => self.tab = self.tab.step(-1),
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
    fn draw_photo(&self, ui: &egui::Ui, full: Rect) {
        let rect = Rect::from_min_size(full.min, vec2(PHOTO_W, full.height()));
        let painter = ui.painter();
        if let Some(texture) = &self.portrait {
            let [width, height] = texture.size();
            let shown_w = rect.height() * width as f32 / height as f32;
            let u = (rect.width() / shown_w).min(1.0);
            painter.image(
                texture.id(),
                rect,
                Rect::from_min_max(pos2(0.0, 0.0), pos2(u, 1.0)),
                Color32::WHITE,
            );
        }
        gradient(
            painter,
            Rect::from_min_max(
                pos2(rect.left() + rect.width() * 0.55, rect.top()),
                rect.max,
            ),
            true,
        );
        gradient(
            painter,
            Rect::from_min_max(pos2(rect.left(), rect.bottom() - 130.0), rect.max),
            false,
        );
        let x = rect.left() + 20.0;
        let y = rect.bottom() - 20.0;
        painter.text(
            pos2(x, y),
            Align2::LEFT_BOTTOM,
            "Photo: Carolyn Williams, CC BY 2.0",
            font(11.0),
            MUTED,
        );
        painter.text(
            pos2(x, y - 16.0),
            Align2::LEFT_BOTTOM,
            "1968–2007, World Rally Champion 1995",
            font(13.0),
            MUTED,
        );
        painter.text(
            pos2(x, y - 34.0),
            Align2::LEFT_BOTTOM,
            "Colin McRae",
            named("barlow-semibold", 19.0),
            INK,
        );
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
        ui.label(egui::RichText::new(text).font(font(14.0)).color(color));
    }

    fn tabs(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 26.0;
            for (tab, name) in [
                (Tab::Play, "Play"),
                (Tab::Picture, "Picture"),
                (Tab::Controls, "Controls"),
            ] {
                let on = self.tab == tab;
                let galley = ui.painter().layout_no_wrap(
                    name.to_string(),
                    named("condensed", 22.0),
                    if on { INK } else { MUTED },
                );
                let (rect, response) =
                    ui.allocate_exact_size(galley.size() + vec2(0.0, 8.0), Sense::click());
                let color = if on || response.hovered() { INK } else { MUTED };
                ui.painter().galley(rect.min, galley, color);
                if on {
                    let bar = Rect::from_min_size(
                        pos2(rect.left(), rect.bottom() - 3.0),
                        vec2(rect.width(), 3.0),
                    );
                    ui.painter().rect_filled(bar, 0.0, ACCENT);
                }
                if response
                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                    .clicked()
                {
                    self.tab = tab;
                }
            }
        });
        let y = ui.cursor().top() - 4.0;
        let rect = ui.max_rect();
        ui.painter()
            .hline(rect.left()..=rect.right(), y, Stroke::new(1.0, LINE));
    }

    fn play_tab(&mut self, ui: &mut egui::Ui) {
        heading(ui, "Game folder");
        let hint = if self.linux {
            "The folder with cmr4.exe, e.g. Z:\\home\\you\\Games\\CMR04"
        } else {
            "The folder with your installed cmr4.exe"
        };
        if folder_field(
            ui,
            &mut self.form.game,
            hint,
            "Choose the folder with your cmr4.exe",
        ) {
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
        ui.label(egui::RichText::new(note).font(font(13.0)).color(color));
        ui.add_space(10.0);
        self.look_cards(ui);
        ui.add_space(12.0);
        heading(ui, "Private runtime folder");
        folder_field(
            ui,
            &mut self.form.runtime,
            "Leave empty for the launcher default",
            "Choose the private runtime folder",
        );
        ui.label(
            egui::RichText::new(
                "The writable copy holds the patch, wrappers, saves and settings. It must be outside the game folder.",
            )
            .font(font(13.0))
            .color(MUTED),
        );
    }

    fn look_cards(&mut self, ui: &mut egui::Ui) {
        heading(ui, "Look");
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
                if look_card(
                    ui,
                    vec2(width, 100.0),
                    self.form.look == look,
                    title,
                    &note,
                    &detail,
                    look == best,
                ) {
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
                field(ui, name);
                ui.vertical(|ui| add(ui));
            });
        };
        row(ui, "Resolution", &mut |ui| {
            chips(ui, resolutions, &mut self.form.resolution)
        });
        if self.form.resolution == Resolution::Original {
            self.form.widescreen = false;
        }
        row(ui, "Frame-rate cap", &mut |ui| {
            chips(ui, &form::FRAME_RATES, &mut self.form.fps_limit)
        });
        row(ui, "VSync", &mut |ui| {
            chips(ui, &form::VSYNCS, &mut self.form.vsync)
        });
        ui.add_space(8.0);
        toggle(
            ui,
            &mut self.form.widescreen,
            "Widescreen: tested 16:9 Low aspect patch",
        );
        ui.label(
            egui::RichText::new("Widescreen requires exact 16:9. Original or another aspect (4:3, 16:10, ultrawide) needs widescreen off; custom modes use 60 Hz enumeration.")
                .font(font(13.0))
                .color(MUTED),
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
        toggle(
            ui,
            &mut self.form.controller,
            "Gamepad through Xidi (recommended)",
        );
        let pads = match self.pads {
            0 => "No XInput pad connected now; mouse and keyboard still work.".to_string(),
            1 => "One XInput pad connected.".to_string(),
            count => format!("{count} XInput pads connected."),
        };
        ui.label(egui::RichText::new(pads).font(font(13.0)).color(MUTED));
        ui.add_space(12.0);
        heading(ui, "What the game sees");
        for (control, action) in [
            ("Left stick", "Steering (X axis)"),
            ("RT / LT", "Accelerate / brake (Y axis, analogue)"),
            ("D-pad, A, B", "Menus (hat, buttons 1 and 2)"),
            ("Other buttons", "Their standard gamepad numbers"),
        ] {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(control).font(font(15.0)).color(INK));
                ui.label(egui::RichText::new(action).font(font(14.0)).color(MUTED));
            });
        }
        ui.add_space(8.0);
        ui.label(
            egui::RichText::new(form::CONTROLS_NOTE)
                .font(font(13.0))
                .color(MUTED),
        );
    }

    fn footer(&mut self, ui: &mut egui::Ui, rect: Rect, ctx: &egui::Context) {
        let button = Rect::from_min_size(
            pos2(rect.right() - 170.0, rect.top()),
            vec2(170.0, rect.height()),
        );
        let galley = ui.painter().layout(
            "Settings are saved when you press Play.".to_string(),
            font(13.0),
            MUTED,
            button.left() - rect.left() - 20.0,
        );
        let y = rect.center().y - galley.size().y / 2.0;
        ui.painter().galley(pos2(rect.left(), y), galley, MUTED);
        let response = ui.interact(button, ui.scope_id().with("play"), Sense::click());
        let ready = self.ready();
        let fill = match (ready, response.hovered()) {
            (false, _) => LINE,
            (true, true) => ACCENT_HOVER,
            (true, false) => ACCENT,
        };
        let slant = 10.0;
        let plate = vec![
            pos2(button.left() + slant, button.top()),
            button.right_top(),
            pos2(button.right() - slant, button.bottom()),
            button.left_bottom(),
        ];
        ui.painter()
            .add(Shape::convex_polygon(plate, fill, Stroke::NONE));
        ui.painter().text(
            button.center(),
            Align2::CENTER_CENTER,
            "Play",
            named("decal", 30.0),
            if ready { ON_ACCENT } else { MUTED },
        );
        if response
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .clicked()
        {
            self.play(ctx);
        }
    }
}

fn gradient(painter: &egui::Painter, rect: Rect, horizontal: bool) {
    let mut mesh = Mesh::default();
    let clear = Color32::from_rgba_unmultiplied(GRAPHITE.r(), GRAPHITE.g(), GRAPHITE.b(), 0);
    let (a, b, c, d) = if horizontal {
        (clear, GRAPHITE, GRAPHITE, clear)
    } else {
        (clear, clear, GRAPHITE, GRAPHITE)
    };
    mesh.colored_vertex(rect.left_top(), a);
    mesh.colored_vertex(rect.right_top(), b);
    mesh.colored_vertex(rect.right_bottom(), c);
    mesh.colored_vertex(rect.left_bottom(), d);
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    painter.add(Shape::mesh(mesh));
}

fn draw_decal(ui: &mut egui::Ui, full: Rect) {
    let painter = ui.painter().with_clip_rect(full);
    for (index, (width, color)) in [(30.0, ACCENT), (16.0, INK.gamma_multiply(0.8))]
        .into_iter()
        .enumerate()
    {
        let x = full.right() - 120.0 + index as f32 * 40.0;
        let slant = 60.0;
        let points = vec![
            pos2(x, full.top()),
            pos2(x + width, full.top()),
            pos2(x + width - slant, full.top() + 112.0),
            pos2(x - slant, full.top() + 112.0),
        ];
        painter.add(Shape::convex_polygon(points, color, Stroke::NONE));
    }
}

fn heading(ui: &mut egui::Ui, text: &str) {
    ui.label(
        egui::RichText::new(text)
            .font(named("barlow-semibold", 14.0))
            .color(MUTED),
    );
}

fn field(ui: &mut egui::Ui, text: &str) {
    let (rect, _) = ui.allocate_exact_size(vec2(118.0, 30.0), Sense::hover());
    ui.painter().text(
        rect.left_center(),
        Align2::LEFT_CENTER,
        text,
        font(15.0),
        INK,
    );
}

fn folder_field(ui: &mut egui::Ui, value: &mut String, hint: &str, title: &str) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        let width = ui.available_width() - 100.0;
        let edit = egui::TextEdit::singleline(value)
            .hint_text(egui::RichText::new(hint).color(MUTED))
            .font(font(15.0))
            .margin(vec2(10.0, 8.0))
            .desired_width(width);
        changed |= ui.add(edit).changed();
        if outline_button(ui, "Browse…").clicked()
            && let Some(dir) = win32::pick_folder(title)
        {
            *value = dir.display().to_string();
            changed = true;
        }
    });
    changed
}

fn outline_button(ui: &mut egui::Ui, text: &str) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(vec2(92.0, 34.0), Sense::click());
    let hover = response.hovered();
    ui.painter().rect(
        rect,
        CornerRadius::same(6),
        if hover { LINE } else { Color32::TRANSPARENT },
        Stroke::new(1.5, ACCENT),
        StrokeKind::Inside,
    );
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        text,
        named("barlow-semibold", 15.0),
        ACCENT,
    );
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

fn chips<T: PartialEq + Copy, S: AsRef<str>>(
    ui: &mut egui::Ui,
    options: &[(T, S)],
    current: &mut T,
) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        for (value, label) in options {
            let on = *current == *value;
            let galley = ui.painter().layout_no_wrap(
                label.as_ref().to_string(),
                if on {
                    named("barlow-semibold", 14.0)
                } else {
                    font(14.0)
                },
                if on { ON_ACCENT } else { INK },
            );
            let size = vec2(galley.size().x + 24.0, 30.0);
            let (rect, response) = ui.allocate_exact_size(size, Sense::click());
            let fill = if on {
                ACCENT
            } else if response.hovered() {
                LINE
            } else {
                Color32::TRANSPARENT
            };
            ui.painter().rect(
                rect,
                CornerRadius::same(6),
                fill,
                Stroke::new(1.0, if on { ACCENT } else { LINE }),
                StrokeKind::Inside,
            );
            let at = rect.center() - galley.size() / 2.0;
            ui.painter().galley(at, galley, INK);
            if response
                .on_hover_cursor(egui::CursorIcon::PointingHand)
                .clicked()
            {
                *current = *value;
            }
        }
    });
}

fn toggle(ui: &mut egui::Ui, on: &mut bool, text: &str) {
    let galley = ui
        .painter()
        .layout_no_wrap(text.to_string(), font(15.0), INK);
    let size = vec2(46.0 + 12.0 + galley.size().x, 30.0);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    if response.clicked() {
        *on = !*on;
    }
    let amount = ui.ctx().animate_bool(response.id, *on);
    let track = Rect::from_min_size(pos2(rect.left(), rect.center().y - 13.0), vec2(46.0, 26.0));
    let painter = ui.painter();
    painter.rect(
        track,
        CornerRadius::same(13),
        if *on { ACCENT } else { Color32::TRANSPARENT },
        Stroke::new(1.5, if *on { ACCENT } else { MUTED }),
        StrokeKind::Inside,
    );
    let x = egui::lerp(track.left() + 13.0..=track.right() - 13.0, amount);
    painter.circle_filled(
        pos2(x, track.center().y),
        9.0,
        if *on { ON_ACCENT } else { MUTED },
    );
    painter.galley(
        pos2(
            track.right() + 12.0,
            rect.center().y - galley.size().y / 2.0,
        ),
        galley,
        INK,
    );
    response.on_hover_cursor(egui::CursorIcon::PointingHand);
}

/// One look preset card; true when clicked (or activated with Space/A while
/// focused, which is how the gamepad reaches it).
fn look_card(
    ui: &mut egui::Ui,
    size: Vec2,
    on: bool,
    title: &str,
    note: &str,
    detail: &str,
    recommended: bool,
) -> bool {
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    let focused = response.has_focus();
    let painter = ui.painter();
    let fill = if on {
        PANEL
    } else if response.hovered() || focused {
        PANEL.gamma_multiply(0.6)
    } else {
        Color32::TRANSPARENT
    };
    let stroke = if on {
        Stroke::new(2.0, ACCENT)
    } else if focused {
        Stroke::new(2.0, INK)
    } else {
        Stroke::new(1.0, LINE)
    };
    painter.rect(
        rect,
        CornerRadius::same(8),
        fill,
        stroke,
        StrokeKind::Inside,
    );
    if on && focused {
        painter.rect_stroke(
            rect.expand(3.0),
            CornerRadius::same(10),
            Stroke::new(1.5, INK),
            StrokeKind::Inside,
        );
    }
    let pad = 12.0;
    painter.text(
        rect.left_top() + vec2(pad, pad - 2.0),
        Align2::LEFT_TOP,
        title,
        named("condensed", 22.0),
        if on { ACCENT } else { INK },
    );
    if recommended {
        let galley = painter.layout_no_wrap(
            "Recommended".into(),
            named("barlow-semibold", 12.0),
            ON_ACCENT,
        );
        let tab = Rect::from_min_size(
            pos2(
                rect.right() - pad - galley.size().x - 12.0,
                rect.top() - 9.0,
            ),
            galley.size() + vec2(12.0, 2.0),
        );
        painter.rect_filled(tab, CornerRadius::same(4), ACCENT);
        painter.galley(tab.min + vec2(6.0, 1.0), galley, ON_ACCENT);
    }
    let galley = painter.layout(note.to_string(), font(13.0), MUTED, size.x - 2.0 * pad);
    painter.galley(rect.left_top() + vec2(pad, pad + 30.0), galley, MUTED);
    response
        .on_hover_text(detail)
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .clicked()
}
