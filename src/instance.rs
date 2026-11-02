//! One launcher at a time (`sdk/docs/STANDARDS.md`, "Clean exit, single
//! instance"). The launcher holds a named mutex
//! (`rea_launcher_win32::instance::single`) from the start until the game
//! it started has exited, so a second start can neither open a second
//! window nor start the game again from the same runtime.

use crate::window::TITLE;

/// The instance name: `Local\rea-launcher-CMR4Launcher`.
pub const NAME: &str = "CMR4Launcher";

/// What a second start tells the player when there was no launcher window
/// to bring forward: the first launcher is waiting for the game.
pub const GAME_RUNNING: &str = "Colin McRae Rally 04 is already running. Quit the game (Alt+F4 works), then start the launcher again.";

/// Whether a top-level window is the running launcher's settings window:
/// its title is the launcher's and its process runs `launcher_exe` (the
/// launcher's own file name, any case). A title alone also matched an
/// Explorer window open on a folder named "Colin McRae Rally 04" and the
/// game's own window, so a second start "found" one of those and exited
/// silently. eframe gives every window winit's generic class, so the class
/// can't tell either.
/// `process_image` is the process's full path as Windows reports it.
pub fn is_launcher_window(title: &str, process_image: &str, launcher_exe: &str) -> bool {
    let name = process_image.rsplit(['\\', '/']).next().unwrap_or_default();
    title == TITLE && name.eq_ignore_ascii_case(launcher_exe)
}

/// What a second start does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecondStart {
    /// The running launcher's window came to the front; exit quietly.
    Focused,
    /// No window to show: the game is running. Say so and refuse to play.
    GameRunning,
}

impl SecondStart {
    /// `window_found`: whether the running launcher's window was brought
    /// forward ([`is_launcher_window`]).
    pub fn new(window_found: bool) -> SecondStart {
        if window_found {
            SecondStart::Focused
        } else {
            SecondStart::GameRunning
        }
    }

    /// The line for `launcher.log`.
    pub fn log_line(self) -> &'static str {
        match self {
            SecondStart::Focused => "another launcher is running: brought its window forward",
            SecondStart::GameRunning => "another launcher is running the game: Play refused",
        }
    }

    /// The exit code and, when the player must be told, the message.
    pub fn outcome(self) -> Result<u32, &'static str> {
        match self {
            SecondStart::Focused => Ok(0),
            SecondStart::GameRunning => Err(GAME_RUNNING),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_second_start_focuses_the_window_or_refuses_to_play() {
        let focused = SecondStart::new(true);
        assert_eq!(focused, SecondStart::Focused);
        assert_eq!(focused.outcome(), Ok(0));
        let running = SecondStart::new(false);
        assert_eq!(running, SecondStart::GameRunning);
        assert_eq!(running.outcome(), Err(GAME_RUNNING));
        assert!(running.log_line().contains("Play refused"));
    }

    #[test]
    fn only_the_launchers_own_window_counts_as_the_running_launcher() {
        let launcher = r"C:\Games\CMR4 Launcher\CMR4Launcher.exe";
        assert!(is_launcher_window(TITLE, launcher, "CMR4Launcher.exe"));
        assert!(is_launcher_window(TITLE, launcher, "cmr4launcher.EXE"));
        // A renamed launcher is still found by its own file name.
        let renamed = r"C:\Games\cmr4.launcher.exe";
        assert!(is_launcher_window(TITLE, renamed, "cmr4.launcher.exe"));
        // Same title, other programs: an Explorer folder window, the game.
        for other in [r"C:\Windows\explorer.exe", r"C:\Games\CMR4\cmr4.exe"] {
            assert!(!is_launcher_window(TITLE, other, "CMR4Launcher.exe"));
        }
        // The launcher's other windows (a file dialog) don't count.
        assert!(!is_launcher_window(
            "Choose the game folder",
            launcher,
            "CMR4Launcher.exe"
        ));
    }
}
