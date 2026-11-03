//! One launcher at a time (`sdk/docs/STANDARDS.md`, "Clean exit, single
//! instance"). The launcher holds a named mutex
//! (`rea_launcher_win32::instance::single`) from the start until the game
//! it started has exited; a second start asks the SDK what to do
//! (`AlreadyRunning::second_start` with `Window::Own`, which only takes a
//! window of this launcher's own exe: a title alone also matched an
//! Explorer folder named "Colin McRae Rally 04"). Before Play the launcher
//! also refuses while the runtime copy's game still runs
//! (`process::find_running`): a launcher killed while its game ran
//! released the mutex.

pub use rea_launcher_win32::instance::{ALREADY_RUNNING, SecondStart};

/// The instance name: `Local\rea-launcher-CMR4Launcher`.
pub const NAME: &str = "CMR4Launcher";

/// What a second start tells the player when there was no launcher window
/// to bring forward: the first launcher is waiting for the game.
pub const GAME_RUNNING: &str = "Colin McRae Rally 04 is already running. Quit the game (Alt+F4 works), then start the launcher again.";

/// What `--check` says while another launcher's window is open: it would
/// check against a runtime the other one may be changing.
pub const ALREADY_OPEN: &str =
    "The Colin McRae Rally 04 launcher is already open. Close it, then run the check again.";

/// What the player is told about a second start, in CMR4's words (the
/// SDK's say the same without the Alt+F4 hint); `None` when the running
/// launcher's window just came forward.
pub fn message(second: SecondStart) -> Option<&'static str> {
    match second {
        SecondStart::Focused => None,
        SecondStart::Busy => Some(GAME_RUNNING),
        SecondStart::WindowOpen => Some(ALREADY_OPEN),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rea_launcher_win32::instance::is_own_window;

    use crate::window::TITLE;

    #[test]
    fn a_second_start_focuses_the_window_or_refuses_with_exit_5() {
        let focused = SecondStart::decide(false, || unreachable!(), || true);
        assert_eq!(focused, SecondStart::Focused);
        assert_eq!((focused.exit_code(), message(focused)), (0, None));
        let busy = SecondStart::decide(false, || unreachable!(), || false);
        assert_eq!(busy, SecondStart::Busy);
        assert_eq!(busy.exit_code(), ALREADY_RUNNING);
        assert_eq!(ALREADY_RUNNING, 5);
        assert_eq!(message(busy), Some(GAME_RUNNING));
    }

    #[test]
    fn check_while_another_launcher_runs_is_refused_without_focusing_it() {
        let never = || -> bool { panic!("an action must not bring a window forward") };
        let open = SecondStart::decide(true, || true, never);
        assert_eq!(open, SecondStart::WindowOpen);
        assert_eq!((open.exit_code(), message(open)), (5, Some(ALREADY_OPEN)));
        let game = SecondStart::decide(true, || false, never);
        assert_eq!((game.exit_code(), message(game)), (5, Some(GAME_RUNNING)));
    }

    #[test]
    fn only_the_launchers_own_window_counts_as_the_running_launcher() {
        // f089792's case, now the SDK's Window::Own rule.
        let launcher = r"C:\Games\CMR4 Launcher\CMR4Launcher.exe";
        assert!(is_own_window(TITLE, TITLE, launcher, "CMR4Launcher.exe"));
        assert!(is_own_window(TITLE, TITLE, launcher, "cmr4launcher.EXE"));
        for other in [r"C:\Windows\explorer.exe", r"C:\Games\CMR4\cmr4.exe"] {
            assert!(!is_own_window(TITLE, TITLE, other, "CMR4Launcher.exe"));
        }
    }
}
