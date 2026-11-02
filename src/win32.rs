//! CMR4's Windows glue: its registry values, its monitor choice and the game
//! start with the close hook. The Win32 calls themselves are
//! `rea_launcher_win32`'s.

use std::path::Path;

use cmr4_launcher::components::CLOSE_HOOK_CONFIG;
use cmr4_launcher::display;
use cmr4_launcher::instance::is_launcher_window;
use cmr4_launcher::registry::{self, KEY, RegValue, RegWrite};
use cmr4_launcher::window::TITLE;
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

/// Takes the launcher's single-instance mutex, or reports that another
/// launcher holds it.
pub fn single_instance(name: &str) -> Result<Guard, AlreadyRunning> {
    instance::single(name)
}

/// This launcher's file name (`CMR4Launcher.exe` unless renamed).
pub fn launcher_exe_name() -> String {
    std::env::current_exe()
        .ok()
        .and_then(|path| {
            path.file_name()
                .map(|name| name.to_string_lossy().into_owned())
        })
        .unwrap_or_else(|| "CMR4Launcher.exe".to_owned())
}

/// Brings the running launcher's settings window forward: a top-level
/// window that [`cmr4_launcher::instance::is_launcher_window`] accepts,
/// in a process running `launcher_exe`. False when there is none (it is
/// the game that is running).
pub fn bring_launcher_forward(launcher_exe: &str) -> bool {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        IsIconic, SW_RESTORE, SetForegroundWindow, ShowWindow,
    };
    let Some(hwnd) = launcher_window(launcher_exe) else {
        return false;
    };
    // SAFETY: window calls on a handle EnumWindows just returned.
    unsafe {
        if IsIconic(hwnd) != 0 {
            ShowWindow(hwnd, SW_RESTORE);
        }
        SetForegroundWindow(hwnd);
    }
    true
}

fn launcher_window(launcher_exe: &str) -> Option<windows_sys::Win32::Foundation::HWND> {
    use windows_sys::Win32::Foundation::{CloseHandle, HWND, LPARAM};
    use windows_sys::Win32::System::Threading::{
        OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
        QueryFullProcessImageNameW,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetWindowTextW, GetWindowThreadProcessId, IsWindowVisible,
    };

    struct Search<'a> {
        launcher_exe: &'a str,
        found: Option<HWND>,
    }

    fn image(pid: u32) -> Option<String> {
        // SAFETY: a query-only handle, closed before returning; the buffer
        // and its length outlive the call.
        unsafe {
            let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
            if process.is_null() {
                return None;
            }
            let mut buffer = [0u16; 1024];
            let mut len = buffer.len() as u32;
            let ok = QueryFullProcessImageNameW(
                process,
                PROCESS_NAME_WIN32,
                buffer.as_mut_ptr(),
                &mut len,
            );
            CloseHandle(process);
            (ok != 0).then(|| String::from_utf16_lossy(&buffer[..len as usize]))
        }
    }

    unsafe extern "system" fn visit(hwnd: HWND, data: LPARAM) -> i32 {
        // SAFETY: `data` is the Search below, alive for the enumeration;
        // the title buffer outlives GetWindowTextW.
        unsafe {
            let search = &mut *(data as *mut Search);
            if IsWindowVisible(hwnd) == 0 {
                return 1;
            }
            let mut title = [0u16; 128];
            let len = GetWindowTextW(hwnd, title.as_mut_ptr(), title.len() as i32);
            let title = String::from_utf16_lossy(&title[..len.max(0) as usize]);
            if title != TITLE {
                return 1;
            }
            let mut pid = 0u32;
            GetWindowThreadProcessId(hwnd, &mut pid);
            match image(pid) {
                Some(image) if is_launcher_window(&title, &image, search.launcher_exe) => {
                    search.found = Some(hwnd);
                    0
                }
                _ => 1,
            }
        }
    }

    let mut search = Search {
        launcher_exe,
        found: None,
    };
    // SAFETY: the callback only touches `search` during this call.
    unsafe {
        EnumWindows(Some(visit), (&raw mut search) as LPARAM);
    }
    search.found
}

/// Starts the game and waits for it. Under Wine the game's own environment
/// carries the DLL overrides, so it works without the Linux helper. CMR04
/// ignores WM_CLOSE and has no OS-level quit path, so Alt+F4 and the
/// compositor's close action do nothing: the SDK's close-hook DLL, hooked
/// into the game's main thread and configured by `REA_CLOSE_HOOK`
/// ([`CLOSE_HOOK_CONFIG`]), ends the process on those requests.
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
    let game = launch.env(rea_close_hook::ENV, CLOSE_HOOK_CONFIG).spawn()?;
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

#[cfg(test)]
mod tests {
    use super::*;

    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DestroyWindow, WS_OVERLAPPEDWINDOW, WS_VISIBLE,
    };

    fn wide(text: &str) -> Vec<u16> {
        text.encode_utf16().chain([0]).collect()
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
    fn a_window_with_the_launchers_title_in_another_program_is_not_the_launcher() {
        // This test process stands for "another program" (an Explorer
        // window on a folder named like the game, or the game itself)
        // showing a window titled like the launcher's.
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
        // Not CMR4Launcher.exe: no launcher window, so a second start
        // says the game is running instead of exiting silently.
        assert!(!bring_launcher_forward("CMR4Launcher.exe"));
        // The same window in the launcher's own process is found.
        assert_eq!(launcher_window(&launcher_exe_name()), Some(hwnd));
        // SAFETY: the window created above, on this thread.
        unsafe { DestroyWindow(hwnd) };
    }
}
