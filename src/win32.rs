//! CMR4's Windows glue: its registry values, its monitor choice and the game
//! start with the close hook. The Win32 calls themselves are
//! `rea_launcher_win32`'s.

use std::path::Path;

use cmr4_launcher::display;
use cmr4_launcher::registry::{self, KEY, RegValue, RegWrite};
use rea_launcher_win32::process::Launch;
use rea_launcher_win32::reg::Key;
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

/// The game's open key, where it is (log label) and its Width/Height.
pub type GameKey = (Key, &'static str, Option<(u32, u32)>);

/// Opens the game's key and reads its current Width/Height.
pub fn game_registry() -> Result<GameKey, String> {
    let (key, place) = Key::machine_or_virtual_store(KEY)?;
    let existing = match (key.dword("Width")?, key.dword("Height")?) {
        (Some(width), Some(height)) => Some((width, height)),
        _ => None,
    };
    Ok((key, place.describe(), existing))
}

/// Writes the plan's values (`registry::writes`) into `key`.
pub fn write_game_registry(key: &Key, writes: &[RegWrite]) -> Result<(), String> {
    for write in writes {
        match (&write.value, write.if_missing) {
            (RegValue::String(text), false) => key.set_string(write.name, text)?,
            (RegValue::String(text), true) => {
                key.set_string_if_missing(write.name, text)?;
            }
            (RegValue::Dword(n), false) => key.set_dword(write.name, *n)?,
            (RegValue::Dword(n), true) => {
                key.seed_dwords(&[(write.name, *n)])?;
            }
        }
    }
    Ok(())
}

/// Seeds Wine's `Map Controllers` when the prefix has no value yet; a value
/// already there (the player's choice) is kept. Returns whether it wrote.
pub fn map_wine_controllers() -> Result<bool, String> {
    Ok(wine::seed_winebus(&registry::WINEBUS_DEFAULTS)?)
}

/// The player's own `WINEDLLOVERRIDES`, merged into the game's.
pub fn inherited_overrides() -> Option<String> {
    let vars: Vec<_> = std::env::vars_os().collect();
    rea_launcher::env::lookup(&vars, rea_launcher::wine::DLL_OVERRIDES)
        .map(|value| value.to_string_lossy().into_owned())
}

/// Starts the game and waits for it. Under Wine the game's own environment
/// carries the DLL overrides, so it works without the Linux helper. CMR04
/// ignores WM_CLOSE and has no OS-level quit path, so Alt+F4 and the
/// compositor's close action do nothing: the close-hook DLL, hooked into the
/// game's main thread, ends the process on those requests.
pub fn run_game(
    exe: &Path,
    workdir: &Path,
    env: &[(&str, String)],
    args: &[String],
    close_hook: &Path,
    log: impl Fn(&str),
) -> Result<u32, String> {
    let mut launch = Launch::new(exe).cwd(workdir);
    for arg in args {
        launch = launch.arg(arg);
    }
    for (name, value) in env {
        launch = launch.env(name, value);
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
