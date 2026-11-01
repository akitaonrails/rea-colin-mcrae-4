#[cfg(windows)]
mod win32;

#[cfg(windows)]
mod app {
    use std::fs;
    use std::path::{Path, PathBuf};

    use cmr4_launcher::config::{self, Settings};
    use cmr4_launcher::patch::{self, Outcome};
    use cmr4_launcher::{GAME_EXE, components, pe, runtime};

    use crate::win32;

    fn home() -> PathBuf {
        std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir)
            .join("CMR4Launcher")
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
        let cli = config::parse_args(std::env::args().skip(1))?;
        if cli.help {
            print!("{}", config::USAGE);
            return Ok(0);
        }
        let settings_path = settings_path();
        let existing = fs::read_to_string(&settings_path).unwrap_or_default();
        let (saved, warnings) = Settings::parse_file(&existing);
        for warning in warnings {
            eprintln!("warning: {warning}");
        }
        let mut settings = saved.merged(cli.settings);
        let game = find_game(&settings)?;
        let runtime = settings
            .runtime
            .clone()
            .unwrap_or_else(|| home().join("CMR4"));
        settings.game = Some(game.clone());
        settings.runtime = Some(runtime.clone());

        let (digest, patch_outcome) = validate_game(&game, settings.widescreen())?;
        if cli.check {
            persist(&settings_path, &settings)?;
            println!("Ready: {}", game.display());
            println!("cmr4.exe SHA-256: {digest}");
            println!("Widescreen check: {patch_outcome:?}");
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

        let wine = win32::is_wine();
        let (place, (width, height)) = win32::write_game_registry(settings.resolution())?;
        if wine {
            if settings.controller() {
                win32::map_wine_controllers()?;
            }
        } else if let Err(error) = win32::prefer_fast_gpu(&runtime_exe) {
            eprintln!("warning: {error}");
        }
        println!(
            "Runtime: {runtime_display} ({copied} files copied); registry: {place}, {width}x{height}",
            runtime_display = runtime.display()
        );
        win32::run_game(&runtime_exe, &runtime, wine, settings.controller())
    }
}

#[cfg(windows)]
fn main() {
    match app::run() {
        Ok(code) => std::process::exit(code as i32),
        Err(error) => {
            eprintln!("error: {error}");
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
