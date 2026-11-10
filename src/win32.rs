//! CMR4's Windows glue: its registry values, its monitor choice and the game
//! start with the close hook. The Win32 calls themselves are
//! `rea_launcher_win32`'s.

use std::path::Path;

use cmr4_launcher::display;
use cmr4_launcher::registry::{self, KEY, RegValue, RegWrite};
use rea_launcher_win32::instance::{self, AlreadyRunning, Guard};
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
    #[allow(deprecated)] // Replaced by the HKCU key and REA_REGISTRY next.
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

/// Sets the game's per-program DLL overrides under Wine (`add`, each
/// `native,builtin` where no value exists) and takes back ours for
/// components turned off (`remove`). Wine's loader ignores
/// `WINEDLLOVERRIDES` in the game's environment block; this key it reads
/// in any prefix (plain Proton included). Returns log lines.
pub fn app_dll_overrides(exe: &str, add: &[&str], remove: &[&str]) -> Result<Vec<String>, String> {
    fn native_first<'a>(dlls: &[&'a str]) -> Vec<(&'a str, wine::DllMode)> {
        dlls.iter()
            .map(|dll| (*dll, wine::DllMode::NativeThenBuiltin))
            .collect()
    }
    let (add, remove) = (native_first(add), native_first(remove));
    let mut lines = Vec::new();
    let removed = wine::remove_app_dll_overrides(exe, &remove)?;
    if !removed.is_empty() {
        lines.push(format!(
            "Wine DLL overrides for {exe}: removed ours for {}",
            removed.join(", ")
        ));
    }
    let applied = wine::app_dll_overrides(exe, &add)?;
    if !applied.written.is_empty() {
        lines.push(format!(
            "Wine DLL overrides for {exe}: native,builtin set for {}",
            applied.written.join(", ")
        ));
    }
    if !applied.ours.is_empty() {
        lines.push(format!(
            "Wine DLL overrides for {exe}: already native,builtin for {}",
            applied.ours.join(", ")
        ));
    }
    for (dll, value) in &applied.player {
        lines.push(format!(
            "Wine DLL overrides for {exe}: {dll} kept as the player set it (\"{value}\")"
        ));
    }
    Ok(lines)
}

/// The player's own `WINEDLLOVERRIDES`, merged into the game's.
pub fn inherited_overrides() -> Option<String> {
    let vars: Vec<_> = std::env::vars_os().collect();
    rea_launcher::env::lookup(&vars, rea_launcher::wine::DLL_OVERRIDES)
        .map(|value| value.to_string_lossy().into_owned())
}

/// Takes the launcher's single-instance mutex, or reports that another
/// launcher holds it.
pub fn single_instance(name: &str) -> Result<Guard, AlreadyRunning> {
    instance::single(name)
}

/// Starts the game and waits for it. Under Wine the game's own environment
/// carries the DLL overrides, so it works without the Linux helper. CMR04
/// ignores WM_CLOSE and has no OS-level quit path, so Alt+F4 and the
/// compositor's close action do nothing: the SDK's close-hook DLL, in the
/// runtime `cmr4.exe`'s imports (`components::with_close_hook`), attaches
/// itself (`REA_CLOSE_HOOK_ATTACH=1`) and, configured by `REA_CLOSE_HOOK`
/// (both in `env`, from the plan), ends the process on those requests.
/// Nothing here touches the running game.
pub fn run_game(
    exe: &Path,
    workdir: &Path,
    env: &[(&str, String)],
    args: &[String],
) -> Result<u32, String> {
    let mut launch = Launch::new(exe).cwd(workdir);
    for arg in args {
        launch = launch.arg(arg);
    }
    for (name, value) in env {
        launch = launch.env(name, value);
    }
    launch.run()
}

#[cfg(test)]
mod tests {
    use super::*;

    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DestroyWindow, WS_OVERLAPPEDWINDOW, WS_VISIBLE,
    };

    fn wide(text: &str) -> Vec<u16> {
        text.encode_utf16().chain([0]).collect()
    }

    /// Under Wine (`scripts/wine-test`): cmr4.exe's per-program overrides
    /// round-trip through the prefix's registry, keep a player's value and
    /// take back only ours. On native Windows nothing is written.
    #[test]
    fn wine_dll_overrides_round_trip_and_keep_the_players_value() {
        use rea_launcher_win32::reg::Hive;
        let exe = "cmr4-launcher-test.exe";
        let key = Key::create(Hive::CurrentUser, &wine::app_defaults_key(exe)).unwrap();
        for dll in ["d3d9", "dinput8", "dsound"] {
            let _ = key.delete_value(dll);
        }
        if !is_wine() {
            assert!(app_dll_overrides(exe, &["d3d9"], &[]).unwrap().is_empty());
            return;
        }
        key.set_string("dsound", "builtin").unwrap();
        let first = app_dll_overrides(exe, &["dinput8", "d3d9", "dsound"], &[]).unwrap();
        assert!(
            first
                .iter()
                .any(|l| l.ends_with("native,builtin set for dinput8, d3d9")),
            "{first:?}"
        );
        assert!(
            first
                .iter()
                .any(|l| l.contains("dsound kept as the player set it")),
            "{first:?}"
        );
        assert_eq!(
            key.string("d3d9").unwrap().as_deref(),
            Some("native,builtin")
        );
        let again = app_dll_overrides(exe, &["dinput8", "d3d9"], &[]).unwrap();
        assert_eq!(again.len(), 1, "{again:?}");
        assert!(again[0].ends_with("already native,builtin for dinput8, d3d9"));
        // The gamepad turned off: Xidi's override goes, the player's stays.
        let off = app_dll_overrides(exe, &["d3d9"], &["dinput8", "dsound"]).unwrap();
        assert!(
            off.iter().any(|l| l.ends_with("removed ours for dinput8")),
            "{off:?}"
        );
        assert_eq!(key.string("dinput8").unwrap(), None);
        assert_eq!(key.string("dsound").unwrap().as_deref(), Some("builtin"));
        for dll in ["d3d9", "dinput8", "dsound"] {
            let _ = key.delete_value(dll);
        }
    }

    #[test]
    fn a_second_launcher_finds_the_first_one_and_can_start_after_it() {
        // A name of its own, so a launcher running on this machine is not
        // disturbed.
        let name = "CMR4Launcher-test-single-instance";
        let first = single_instance(name).unwrap();
        assert!(single_instance(name).is_err());
        drop(first);
        assert!(single_instance(name).is_ok());
    }

    #[test]
    fn the_second_start_finds_only_a_window_with_the_launchers_title() {
        use cmr4_launcher::window::TITLE;
        use rea_launcher_win32::instance::Window;
        // This test process stands for the running launcher (Window::Own
        // takes windows of this program's own exe). Another program's
        // same-titled window is the SDK's tested is_own_window case.
        let running = AlreadyRunning;
        assert!(!running.is_window_found(Window::Own(TITLE)));
        let (class, title) = (wide("STATIC"), wide(TITLE));
        // SAFETY: a system class and NUL-terminated strings; destroyed below.
        let hwnd = unsafe {
            CreateWindowExW(
                0,
                class.as_ptr(),
                title.as_ptr(),
                WS_OVERLAPPEDWINDOW | WS_VISIBLE,
                0,
                0,
                200,
                100,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null(),
            )
        };
        assert!(!hwnd.is_null(), "no test window");
        assert!(running.is_window_found(Window::Own(TITLE)));
        assert!(!running.is_window_found(Window::Own("Colin McRae Rally 04 (other)")));
        // A plain second start brings it forward and exits 0; --check
        // leaves it and is refused with 5.
        assert_eq!(
            running.second_start(true, Window::Own(TITLE)).exit_code(),
            5
        );
        assert_eq!(
            running.second_start(false, Window::Own(TITLE)).exit_code(),
            0
        );
        // SAFETY: the window created above, on this thread.
        unsafe { DestroyWindow(hwnd) };
        assert_eq!(
            running.second_start(false, Window::Own(TITLE)).exit_code(),
            5
        );
    }

    #[test]
    fn no_game_runs_from_a_runtime_copy_nobody_started() {
        let exe = std::env::temp_dir()
            .join("cmr4-launcher-test-runtime")
            .join("cmr4.exe");
        assert!(rea_launcher_win32::process::find_running(&exe).is_empty());
        // This test program is found by its own image.
        let me = std::env::current_exe().unwrap();
        assert!(rea_launcher_win32::process::find_running(&me).contains(&std::process::id()));
    }
}
