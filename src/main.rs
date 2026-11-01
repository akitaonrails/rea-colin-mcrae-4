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
    use cmr4_launcher::{GAME_EXE, components, pe, runtime};

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

    fn persist(path: &Path, settings: &Settings) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("cannot create {}: {error}", parent.display()))?;
        }
        fs::write(path, settings.render())
            .map_err(|error| format!("cannot write {}: {error}", path.display()))
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
        let settings_path = settings_path();
        let mut existing = fs::read_to_string(&settings_path).unwrap_or_default();
        let show_window =
            cli.settings_window || (!cli.no_window && raw.iter().all(|arg| arg == "--linux"));
        if show_window {
            log_line("settings window opened");
            match ui::show(&settings_path, &existing, cli.linux) {
                Ok(ui::Outcome::Play) => {
                    log_line("settings window: Play");
                    if cli.settings_window {
                        return Ok(0);
                    }
                    existing = fs::read_to_string(&settings_path).unwrap_or_default();
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

        let (saved, warnings) = Settings::parse_file(&existing);
        for warning in warnings {
            log_line(&format!("warning: {warning}"));
        }
        let mut settings = saved.merged(cli.settings);
        let game = find_game(&settings)?;
        let runtime = settings
            .runtime
            .clone()
            .unwrap_or_else(|| home().join("CMR4"));
        settings.game = Some(game.clone());
        settings.runtime = Some(runtime.clone());
        log_line(&format!("game: {}", game.display()));
        log_line(&format!("runtime: {}", runtime.display()));
        log_line(&format!("settings: {settings:?}"));

        let (digest, patch_outcome) = validate_game(&game, settings.widescreen())?;
        log_line(&format!(
            "cmr4.exe SHA-256: {digest}; patch: {patch_outcome:?}"
        ));
        if cli.check {
            persist(&settings_path, &settings)?;
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
        components::validate(&components_dir, settings.controller())?;
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
            fs::write(&runtime_exe, &bytes)
                .map_err(|error| format!("cannot write {}: {error}", runtime_exe.display()))?;
        }
        components::install(&game, &runtime, &components_dir, &settings)?;
        persist(&settings_path, &settings)?;
        log_line("components and owned configuration installed");

        let wine = win32::is_wine();
        let (place, (width, height)) = win32::write_game_registry(settings.resolution())?;
        if wine {
            if settings.controller() {
                win32::map_wine_controllers()?;
            }
        } else if let Err(error) = win32::prefer_fast_gpu(&runtime_exe) {
            log_line(&format!("warning: {error}"));
        }
        log_line(&format!(
            "registry: {place}, {width}x{height}; Wine: {wine}"
        ));
        log_line("starting game");
        let code = win32::run_game(&runtime_exe, &runtime, wine, settings.controller())?;
        log_line(&format!("game exited with {code}"));
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
