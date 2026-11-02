//! Loaded into CMR04's main thread by the launcher's `WH_GETMESSAGE` hook.
//!
//! The game's window procedure returns 0 for `WM_CLOSE` and its main loop
//! ignores `WM_QUIT`, so neither Alt+F4 nor a compositor close request
//! (which Wine turns into `SC_CLOSE`) can end it. This hook ends the process
//! when one of those requests reaches the game's own window.
//!
//! Only the main window counts: the intro movies' video window is closed
//! with `WM_CLOSE` on the same thread, and that must not end the game.

/// The game's window class (`sprintf(.., "PC_CMR4")` before RegisterClassA).
pub const GAME_CLASS: &str = "PC_CMR4";
#[cfg(test)]
const WM_CLOSE: u32 = 0x0010;
const WM_SYSKEYDOWN: u32 = 0x0104;
const WM_SYSCOMMAND: u32 = 0x0112;
const SC_CLOSE: usize = 0xF060;
const VK_F4: usize = 0x73;
/// `WM_SYSKEYDOWN` lParam bit 29: Alt was held.
const ALT_DOWN: isize = 1 << 29;

/// True for the requests a player uses to close a window.
pub fn is_close_request(message: u32, wparam: usize, lparam: isize) -> bool {
    match message {
        WM_SYSCOMMAND => wparam & 0xFFF0 == SC_CLOSE,
        WM_SYSKEYDOWN => wparam == VK_F4 && lparam & ALT_DOWN != 0,
        _ => false,
    }
}

#[cfg(windows)]
mod hook {
    use windows_sys::Win32::Foundation::HWND;
    use windows_sys::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
    use windows_sys::Win32::System::Threading::{GetCurrentProcess, TerminateProcess};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CallNextHookEx, GA_ROOT, GetAncestor, GetClassNameW, HC_ACTION, MSG, PM_REMOVE,
    };

    /// True when `hwnd` belongs to the game's main window.
    unsafe fn is_game_window(hwnd: HWND) -> bool {
        // SAFETY: plain window queries into a local buffer.
        unsafe {
            let root = GetAncestor(hwnd, GA_ROOT);
            let mut name = [0u16; 32];
            let len = GetClassNameW(
                if root.is_null() { hwnd } else { root },
                name.as_mut_ptr(),
                32,
            );
            len > 0 && String::from_utf16_lossy(&name[..len as usize]) == super::GAME_CLASS
        }
    }

    /// # Safety
    /// Called by Windows as a `WH_GETMESSAGE` hook; `lparam` points to a `MSG`.
    #[unsafe(no_mangle)]
    pub unsafe extern "system" fn close_hook(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        // SAFETY: for HC_ACTION, lparam is the MSG being removed from the queue.
        unsafe {
            if code == HC_ACTION as i32 && wparam == PM_REMOVE as usize {
                let msg = &*(lparam as *const MSG);
                if super::is_close_request(msg.message, msg.wParam, msg.lParam)
                    && is_game_window(msg.hwnd)
                {
                    // The game has no clean shutdown path to call into.
                    TerminateProcess(GetCurrentProcess(), 0);
                }
            }
            CallNextHookEx(std::ptr::null_mut(), code, wparam, lparam)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn close_requests_are_recognized() {
        assert!(is_close_request(WM_SYSCOMMAND, SC_CLOSE, 0));
        // Low four bits of SC_* are reserved and may be set.
        assert!(is_close_request(WM_SYSCOMMAND, SC_CLOSE | 0x2, 0));
        assert!(is_close_request(WM_SYSKEYDOWN, VK_F4, ALT_DOWN | 1));
    }

    #[test]
    fn other_input_is_left_alone() {
        assert!(!is_close_request(WM_SYSKEYDOWN, VK_F4, 1)); // F4 without Alt
        assert!(!is_close_request(WM_SYSKEYDOWN, 0x09, ALT_DOWN)); // Alt+Tab
        assert!(!is_close_request(WM_SYSCOMMAND, 0xF140, 0)); // SC_SCREENSAVE
        assert!(!is_close_request(0x0100, VK_F4, 0)); // WM_KEYDOWN F4
        // The movie window is closed with WM_CLOSE; only SC_CLOSE / Alt+F4 count.
        assert!(!is_close_request(WM_CLOSE, 0, 0));
    }
}
