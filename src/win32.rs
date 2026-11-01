//! All Win32 FFI: registry, Wine detection, GPU preference, and child start.

use std::ffi::{OsStr, OsString};
use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use std::ptr::{null, null_mut};

use cmr4_launcher::components;
use cmr4_launcher::config::Resolution;
use cmr4_launcher::registry::{self, KEY, VIRTUAL_STORE_KEY, WINEBUS_KEY};
use windows_sys::Win32::Foundation::{
    CloseHandle, ERROR_ACCESS_DENIED, ERROR_FILE_NOT_FOUND, ERROR_SUCCESS,
};
use windows_sys::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress};
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
    // SAFETY: read-only loader queries with NUL-terminated names.
    unsafe {
        let ntdll = GetModuleHandleW(wide("ntdll.dll").as_ptr());
        !ntdll.is_null() && GetProcAddress(ntdll, c"wine_get_version".as_ptr().cast()).is_some()
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

pub fn write_game_registry(resolution: Resolution) -> Result<(&'static str, (u32, u32)), String> {
    let (key, place) = game_key()?;
    let existing = match (read_dword(&key, "Width")?, read_dword(&key, "Height")?) {
        (Some(width), Some(height)) => Some((width, height)),
        _ => None,
    };
    let (width, height) = registry::dimensions(resolution, existing);
    write_dword(&key, "Width", width)?;
    write_dword(&key, "Height", height)?;
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

pub fn run_game(exe: &Path, workdir: &Path, wine: bool, controller: bool) -> Result<u32, String> {
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
