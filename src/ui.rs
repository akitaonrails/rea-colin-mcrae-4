//! The native settings window: `cmr4_launcher::window` in an eframe
//! viewport, with the XInput pad and the Windows folder picker.

use std::path::Path;

use cmr4_launcher::window::{Host, METRICS, PALETTE, TITLE, Window};
use rea_launcher_ui::pad::{self, PadNav};
use rea_launcher_ui::window::{SIZE, WindowFit};
use rea_launcher_ui::{eframe, egui, theme};
use rea_launcher_win32::{dialog, xinput};

use crate::win32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Play,
    Closed,
}

struct App<'a> {
    window: Window,
    fit: WindowFit,
    pad: PadNav,
    outcome: &'a mut Outcome,
}

impl eframe::App for App<'_> {
    fn raw_input_hook(&mut self, ctx: &egui::Context, raw: &mut egui::RawInput) {
        let navs = self.pad.update(xinput::buttons());
        pad::inject(ctx, raw, &navs);
        self.window.navigate(&navs);
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.fit.first_frame(&ctx);
        self.window.draw(ui);
        if self.window.played {
            *self.outcome = Outcome::Play;
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        ctx.request_repaint_after(std::time::Duration::from_millis(50));
    }
}

pub fn show(settings_path: &Path, text: Option<&str>, linux: bool) -> Result<Outcome, String> {
    let mut outcome = Outcome::Closed;
    // XWayland fractional scaling can lose a post-creation resize. Under Wine
    // create the final surface now; the first frame only changes egui's zoom.
    // Wine then scales the window by its own DPI setting, so that is divided
    // out. On Windows the first frame fits it to the DPI-aware scale.
    // Wine's scale first: it makes the process DPI aware, so the screen
    // size below is in pixels.
    let wine_scale = win32::wine_dpi_scale();
    let screen = win32::screen_size();
    let fit = WindowFit::new(SIZE, win32::is_wine(), screen, wine_scale).with_native_fit(true);
    let options = fit.native_options(TITLE);
    let host = Host {
        screen,
        linux,
        pads: xinput::connected(),
        pick_folder: dialog::pick_folder,
        open_url: |url| {
            let _ = dialog::open_url(url);
        },
    };
    let path = settings_path.to_path_buf();
    eframe::run_native(
        TITLE,
        options,
        Box::new(|cc| {
            theme::install_with(&cc.egui_ctx, &PALETTE, &METRICS);
            Ok(Box::new(App {
                window: Window::new(path, text, host),
                fit,
                pad: PadNav::default(),
                outcome: &mut outcome,
            }))
        }),
    )
    .map_err(|error| error.to_string())?;
    Ok(outcome)
}
