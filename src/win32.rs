//! CMR4's Windows glue: its registry values, its monitor choice and the game
//! start with the close hook. The Win32 calls themselves are
//! `rea_launcher_win32`'s.

use std::path::Path;

use cmr4_launcher::components;
use cmr4_launcher::config::Resolution;
use cmr4_launcher::display;
use cmr4_launcher::registry::{self, KEY};
use rea_launcher_win32::process::Launch;
use rea_launcher_win32::reg::{Key, Place};
use rea_launcher_win32::{display as screens, wine};

pub use rea_launcher_win32::display::wine_dpi_scale;
pub use rea_launcher_win32::wine::is_wine;

/// The monitor the game will fill and its dgVoodoo output number.
pub fn display_target() -> Option<display::Target> {
    display::choose(&screens::monitors())
        .or_else(|| screens::screen_size().map(|size| display::Target { size, output: None }))
}

/// The size of the monitor the game will fill.
pub fn screen_size() -> Option<(u32, u32)> {
    display_target().map(|target| target.size)
}

/// USB vendor and product IDs of the pad in XInput slot 0, the one Xidi
/// presents as controller 1.
pub fn gamepad_ids() -> Option<(u16, u16)> {
    rea_launcher_win32::xinput::ids(0)
}

pub fn write_game_registry(
    resolution: Resolution,
    runtime: &Path,
) -> Result<(&'static str, (u32, u32)), String> {
    let (key, place) = Key::machine_or_virtual_store(KEY)?;
    let place = match place {
        Place::Machine => "HKLM 32-bit view",
        Place::VirtualStore => "HKCU VirtualStore",
    };
    let root = registry::install_root(runtime);
    for name in registry::INSTALL_PATH_VALUES {
        key.set_string(name, &root)?;
    }
    let existing = match (key.dword("Width")?, key.dword("Height")?) {
        (Some(width), Some(height)) => Some((width, height)),
        _ => None,
    };
    let (width, height) = registry::dimensions(resolution, existing);
    key.set_dword("Width", width)?;
    key.set_dword("Height", height)?;
    key.set_string_if_missing("LANGUAGE", "E")?;
    key.seed_dwords(&registry::DEFAULT_DWORDS)?;
    Ok((place, (width, height)))
}

/// Seeds Wine's `Map Controllers` when the prefix has no value yet; a value
/// already there (the player's choice) is kept. Returns whether it wrote.
pub fn map_wine_controllers() -> Result<bool, String> {
    Ok(wine::seed_winebus(&registry::WINEBUS_DEFAULTS)?)
}

/// Starts the game and waits for it. Under Wine the game's own environment
/// carries the DLL overrides, so it works without the Linux helper. CMR04
/// ignores WM_CLOSE and has no OS-level quit path, so Alt+F4 and the
/// compositor's close action do nothing: the close-hook DLL, hooked into the
/// game's main thread, ends the process on those requests.
pub fn run_game(
    exe: &Path,
    workdir: &Path,
    wine: bool,
    controller: bool,
    close_hook: &Path,
    log: impl Fn(&str),
) -> Result<u32, String> {
    let mut launch = Launch::new(exe).cwd(workdir);
    if wine {
        let vars: Vec<_> = std::env::vars_os().collect();
        let existing = rea_launcher::env::lookup(&vars, rea_launcher::wine::DLL_OVERRIDES)
            .map(|value| value.to_string_lossy().into_owned());
        launch = launch.env(
            rea_launcher::wine::DLL_OVERRIDES,
            components::wine_overrides(existing.as_deref(), controller),
        );
    }
    let game = launch.spawn()?;
    let _hook = match game.message_hook(close_hook, c"close_hook") {
        Ok(hook) => {
            log("close hook installed (Alt+F4 and window close end the game)");
            Some(hook)
        }
        Err(error) => {
            log(&format!("warning: {error}"));
            None
        }
    };
    game.wait()
}
