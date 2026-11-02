//! All Win32 FFI: registry, Wine detection, GPU preference, and child start.

use std::ffi::{OsStr, OsString};
use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use std::ptr::{null, null_mut};

use cmr4_launcher::components;
use cmr4_launcher::config::Resolution;
use cmr4_launcher::display;
use cmr4_launcher::registry::{self, KEY, VIRTUAL_STORE_KEY, WINEBUS_KEY};
use windows_sys::Win32::Foundation::{
    CloseHandle, ERROR_ACCESS_DENIED, ERROR_FILE_NOT_FOUND, ERROR_SUCCESS,
};
use windows_sys::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress, LoadLibraryW};
use windows_sys::Win32::System::Registry::{
    HKEY, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_QUERY_VALUE, KEY_SET_VALUE, KEY_WOW64_32KEY,
    REG_DWORD, REG_OPTION_NON_VOLATILE, REG_SZ, RegCloseKey, RegCreateKeyExW, RegQueryValueExW,
    RegSetValueExW,
};
use windows_sys::Win32::System::Threading::{
    CREATE_UNICODE_ENVIRONMENT, CreateProcessW, GetExitCodeProcess, INFINITE, PROCESS_INFORMATION,
    STARTUPINFOW, WaitForSingleObject,
};

fn wide(value: impl AsRef<OsStr>) -> Vec<u16> {
    value.as_ref().encode_wide().chain(Some(0)).collect()
}

fn last_error() -> std::io::Error {
    std::io::Error::last_os_error()
}

pub fn is_wine() -> bool {
    // Some Wine builds hide `wine_get_version`, so also accept the
    // environment our Linux launcher and Proton always set.
    // SAFETY: read-only loader queries with NUL-terminated names.
    let exported = unsafe {
        let ntdll = GetModuleHandleW(wide("ntdll.dll").as_ptr());
        !ntdll.is_null() && GetProcAddress(ntdll, c"wine_get_version".as_ptr().cast()).is_some()
    };
    exported
        || ["WINEPREFIX", "WINELOADER", "STEAM_COMPAT_DATA_PATH"]
            .into_iter()
            .any(|name| std::env::var_os(name).is_some())
}

pub fn message(title: &str, text: &str, error: bool) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        MB_ICONERROR, MB_ICONINFORMATION, MB_OK, MessageBoxW,
    };
    let icon = if error {
        MB_ICONERROR
    } else {
        MB_ICONINFORMATION
    };
    // SAFETY: NUL-terminated strings and no owner window.
    unsafe {
        MessageBoxW(
            null_mut(),
            wide(text).as_ptr(),
            wide(title).as_ptr(),
            MB_OK | icon,
        );
    }
}

/// The newest available XInput function, or None when XInput is unavailable.
fn xinput_proc(name: &std::ffi::CStr) -> Option<unsafe extern "system" fn() -> isize> {
    for dll in ["xinput1_4.dll", "xinput1_3.dll", "xinput9_1_0.dll"] {
        // SAFETY: plain loader calls with NUL-terminated names.
        unsafe {
            let module = LoadLibraryW(wide(dll).as_ptr());
            if !module.is_null()
                && let Some(proc) = GetProcAddress(module, name.as_ptr().cast())
            {
                return Some(proc);
            }
        }
    }
    None
}

fn xinput_states() -> Vec<(u32, [u8; 16])> {
    type GetState = unsafe extern "system" fn(u32, *mut [u8; 16]) -> u32;
    let Some(get) = xinput_proc(c"XInputGetState") else {
        return Vec::new();
    };
    // SAFETY: XInputGetState's signature; its output state is exactly 16 bytes.
    let get: GetState = unsafe { std::mem::transmute(get) };
    (0..4)
        .filter_map(|slot| {
            let mut state = [0u8; 16];
            (unsafe { get(slot, &mut state) } == 0).then_some((slot, state))
        })
        .collect()
}

/// USB vendor and product IDs of the pad in XInput slot 0, the one Xidi
/// presents as controller 1. Uses XInputGetCapabilitiesEx (xinput1_4
/// ordinal 108), which Windows 10+ and Wine provide.
pub fn gamepad_ids() -> Option<(u16, u16)> {
    type CapabilitiesEx = unsafe extern "system" fn(u32, u32, u32, *mut [u8; 32]) -> u32;
    // SAFETY: loader calls; ordinal 108's signature fills a 32-byte
    // XINPUT_CAPABILITIES_EX whose VendorId/ProductId sit at bytes 20 and 22.
    unsafe {
        let module = LoadLibraryW(wide("xinput1_4.dll").as_ptr());
        if module.is_null() {
            return None;
        }
        let proc = GetProcAddress(module, 108 as *const u8)?;
        let get: CapabilitiesEx = std::mem::transmute(proc);
        let mut caps = [0u8; 32];
        if get(1, 0, 0, &mut caps) != 0 {
            return None;
        }
        let vendor = u16::from_le_bytes([caps[20], caps[21]]);
        let product = u16::from_le_bytes([caps[22], caps[23]]);
        (vendor != 0 && product != 0).then_some((vendor, product))
    }
}

pub fn xinput_pads() -> Vec<u32> {
    xinput_states().into_iter().map(|(slot, _)| slot).collect()
}

pub fn xinput_buttons() -> u16 {
    xinput_states().into_iter().fold(0, |all, (_, state)| {
        all | u16::from_le_bytes([state[4], state[5]])
    })
}

/// Monitors in enumeration order, sized by their current physical mode so
/// desktop scaling does not shrink them.
pub fn monitors() -> Vec<display::Monitor> {
    use windows_sys::Win32::Foundation::{LPARAM, RECT};
    use windows_sys::Win32::Graphics::Gdi::{
        DEVMODEW, ENUM_CURRENT_SETTINGS, EnumDisplayMonitors, EnumDisplaySettingsW,
        GetMonitorInfoW, HDC, HMONITOR, MONITORINFOEXW,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::MONITORINFOF_PRIMARY;

    unsafe extern "system" fn visit(monitor: HMONITOR, _: HDC, _: *mut RECT, data: LPARAM) -> i32 {
        // SAFETY: `data` is the Vec passed below and outlives the enumeration;
        // the info and mode structs are sized before the calls fill them.
        unsafe {
            let found = &mut *(data as *mut Vec<display::Monitor>);
            let mut info: MONITORINFOEXW = std::mem::zeroed();
            info.monitorInfo.cbSize = size_of::<MONITORINFOEXW>() as u32;
            if GetMonitorInfoW(monitor, (&raw mut info).cast()) == 0 {
                return 1;
            }
            let rect = info.monitorInfo.rcMonitor;
            let mut mode: DEVMODEW = std::mem::zeroed();
            mode.dmSize = size_of::<DEVMODEW>() as u16;
            let (width, height) =
                if EnumDisplaySettingsW(info.szDevice.as_ptr(), ENUM_CURRENT_SETTINGS, &mut mode)
                    != 0
                {
                    (mode.dmPelsWidth, mode.dmPelsHeight)
                } else {
                    (
                        (rect.right - rect.left) as u32,
                        (rect.bottom - rect.top) as u32,
                    )
                };
            if width > 0 && height > 0 {
                found.push(display::Monitor {
                    width,
                    height,
                    primary: info.monitorInfo.dwFlags & MONITORINFOF_PRIMARY != 0,
                });
            }
            1
        }
    }

    let mut found = Vec::new();
    // SAFETY: the callback only writes into `found` during this call.
    unsafe {
        EnumDisplayMonitors(null_mut(), null(), Some(visit), (&raw mut found) as LPARAM);
    }
    found
}

/// The monitor the game will fill and its dgVoodoo output number.
pub fn display_target() -> Option<display::Target> {
    display::choose(&monitors()).or_else(|| {
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN,
        };
        // SAFETY: plain metric queries.
        let (width, height) =
            unsafe { (GetSystemMetrics(SM_CXSCREEN), GetSystemMetrics(SM_CYSCREEN)) };
        (width > 0 && height > 0).then_some(display::Target {
            size: (width as u32, height as u32),
            output: None,
        })
    })
}

pub fn screen_size() -> Option<(u32, u32)> {
    display_target().map(|target| target.size)
}

pub fn pick_folder(title: &str) -> Option<std::path::PathBuf> {
    use std::os::windows::ffi::OsStringExt;
    use windows_sys::Win32::System::Com::{
        COINIT_APARTMENTTHREADED, CoInitializeEx, CoTaskMemFree,
    };
    use windows_sys::Win32::UI::Shell::{
        BIF_NEWDIALOGSTYLE, BIF_RETURNONLYFSDIRS, BROWSEINFOW, SHBrowseForFolderW,
        SHGetPathFromIDListW,
    };
    let mut title = wide(title);
    let mut name = [0u16; 260];
    // SAFETY: buffers live across the modal call; the returned item list is
    // released with the COM allocator.
    unsafe {
        CoInitializeEx(std::ptr::null(), COINIT_APARTMENTTHREADED as u32);
        let info = BROWSEINFOW {
            hwndOwner: std::ptr::null_mut(),
            pidlRoot: std::ptr::null_mut(),
            pszDisplayName: name.as_mut_ptr(),
            lpszTitle: title.as_mut_ptr(),
            ulFlags: BIF_RETURNONLYFSDIRS | BIF_NEWDIALOGSTYLE,
            lpfn: None,
            lParam: 0,
            iImage: 0,
        };
        let list = SHBrowseForFolderW(&info);
        if list.is_null() {
            return None;
        }
        let mut path = [0u16; 1024];
        let ok = SHGetPathFromIDListW(list, path.as_mut_ptr()) != 0;
        CoTaskMemFree(list as *const _);
        let len = path.iter().position(|&char| char == 0).unwrap_or(0);
        (ok && len > 0).then(|| std::ffi::OsString::from_wide(&path[..len]).into())
    }
}

struct Key(HKEY);

impl Drop for Key {
    fn drop(&mut self) {
        // SAFETY: this wrapper owns one open registry key.
        unsafe {
            RegCloseKey(self.0);
        }
    }
}

fn create_key(root: HKEY, path: &str, wow32: bool) -> Result<Key, u32> {
    let mut key: HKEY = null_mut();
    let access = KEY_QUERY_VALUE | KEY_SET_VALUE | if wow32 { KEY_WOW64_32KEY } else { 0 };
    // SAFETY: output key points to valid storage and strings are NUL-terminated.
    let result = unsafe {
        RegCreateKeyExW(
            root,
            wide(path).as_ptr(),
            0,
            null(),
            REG_OPTION_NON_VOLATILE,
            access,
            null(),
            &mut key,
            null_mut(),
        )
    };
    if result == ERROR_SUCCESS {
        Ok(Key(key))
    } else {
        Err(result)
    }
}

fn game_key() -> Result<(Key, &'static str), String> {
    match create_key(HKEY_LOCAL_MACHINE, KEY, true) {
        Ok(key) => Ok((key, "HKLM 32-bit view")),
        Err(ERROR_ACCESS_DENIED) => Ok((
            create_key(HKEY_CURRENT_USER, VIRTUAL_STORE_KEY, false)
                .map_err(|code| format!("cannot open HKCU VirtualStore key (error {code})"))?,
            "HKCU VirtualStore",
        )),
        Err(code) => Err(format!("cannot open CMR4 registry key (error {code})")),
    }
}

fn read_dword(key: &Key, name: &str) -> Result<Option<u32>, String> {
    let name_wide = wide(name);
    let mut kind = 0u32;
    let mut value = 0u32;
    let mut size = size_of::<u32>() as u32;
    // SAFETY: the key is open and output pointers refer to sized local values.
    let result = unsafe {
        RegQueryValueExW(
            key.0,
            name_wide.as_ptr(),
            null(),
            &mut kind,
            (&mut value as *mut u32).cast(),
            &mut size,
        )
    };
    match result {
        ERROR_SUCCESS if kind == REG_DWORD && size == 4 => Ok(Some(value)),
        ERROR_SUCCESS => Err(format!("registry value {name} is not a DWORD")),
        ERROR_FILE_NOT_FOUND => Ok(None),
        code => Err(format!("cannot read registry value {name} (error {code})")),
    }
}

fn write_dword(key: &Key, name: &str, value: u32) -> Result<(), String> {
    let name = wide(name);
    // SAFETY: the key is open and the value points to four readable bytes.
    let result = unsafe {
        RegSetValueExW(
            key.0,
            name.as_ptr(),
            0,
            REG_DWORD,
            (&value as *const u32).cast(),
            4,
        )
    };
    if result == ERROR_SUCCESS {
        Ok(())
    } else {
        Err(format!("cannot write registry DWORD (error {result})"))
    }
}

fn write_string(key: &Key, name: &str, value: &str) -> Result<(), String> {
    let name_wide = wide(name);
    let value_wide = wide(value);
    // SAFETY: the key is open and both UTF-16 buffers are NUL-terminated.
    let result = unsafe {
        RegSetValueExW(
            key.0,
            name_wide.as_ptr(),
            0,
            REG_SZ,
            value_wide.as_ptr().cast(),
            (value_wide.len() * 2) as u32,
        )
    };
    if result == ERROR_SUCCESS {
        Ok(())
    } else {
        Err(format!("cannot write registry string (error {result})"))
    }
}

fn set_user_value_if_missing(path: &str, name: &str, data: &str) -> Result<bool, String> {
    let key = create_key(HKEY_CURRENT_USER, path, false)
        .map_err(|code| format!("cannot open HKCU\\{path} (error {code})"))?;
    let wide_name = wide(name);
    // SAFETY: existence query on an open key; no output buffer is requested.
    let result = unsafe {
        RegQueryValueExW(
            key.0,
            wide_name.as_ptr(),
            null(),
            null_mut(),
            null_mut(),
            null_mut(),
        )
    };
    if result == ERROR_FILE_NOT_FOUND {
        write_string(&key, name, data)?;
        return Ok(true);
    }
    if result != ERROR_SUCCESS {
        return Err(format!(
            "cannot read registry value {name} (error {result})"
        ));
    }
    Ok(false)
}

/// Keeps the settings window on the GPU through GLX under Wine, without
/// replacing an existing user choice.
pub fn prefer_gpu_opengl() -> Result<bool, String> {
    set_user_value_if_missing(r"Software\Wine\X11 Driver", "UseEGL", "N")
}

fn has_value(key: &Key, name: &str) -> Result<bool, String> {
    let name = wide(name);
    // SAFETY: existence query on an open key; no output buffer is requested.
    let result = unsafe {
        RegQueryValueExW(
            key.0,
            name.as_ptr(),
            null(),
            null_mut(),
            null_mut(),
            null_mut(),
        )
    };
    match result {
        ERROR_SUCCESS => Ok(true),
        ERROR_FILE_NOT_FOUND => Ok(false),
        code => Err(format!("cannot read registry value (error {code})")),
    }
}

pub fn write_game_registry(
    resolution: Resolution,
    runtime: &Path,
) -> Result<(&'static str, (u32, u32)), String> {
    let (key, place) = game_key()?;
    let root = registry::install_root(runtime);
    for name in registry::INSTALL_PATH_VALUES {
        write_string(&key, name, &root)?;
    }
    let existing = match (read_dword(&key, "Width")?, read_dword(&key, "Height")?) {
        (Some(width), Some(height)) => Some((width, height)),
        _ => None,
    };
    let (width, height) = registry::dimensions(resolution, existing);
    write_dword(&key, "Width", width)?;
    write_dword(&key, "Height", height)?;
    if !has_value(&key, "LANGUAGE")? {
        write_string(&key, "LANGUAGE", "E")?;
    }
    for (name, value) in registry::DEFAULT_DWORDS {
        if !has_value(&key, name)? {
            write_dword(&key, name, value)?;
        }
    }
    Ok((place, (width, height)))
}

pub fn map_wine_controllers() -> Result<(), String> {
    let key = create_key(HKEY_LOCAL_MACHINE, WINEBUS_KEY, false)
        .map_err(|code| format!("cannot open Wine winebus key (error {code})"))?;
    write_dword(&key, "Map Controllers", 1)
}

/// Selects Windows' high-performance GPU without replacing an existing user choice.
pub fn prefer_fast_gpu(exe: &Path) -> Result<bool, String> {
    let key = create_key(
        HKEY_CURRENT_USER,
        r"Software\Microsoft\DirectX\UserGpuPreferences",
        false,
    )
    .map_err(|code| format!("cannot open GPU preference key (error {code})"))?;
    let name = exe.display().to_string();
    let name_wide = wide(&name);
    // SAFETY: existence query on an open key; no output buffer is requested.
    let result = unsafe {
        RegQueryValueExW(
            key.0,
            name_wide.as_ptr(),
            null(),
            null_mut(),
            null_mut(),
            null_mut(),
        )
    };
    if result == ERROR_FILE_NOT_FOUND {
        write_string(&key, &name, "GpuPreference=2;")?;
        return Ok(true);
    }
    if result != ERROR_SUCCESS {
        return Err(format!("cannot read GPU preference (error {result})"));
    }
    Ok(false)
}

fn environment_block(wine: bool, controller: bool) -> Vec<u16> {
    let mut environment: Vec<(OsString, OsString)> = std::env::vars_os().collect();
    if wine {
        let existing = environment
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case("WINEDLLOVERRIDES"))
            .map(|(_, value)| value.to_string_lossy().into_owned());
        environment.retain(|(name, _)| !name.eq_ignore_ascii_case("WINEDLLOVERRIDES"));
        environment.push((
            OsString::from("WINEDLLOVERRIDES"),
            OsString::from(components::wine_overrides(existing.as_deref(), controller)),
        ));
    }
    environment.sort_by_key(|(name, _)| name.to_string_lossy().to_uppercase());
    let mut block = Vec::new();
    for (name, value) in environment {
        block.extend(name.encode_wide());
        block.push('=' as u16);
        block.extend(value.encode_wide());
        block.push(0);
    }
    block.push(0);
    block
}

struct Child(PROCESS_INFORMATION);

impl Drop for Child {
    fn drop(&mut self) {
        // SAFETY: these handles were returned by CreateProcessW and are owned here.
        unsafe {
            CloseHandle(self.0.hThread);
            CloseHandle(self.0.hProcess);
        }
    }
}

/// Keeps the close hook installed for as long as the game runs.
struct CloseHook(windows_sys::Win32::UI::WindowsAndMessaging::HHOOK);

impl Drop for CloseHook {
    fn drop(&mut self) {
        // SAFETY: the handle came from SetWindowsHookExW and is released once.
        unsafe {
            windows_sys::Win32::UI::WindowsAndMessaging::UnhookWindowsHookEx(self.0);
        }
    }
}

/// CMR04 ignores WM_CLOSE and has no OS-level quit path, so Alt+F4 and the
/// compositor's close action do nothing. This hook DLL, loaded into the game's
/// main thread, ends the process on those requests.
fn install_close_hook(dll: &Path, thread: u32) -> Result<CloseHook, String> {
    use windows_sys::Win32::UI::WindowsAndMessaging::{SetWindowsHookExW, WH_GETMESSAGE};
    // SAFETY: NUL-terminated path and export name; the module stays loaded
    // for the launcher's lifetime, which outlives the hook.
    unsafe {
        let module = LoadLibraryW(wide(dll).as_ptr());
        if module.is_null() {
            return Err(format!("cannot load {}: {}", dll.display(), last_error()));
        }
        let Some(proc) = GetProcAddress(module, c"close_hook".as_ptr().cast()) else {
            return Err(format!("{} has no close_hook export", dll.display()));
        };
        let hook = SetWindowsHookExW(
            WH_GETMESSAGE,
            Some(std::mem::transmute::<
                unsafe extern "system" fn() -> isize,
                unsafe extern "system" fn(i32, usize, isize) -> isize,
            >(proc)),
            module,
            thread,
        );
        if hook.is_null() {
            return Err(format!("cannot install close hook: {}", last_error()));
        }
        Ok(CloseHook(hook))
    }
}

pub fn run_game(
    exe: &Path,
    workdir: &Path,
    wine: bool,
    controller: bool,
    close_hook: &Path,
    log: impl Fn(&str),
) -> Result<u32, String> {
    let application = wide(exe);
    let mut command = wide(format!("\"{}\"", exe.display()));
    let directory = wide(workdir);
    let environment = environment_block(wine, controller);
    let mut startup: STARTUPINFOW = unsafe { std::mem::zeroed() };
    startup.cb = size_of::<STARTUPINFOW>() as u32;
    let mut info: PROCESS_INFORMATION = unsafe { std::mem::zeroed() };
    // SAFETY: all pointers refer to live buffers; the child owns returned handles.
    let created = unsafe {
        CreateProcessW(
            application.as_ptr(),
            command.as_mut_ptr(),
            null(),
            null(),
            0,
            if wine { CREATE_UNICODE_ENVIRONMENT } else { 0 },
            if wine {
                environment.as_ptr().cast()
            } else {
                null()
            },
            directory.as_ptr(),
            &startup,
            &mut info,
        )
    };
    if created == 0 {
        return Err(format!("cannot start {}: {}", exe.display(), last_error()));
    }
    let child = Child(info);
    let _hook = match install_close_hook(close_hook, child.0.dwThreadId) {
        Ok(hook) => {
            log("close hook installed (Alt+F4 and window close end the game)");
            Some(hook)
        }
        Err(error) => {
            log(&format!("warning: {error}"));
            None
        }
    };
    // SAFETY: the process handle remains valid until `child` is dropped.
    unsafe {
        WaitForSingleObject(child.0.hProcess, INFINITE);
        let mut code = 0u32;
        if GetExitCodeProcess(child.0.hProcess, &mut code) == 0 {
            return Err(format!("cannot read game exit status: {}", last_error()));
        }
        Ok(code)
    }
}
