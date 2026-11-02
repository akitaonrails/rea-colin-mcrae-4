#![cfg_attr(windows, windows_subsystem = "windows")]

#[cfg(windows)]
mod ui;
#[cfg(windows)]
mod win32;

#[cfg(windows)]
mod app {
    use std::fs;
    use std::path::{Path, PathBuf};

    use cmr4_launcher::audience;
    use cmr4_launcher::config::{self, Settings};
    use cmr4_launcher::edition::{self, Prepared};
    use cmr4_launcher::instance::{self, SecondStart};
    use cmr4_launcher::window::TITLE;
    use cmr4_launcher::{GAME_EXE, components, plan, profile, runtime};
    use rea_launcher::identify::validate_pe32;
    use rea_launcher::log::Log;
    use rea_launcher::{fsx, paths};
    use rea_launcher_win32::gpu;
    use rea_launcher_win32::report::{self, Mode};

    use crate::{ui, win32};

    const WINDOW_CLOSED: u32 = 3;
    const WINDOW_UNAVAILABLE: u32 = 4;

    fn home() -> PathBuf {
        std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir)
            .join("CMR4Launcher")
    }

    /// `launcher.log`, with the SDK's size cap (past 1 MiB the next launch
    /// starts it afresh).
    pub fn log_line(line: &str) {
        Log::new(home().join("launcher.log")).line(line);
    }

    /// How a message reaches the player in this run (see
    /// `cmr4_launcher::audience`).
    pub fn mode() -> Mode {
        let args: Vec<String> = std::env::args().skip(1).collect();
        let audience = audience::audience(&args, win32::is_wine());
        let interactive = audience.interactive;
        if audience.command {
            Mode::Command { interactive }
        } else {
            Mode::Play { interactive }
        }
    }

    /// Information for the player (`--help`, `--check`).
    pub fn note(text: &str) {
        report::note(TITLE, text, mode(), false);
    }

    /// An error the launcher stops on.
    pub fn fatal(text: &str) {
        report::fatal(TITLE, text, mode());
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

    /// The game's `cmr4.exe`, identified, with the widescreen fix when
    /// wanted and the build is known (an unknown build runs unpatched).
    fn validate_game(game: &Path, widescreen: bool) -> Result<Prepared, String> {
        let path = game.join(GAME_EXE);
        let bytes =
            fs::read(&path).map_err(|error| format!("cannot read {}: {error}", path.display()))?;
        validate_pe32(&bytes).map_err(|error| format!("{}: {error}", path.display()))?;
        Ok(edition::prepare(&bytes, widescreen))
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
            note(&config::usage());
            return Ok(0);
        }
        // Held until the game has exited: a second start brings this
        // window forward, or refuses to play while the game runs.
        let _instance = match win32::single_instance(instance::NAME) {
            Ok(guard) => guard,
            Err(_) => {
                let second = SecondStart::for_run(cli.check, || {
                    win32::bring_launcher_forward(&win32::launcher_exe_name())
                });
                log_line(second.log_line());
                return second.outcome().map_err(str::to_owned);
            }
        };
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
        let target = win32::display_target();
        let screen = target.map(|t| t.size);
        // A look sets the values its own source (file or flags) left unset.
        let saved = saved.resolve_look(screen);
        let cli_values = cli.settings.resolve_look(screen);
        let mut settings = saved.clone().merged(cli_values.clone());
        if settings.resolution.is_none() {
            settings.resolution = Some(config::default_resolution(screen));
        }
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
        log_line(&format!(
            "look: {}",
            match settings.look {
                Some(look) => format!("{} (from the file or --look)", look.value()),
                None => "none set (the values above)".to_string(),
            }
        ));

        let exe = validate_game(&game, settings.widescreen())?;
        log_line(&exe.describe());
        if exe.edition.is_none() {
            log_line(&format!(
                "warning: {}",
                edition::unknown_message(&exe.sha256)
            ));
        }
        // Only --game is remembered from the command line. A resolution the
        // launcher picked from the screen is saved too, except inside
        // gamescope, whose output is cmr4-play's fallback size.
        let picked = (!gamescope).then(|| settings.resolution());
        remember(
            &settings_path,
            config::remember(&existing, &saved, &cli_values, &game, picked),
        )?;
        if cli.check {
            let mut text = format!(
                "Ready to play.\n\nGame folder: {}\n{}",
                game.display(),
                exe.describe()
            );
            if exe.edition.is_none() {
                text = format!("{text}\n\n{}", edition::unknown_message(&exe.sha256));
            }
            log_line("check passed");
            note(&text);
            return Ok(0);
        }

        let components_dir = std::env::current_exe()
            .ok()
            .and_then(|path| path.parent().map(|dir| dir.join("components")))
            .ok_or("cannot locate the launcher's components folder")?;
        components::validate(&components_dir, settings.controller(), wine)?;
        for line in components::describe(&components_dir) {
            log_line(&format!("component {line}"));
        }
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

        // Always from the game's own bytes, so switching the fix off needs
        // no restore step.
        let runtime_exe = runtime.join(GAME_EXE);
        if fs::read(&runtime_exe).ok().as_deref() != Some(exe.bytes.as_slice()) {
            fsx::write_atomic(&runtime_exe, &exe.bytes)
                .map_err(|error| format!("cannot write {error}"))?;
        }
        // gamescope exposes one output; elsewhere avoid a portrait primary monitor.
        let output = target.and_then(|t| t.output).filter(|_| !gamescope);
        let platform = components::Platform::new(wine, gamescope, output);
        let (key, place, existing) = win32::game_registry()?;
        let inherited = win32::inherited_overrides();
        // An unknown build runs without the fix: dgVoodoo then keeps the
        // game's own frame shape.
        let plan = plan::plan(
            &settings,
            exe.widescreen,
            &runtime,
            &platform,
            existing,
            inherited.as_deref(),
        );
        components::install(
            &game,
            &runtime,
            &components_dir,
            &settings,
            exe.widescreen,
            &platform,
        )?;
        log_line(&format!(
            "components installed: {}; taken out: {}",
            plan.wrappers.join(", "),
            if plan.taken_out.is_empty() {
                "none".to_string()
            } else {
                plan.taken_out.join(", ")
            }
        ));
        log_line(&format!(
            "display/render policy: Wine={wine}, gamescope={gamescope}, borderless={}, monitor={:?}, dgVoodoo output={}, game renders {}",
            wine && !gamescope,
            target.map(|t| t.size),
            output.map_or("default".into(), |n| n.to_string()),
            plan.game_mode.value(),
        ));

        win32::write_game_registry(&key, &plan.registry)?;
        drop(key);
        if !plan.winebus.is_empty() && win32::map_wine_controllers()? {
            log_line("Wine: winebus Map Controllers seeded (absent before)");
        }
        if plan.prefer_fast_gpu
            && let Err(error) = gpu::prefer_fast_gpu(&runtime_exe)
        {
            log_line(&format!("warning: {error}"));
        }
        log_line(&format!(
            "registry: {place}, game mode {}; Wine: {wine}",
            plan.game_mode.value()
        ));
        for (name, value) in &plan.env {
            log_line(&format!("game environment: {name}={value}"));
        }
        let options_file = runtime.join("OPTIONS");
        if plan.gamepad_profile {
            choose_gamepad(&options_file);
        }
        log_line("starting game");
        let code = win32::run_game(
            &runtime_exe,
            &runtime,
            &plan.env,
            &plan.args,
            &components_dir.join(components::CLOSE_HOOK_DLL),
            log_line,
        )?;
        log_line(&format!("game exited with {code}"));
        if plan.gamepad_profile {
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
            app::fatal(&error);
            std::process::exit(1);
        }
    }
}

#[cfg(not(windows))]
fn main() {
    if std::env::args().any(|arg| matches!(arg.as_str(), "--help" | "-h" | "/?")) {
        print!("{}", cmr4_launcher::config::usage());
    } else {
        eprintln!("CMR4Launcher is a Windows program; on Linux run it with Proton or Wine.");
        std::process::exit(2);
    }
}
