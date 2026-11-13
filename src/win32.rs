//! CMR4's Windows glue: its registry values, its monitor choice and the game
//! start with the close hook. The Win32 calls themselves are
//! `rea_launcher_win32`'s.

use std::path::Path;

use cmr4_launcher::display;
use cmr4_launcher::registry::{self, Previous};
use rea_launcher_win32::instance::{self, AlreadyRunning, Guard};
use rea_launcher_win32::process::Launch;
use rea_launcher_win32::reg::{self, Data, Hive, Key, Mismatch, Value};
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

/// What the game's key held before this launch: Width/Height from the
/// launcher's HKCU key; while that key doesn't exist yet (a first launch,
/// or the first since launcher 0.5.1), the game's own values from where it read them
/// before (the VirtualStore, else HKLM), carried over once. An old value
/// of the wrong type is not carried.
pub fn previous_values() -> Result<Previous, String> {
    previous_at(registry::USER_KEY, registry::MACHINE_KEY)
}

fn previous_at(user_key: &str, machine_key: &str) -> Result<Previous, String> {
    fn size(width: Option<u32>, height: Option<u32>) -> Option<(u32, u32)> {
        width.zip(height)
    }
    if let Some(key) = Key::open_read(Hive::CurrentUser, user_key)? {
        return Ok(Previous {
            size: size(key.dword("Width")?, key.dword("Height")?),
            carried: Vec::new(),
        });
    }
    let old = Key::read_machine(machine_key)?;
    let dword = |name| old.dword(name).ok().flatten();
    let mut carried = Vec::new();
    if let Some(text) = old.string("LANGUAGE").ok().flatten() {
        carried.push(("LANGUAGE", Data::String(text)));
    }
    for name in registry::CARRIED_DWORDS {
        if let Some(value) = dword(name) {
            carried.push((name, Data::Dword(value)));
        }
    }
    Ok(Previous {
        size: size(dword("Width"), dword("Height")),
        carried,
    })
}

/// Writes the plan's values under `HKCU\<registry::USER_KEY>` and reads
/// them back; the error lists every value the game would not see.
pub fn write_game_registry(values: &[Value]) -> Result<(), Vec<Mismatch>> {
    reg::write_verified(Hive::CurrentUser, registry::USER_KEY, values)
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
/// runtime `cmr4.exe`'s imports (`components::with_imports`), attaches
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

    use windows_sys::Win32::System::Registry::{
        HKEY, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, RegDeleteTreeW,
    };
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

    /// Deletes a test key and everything under it.
    fn delete_tree(hive: HKEY, path: &str) {
        // SAFETY: a predefined root and a NUL-terminated path.
        unsafe { RegDeleteTreeW(hive, wide(path).as_ptr()) };
    }

    /// Under `scripts/wine-test` (and on Windows, HKCU only): the old
    /// key's values are carried over once into the HKCU key, read back
    /// through `write_verified`; afterwards the HKCU key is what counts.
    #[test]
    fn the_hkcu_key_carries_old_values_over_once_and_reads_back() {
        use cmr4_launcher::config::Resolution;
        let user = r"Software\rea-cmr4-launcher-test\Colin McRae Rally 04";
        let machine = r"SOFTWARE\rea-cmr4-launcher-test\Colin McRae Rally 04";
        let wow64 = reg::windows_is_64_bit();
        let store = reg::virtual_store_path(machine, wow64);
        let clean = || {
            delete_tree(HKEY_CURRENT_USER, r"Software\rea-cmr4-launcher-test");
            delete_tree(
                HKEY_CURRENT_USER,
                &reg::virtual_store_path(r"SOFTWARE\rea-cmr4-launcher-test", wow64),
            );
            if is_wine() {
                delete_tree(HKEY_LOCAL_MACHINE, r"SOFTWARE\rea-cmr4-launcher-test");
                delete_tree(
                    HKEY_LOCAL_MACHINE,
                    r"SOFTWARE\WOW6432Node\rea-cmr4-launcher-test",
                );
            }
        };
        clean();
        // Nothing anywhere: a first run gets the defaults.
        assert_eq!(previous_at(user, machine).unwrap(), Previous::default());
        // An older launcher's VirtualStore copy (and, under Wine, HKLM).
        let old = Key::create(Hive::CurrentUser, &store).unwrap();
        old.set_string("LANGUAGE", "F").unwrap();
        old.set_dword("SPEC_SELECTION1", 2).unwrap();
        old.set_dword("Width", 800).unwrap();
        old.set_dword("Height", 600).unwrap();
        old.set_string("ADAPTER_VID", "not a dword").unwrap();
        drop(old);
        if is_wine() {
            let hklm = Key::machine(machine).unwrap();
            hklm.set_dword("ADAPTER_PID", 7).unwrap();
            hklm.set_dword("SPEC_SELECTION1", 9).unwrap();
        }
        let previous = previous_at(user, machine).unwrap();
        assert_eq!(previous.size, Some((800, 600)));
        let mut carried = vec![
            ("LANGUAGE", Data::String("F".into())),
            ("SPEC_SELECTION1", Data::Dword(2)),
        ];
        if is_wine() {
            carried.insert(1, ("ADAPTER_PID", Data::Dword(7)));
        }
        assert_eq!(previous.carried, carried, "VirtualStore first, then HKLM");
        let values = registry::values(
            Resolution::Original,
            std::path::Path::new(r"C:\CMR4"),
            &previous,
        );
        reg::write_verified(Hive::CurrentUser, user, &values).unwrap();
        let key = Key::open_read(Hive::CurrentUser, user).unwrap().unwrap();
        assert_eq!(
            key.string("INSTALL_PATH").unwrap().as_deref(),
            Some(r"C:\CMR4")
        );
        assert_eq!(key.string("LANGUAGE").unwrap().as_deref(), Some("F"));
        assert_eq!(key.dword("SPEC_SELECTION1").unwrap(), Some(2));
        assert_eq!(key.dword("Width").unwrap(), Some(800));
        assert_eq!(key.dword("ADAPTER_VID").unwrap(), Some(0));
        // The game changes its own values; the next launch keeps them and
        // no longer looks at the old key.
        let game = Key::create(Hive::CurrentUser, user).unwrap();
        game.set_string("LANGUAGE", "G").unwrap();
        game.set_dword("Width", 1280).unwrap();
        game.set_dword("Height", 960).unwrap();
        drop(game);
        let again = previous_at(user, machine).unwrap();
        assert_eq!(again.size, Some((1280, 960)));
        assert!(again.carried.is_empty());
        let values = registry::values(
            Resolution::Custom(1440, 1080),
            std::path::Path::new(r"C:\CMR4"),
            &again,
        );
        reg::write_verified(Hive::CurrentUser, user, &values).unwrap();
        assert_eq!(key.string("LANGUAGE").unwrap().as_deref(), Some("G"));
        assert_eq!(key.dword("Width").unwrap(), Some(1440));
        drop(key);
        clean();
        assert!(Key::open_read(Hive::CurrentUser, user).unwrap().is_none());
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
