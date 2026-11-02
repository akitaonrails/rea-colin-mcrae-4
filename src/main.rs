#![cfg_attr(windows, windows_subsystem = "windows")]

#[cfg(windows)]
mod ui;
#[cfg(windows)]
mod win32;

#[cfg(windows)]
mod app {
    use std::fs;
    use std::path::{Path, PathBuf};

    use cmr4_launcher::config::{self, Settings};
    use cmr4_launcher::patch::{self, Outcome};
    use cmr4_launcher::{GAME_EXE, components, profile, registry, runtime};
    use rea_launcher::identify::validate_pe32;
    use rea_launcher::log::Log;
    use rea_launcher::{fsx, paths};
    use rea_launcher_win32::dialog::{self, Icon};
    use rea_launcher_win32::gpu;

    use crate::{ui, win32};

    const WINDOW_CLOSED: u32 = 3;
    const WINDOW_UNAVAILABLE: u32 = 4;

    fn home() -> PathBuf {
        std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir)
            .join("CMR4Launcher")
    }

    /// `launcher.log`, never truncated (as before the shared SDK).
    pub fn log_line(line: &str) {
        Log::new(home().join("launcher.log"))
            .without_limit()
            .line(line);
    }

    pub fn message(text: &str, error: bool) {
        let icon = if error { Icon::Error } else { Icon::Info };
        dialog::message("Colin McRae Rally 04", text, icon);
    }

    fn settings_path() -> PathBuf {
        let portable = std::env::current_exe()
            .ok()
            .and_then(|path| path.parent().map(|dir| dir.join(config::FILE_NAME)));
        match portable {
            Some(path) if path.is_file() => path,
            _ => home().join(config::FILE_NAME),
        }
    }

    fn find_game(settings: &Settings) -> Result<PathBuf, String> {
        if let Some(game) = &settings.game {
            return Ok(game.clone());
        }
        if let Some(dir) = std::env::current_exe()
            .ok()
            .and_then(|path| path.parent().map(Path::to_path_buf))
            && dir.join(GAME_EXE).is_file()
        {
            return Ok(dir);
        }
        Err(format!(
            "tell the launcher where the game is with --game <folder containing {GAME_EXE}>"
        ))
    }

    fn validate_game(game: &Path, widescreen: bool) -> Result<(String, Outcome), String> {
        let path = game.join(GAME_EXE);
        let bytes =
            fs::read(&path).map_err(|error| format!("cannot read {}: {error}", path.display()))?;
        validate_pe32(&bytes).map_err(|error| format!("{}: {error}", path.display()))?;
        let mut patched = bytes.clone();
        let outcome = patch::widescreen(&mut patched, widescreen)
            .map_err(|error| format!("{}: {error}", path.display()))?;
        Ok((patch::sha256(&bytes), outcome))
    }

    /// The settings file's text, or None when there is none yet. A file that
    /// exists but can't be read stops the launch rather than being replaced.
    fn read_settings(path: &Path) -> Result<Option<String>, String> {
        config::read_file(path).map_err(|error| format!("cannot read {}: {error}", path.display()))
    }

    /// Writes back the game folder and an auto-picked resolution (see
    /// `config::remember`), keeping every other line of the file.
    fn remember(path: &Path, text: Option<String>) -> Result<(), String> {
        let Some(text) = text else {
            return Ok(());
        };
        // The error names the file.
        fsx::write_atomic(path, text).map_err(|error| format!("cannot write {error}"))
    }

    /// Points an untouched game profile at the connected pad (see `profile`).
    fn choose_gamepad(path: &Path) {
        let Some((vendor, product)) = win32::gamepad_ids() else {
            log_line("gamepad setup: no XInput pad in slot 0; profile left as is");
            return;
        };
        match profile::choose_gamepad_in(path, vendor, product) {
            Ok(true) => log_line(&format!(
                "gamepad setup: profile now uses pad {vendor:04x}:{product:04x} with analogue pedals"
            )),
            Ok(false) => log_line("gamepad setup: profile already configured or not created yet"),
            Err(error) => log_line(&format!("warning: gamepad setup: {error}")),
        }
    }

    pub fn run() -> Result<u32, String> {
        log_line(concat!("CMR4Launcher ", env!("CARGO_PKG_VERSION")));
        let raw: Vec<String> = std::env::args().skip(1).collect();
        let cli = config::parse_args(raw.clone())?;
        if cli.help {
            log_line("help requested");
            message(config::USAGE, false);
            return Ok(0);
        }
        let wine = win32::is_wine();
        let gamescope = std::env::var_os("GAMESCOPE_WAYLAND_DISPLAY").is_some();
        let settings_path = settings_path();
        let mut existing = read_settings(&settings_path)?;
        let show_window =
            cli.settings_window || (!cli.no_window && raw.iter().all(|arg| arg == "--linux"));
        if show_window {
            if wine {
                match gpu::prefer_glx() {
                    Ok(true) => log_line("Wine settings window: OpenGL through GLX (UseEGL=N)"),
                    Ok(false) => {}
                    Err(error) => log_line(&format!("warning: {error}")),
                }
            }
            log_line("settings window opened");
            match ui::show(&settings_path, existing.as_deref(), cli.linux) {
                Ok(ui::Outcome::Play) => {
                    log_line("settings window: Play");
                    if cli.settings_window {
                        return Ok(0);
                    }
                    existing = read_settings(&settings_path)?;
                }
                Ok(ui::Outcome::Closed) => {
                    log_line("settings window: closed");
                    return Ok(if cli.settings_window {
                        WINDOW_CLOSED
                    } else {
                        0
                    });
                }
                Err(error) => {
                    log_line(&format!("settings window unavailable: {error}"));
                    if cli.settings_window {
                        return Ok(WINDOW_UNAVAILABLE);
                    }
                }
            }
        }

        let existing = existing.unwrap_or_default();
        let (saved, warnings) = Settings::parse_file(&existing);
        for warning in warnings {
            log_line(&format!("warning: {warning}"));
        }
        let cli_values = cli.settings.clone();
        let mut settings = saved.clone().merged(cli.settings);
        let target = win32::display_target();
        if settings.resolution.is_none() {
            settings.resolution = Some(config::default_resolution(target.map(|t| t.size)));
        }
        config::validate_picture(&settings)?;
        log_line(&format!("resolution: {}", settings.resolution().value()));
        let game = find_game(&settings)?;
        let cwd = std::env::current_dir().map_err(|error| format!("current directory: {error}"))?;
        let game = runtime::game_folder(&game, &cwd)?;
        let runtime = settings
            .runtime
            .clone()
            .unwrap_or_else(|| home().join("CMR4"));
        let runtime = paths::resolve(&runtime, &cwd)?;
        settings.game = Some(game.clone());
        settings.runtime = Some(runtime.clone());
        log_line(&format!("game: {}", game.display()));
        log_line(&format!("runtime: {}", runtime.display()));
        log_line(&format!("settings: {settings:?}"));

        let (digest, patch_outcome) = validate_game(&game, settings.widescreen())?;
        log_line(&format!(
            "cmr4.exe SHA-256: {digest}; patch: {patch_outcome:?}"
        ));
        // Only --game is remembered from the command line. A resolution the
        // launcher picked from the screen is saved too, except inside
        // gamescope, whose output is cmr4-play's fallback size.
        let picked = (!gamescope).then(|| settings.resolution());
        remember(
            &settings_path,
            config::remember(&existing, &saved, &cli_values, &game, picked),
        )?;
        if cli.check {
            let text = format!(
                "Ready to play.\n\nGame folder: {}\ncmr4.exe SHA-256: {}\nWidescreen check: {:?}",
                game.display(),
                digest,
                patch_outcome
            );
            log_line("check passed");
            message(&text, false);
            return Ok(0);
        }

        let components_dir = std::env::current_exe()
            .ok()
            .and_then(|path| path.parent().map(|dir| dir.join("components")))
            .ok_or("cannot locate the launcher's components folder")?;
        components::validate(&components_dir, settings.controller(), wine)?;
        paths::check_separate(&game, &runtime)?;
        match runtime::remove_legacy_temps(&runtime) {
            Ok(removed) if !removed.is_empty() => log_line(&format!(
                "removed {} temp file(s) an interrupted older launch left",
                removed.len()
            )),
            Ok(_) => {}
            Err(error) => log_line(&format!("warning: {error}")),
        }
        let report = runtime::sync(&game, &runtime)
            .map_err(|error| format!("cannot prepare {}: {error}", runtime.display()))?;
        for warning in &report.warnings {
            log_line(&format!("warning: {warning}"));
        }
        log_line(&format!(
            "runtime synchronized: {} files copied",
            report.brought
        ));

        let runtime_exe = runtime.join(GAME_EXE);
        let mut bytes = fs::read(&runtime_exe)
            .map_err(|error| format!("cannot read {}: {error}", runtime_exe.display()))?;
        let outcome = patch::widescreen(&mut bytes, settings.widescreen())
            .map_err(|error| format!("{}: {error}", runtime_exe.display()))?;
        if matches!(outcome, Outcome::Patched | Outcome::Restored) {
            fsx::write_atomic(&runtime_exe, &bytes)
                .map_err(|error| format!("cannot write {error}"))?;
        }
        // gamescope exposes one output; elsewhere avoid a portrait primary monitor.
        let game_mode = registry::game_mode(settings.resolution(), settings.widescreen());
        let output = target.and_then(|t| t.output).filter(|_| !gamescope);
        components::install(
            &game,
            &runtime,
            &components_dir,
            &settings,
            wine,
            gamescope,
            output,
        )?;
        log_line("components and owned configuration installed");
        log_line(&format!(
            "display/render policy: Wine={wine}, gamescope={gamescope}, borderless={}, monitor={:?}, dgVoodoo output={}, game renders {}",
            wine && !gamescope,
            target.map(|t| t.size),
            output.map_or("default".into(), |n| n.to_string()),
            game_mode.value(),
        ));

        let (place, (width, height)) = win32::write_game_registry(game_mode, &runtime)?;
        if wine {
            if settings.controller() && win32::map_wine_controllers()? {
                log_line("Wine: winebus Map Controllers seeded (absent before)");
            }
        } else if let Err(error) = gpu::prefer_fast_gpu(&runtime_exe) {
            log_line(&format!("warning: {error}"));
        }
        log_line(&format!(
            "registry: {place}, {width}x{height}; Wine: {wine}"
        ));
        let options_file = runtime.join("OPTIONS");
        if settings.controller() {
            choose_gamepad(&options_file);
        }
        log_line("starting game");
        let code = win32::run_game(
            &runtime_exe,
            &runtime,
            wine,
            settings.controller(),
            &components_dir.join(components::CLOSE_HOOK_DLL),
            log_line,
        )?;
        log_line(&format!("game exited with {code}"));
        if settings.controller() {
            // A first run creates OPTIONS while playing; set it up for the next.
            choose_gamepad(&options_file);
        }
        Ok(code)
    }
}

#[cfg(windows)]
fn main() {
    match app::run() {
        Ok(code) => std::process::exit(code as i32),
        Err(error) => {
            app::log_line(&format!("error: {error}"));
            app::message(&error, true);
            std::process::exit(1);
        }
    }
}

#[cfg(not(windows))]
fn main() {
    if std::env::args().any(|arg| matches!(arg.as_str(), "--help" | "-h" | "/?")) {
        print!("{}", cmr4_launcher::config::USAGE);
    } else {
        eprintln!("CMR4Launcher is a Windows program; on Linux run it with Proton or Wine.");
        std::process::exit(2);
    }
}
