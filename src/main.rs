#![cfg_attr(windows, windows_subsystem = "windows")]

#[cfg(windows)]
mod ui;
#[cfg(windows)]
mod win32;

#[cfg(windows)]
mod app {
    use std::fs;
    use std::io::Write as _;
    use std::path::{Path, PathBuf};

    use cmr4_launcher::config::{self, Settings};
    use cmr4_launcher::patch::{self, Outcome};
    use cmr4_launcher::{GAME_EXE, components, pe, profile, registry, runtime};

    use crate::{ui, win32};

    const WINDOW_CLOSED: u32 = 3;
    const WINDOW_UNAVAILABLE: u32 = 4;

    fn home() -> PathBuf {
        std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir)
            .join("CMR4Launcher")
    }

    pub fn log_line(line: &str) {
        let dir = home();
        let _ = fs::create_dir_all(&dir);
        if let Ok(mut file) = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(dir.join("launcher.log"))
        {
            let _ = writeln!(file, "{line}");
        }
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
        pe::validate_pe32(&bytes).map_err(|error| format!("{}: {error}", path.display()))?;
        let mut patched = bytes.clone();
        let outcome = patch::widescreen(&mut patched, widescreen)
            .map_err(|error| format!("{}: {error}", path.display()))?;
        Ok((patch::sha256(&bytes), outcome))
    }

    /// The settings file's text, or None when there is none yet. A file that
    /// exists but can't be read stops the launch rather than being replaced.
    fn read_settings(path: &Path) -> Result<Option<String>, String> {
        match fs::read_to_string(path) {
            Ok(text) => Ok(Some(text)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(format!("cannot read {}: {error}", path.display())),
        }
    }

    /// Remembers the game folder, keeping every other line of the file.
    fn remember_game(path: &Path, text: &str, game: &Path) -> Result<(), String> {
        runtime::write_atomic(path, config::remember_game(text, game).as_bytes())
            .map_err(|error| format!("cannot write {}: {error}", path.display()))
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
            win32::message("Colin McRae Rally 04", config::USAGE, false);
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
                match win32::prefer_gpu_opengl() {
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
        let cli_game = cli.settings.game.clone();
        let mut settings = saved.clone().merged(cli.settings);
        let target = win32::display_target();
        if settings.resolution.is_none() {
            settings.resolution = Some(config::default_resolution(target.map(|t| t.size)));
        }
        config::validate_picture(&settings)?;
        log_line(&format!("resolution: {}", settings.resolution().value()));
        let game = find_game(&settings)?;
        let game = fs::canonicalize(&game)
            .map_err(|error| format!("game folder {}: {error}", game.display()))?;
        let game = runtime::simplify_extended_length(&game);
        let runtime = settings
            .runtime
            .clone()
            .unwrap_or_else(|| home().join("CMR4"));
        let cwd = std::env::current_dir().map_err(|error| format!("current directory: {error}"))?;
        let runtime = runtime::resolve(&runtime, &cwd)?;
        let runtime = runtime::simplify_extended_length(&runtime);
        // Only the game folder is remembered from the command line.
        let remember = config::should_remember_game(saved.game.as_deref(), cli_game.as_deref());
        settings.game = Some(game.clone());
        settings.runtime = Some(runtime.clone());
        log_line(&format!("game: {}", game.display()));
        log_line(&format!("runtime: {}", runtime.display()));
        log_line(&format!("settings: {settings:?}"));

        let (digest, patch_outcome) = validate_game(&game, settings.widescreen())?;
        log_line(&format!(
            "cmr4.exe SHA-256: {digest}; patch: {patch_outcome:?}"
        ));
        if remember {
            remember_game(&settings_path, &existing, &game)?;
        }
        if cli.check {
            let text = format!(
                "Ready to play.\n\nGame folder: {}\ncmr4.exe SHA-256: {}\nWidescreen check: {:?}",
                game.display(),
                digest,
                patch_outcome
            );
            log_line("check passed");
            win32::message("Colin McRae Rally 04", &text, false);
            return Ok(0);
        }

        let components_dir = std::env::current_exe()
            .ok()
            .and_then(|path| path.parent().map(|dir| dir.join("components")))
            .ok_or("cannot locate the launcher's components folder")?;
        components::validate(&components_dir, settings.controller(), wine)?;
        runtime::check_separate(&game, &runtime)?;
        let copied = runtime::sync(&game, &runtime)
            .map_err(|error| format!("cannot prepare {}: {error}", runtime.display()))?;
        log_line(&format!("runtime synchronized: {copied} files copied"));

        let runtime_exe = runtime.join(GAME_EXE);
        let mut bytes = fs::read(&runtime_exe)
            .map_err(|error| format!("cannot read {}: {error}", runtime_exe.display()))?;
        let outcome = patch::widescreen(&mut bytes, settings.widescreen())
            .map_err(|error| format!("{}: {error}", runtime_exe.display()))?;
        if matches!(outcome, Outcome::Patched | Outcome::Restored) {
            runtime::write_atomic(&runtime_exe, &bytes)
                .map_err(|error| format!("cannot write {}: {error}", runtime_exe.display()))?;
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
        } else if let Err(error) = win32::prefer_fast_gpu(&runtime_exe) {
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
            win32::message("Colin McRae Rally 04", &error, true);
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
