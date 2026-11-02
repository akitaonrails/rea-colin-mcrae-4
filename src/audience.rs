//! Who sees what the launcher says (`sdk/docs/STANDARDS.md`, "Messages
//! reach the player"). The binary hands this to
//! `rea_launcher_win32::report`: a command's output (`--help`, `--check`)
//! goes to the terminal, a play-time error is a dialog whenever a person
//! may be watching.

/// The kind of run, from the launcher's arguments.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Audience {
    /// A script-facing command (`--help`, `--check`): its output and its
    /// errors are the command's result.
    pub command: bool,
    /// A person may be watching, so a dialog may be shown.
    pub interactive: bool,
}

/// The audience of a run started with `args` (the launcher's arguments,
/// program name excluded); `wine`: running under Wine.
///
/// `--no-window` usually means a script, so no dialog. Under Wine it
/// doesn't: the launcher is started by `cmr4-play` (the window run, then a
/// `--no-window` game run), the AppImage or Proton, for a player in front
/// of the screen, and its stdout goes to a log nobody reads. Without a
/// dialog there, a wrong game folder looked like a game that never
/// started.
pub fn audience(args: &[String], wine: bool) -> Audience {
    let has = |flag: &str| args.iter().any(|arg| arg == flag);
    Audience {
        command: has("--check") || has("--help") || has("-h") || has("/?"),
        interactive: wine || !has("--no-window"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rea_launcher_win32::report::{Mode, Route, Terminal, route};

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_string()).collect()
    }

    /// The `report::Mode` the binary builds from an audience.
    fn mode(audience: Audience) -> Mode {
        let interactive = audience.interactive;
        if audience.command {
            Mode::Command { interactive }
        } else {
            Mode::Play { interactive }
        }
    }

    /// `cmr4-play`, the AppImage and Proton: no console, stdout to a log.
    const UNDER_HELPER: Terminal = Terminal {
        attached: false,
        redirected: true,
        wine: true,
    };

    #[test]
    fn a_play_error_under_the_linux_helper_is_a_dialog() {
        // cmr4-play's game run (`--no-window`) and its window run.
        for run in [&["--no-window"][..], &["--settings", "--linux"], &[]] {
            let route = route(mode(audience(&args(run), true)), UNDER_HELPER);
            assert_eq!(
                route,
                Route {
                    print: true,
                    dialog: true
                },
                "{run:?}"
            );
        }
    }

    #[test]
    fn check_and_help_print_and_show_a_dialog_only_when_unseen() {
        let check = audience(&args(&["--check", "--no-window"]), true);
        assert!(check.command);
        // Under Wine stdout is a log: the player gets the dialog too.
        assert!(route(mode(check), UNDER_HELPER).dialog);
        // From a Windows terminal: printed, no dialog.
        let terminal = Terminal {
            attached: true,
            redirected: false,
            wine: false,
        };
        let check = audience(&args(&["--check"]), false);
        assert_eq!(
            route(mode(check), terminal),
            Route {
                print: true,
                dialog: false
            }
        );
        assert!(audience(&args(&["--help"]), false).command);
    }

    #[test]
    fn a_windows_script_with_no_window_never_gets_a_dialog() {
        let script = audience(&args(&["--no-window", "--game", "C:\\CMR4"]), false);
        assert_eq!(
            script,
            Audience {
                command: false,
                interactive: false
            }
        );
        let redirected = Terminal {
            attached: false,
            redirected: true,
            wine: false,
        };
        assert!(!route(mode(script), redirected).dialog);
        // A double-clicked shortcut is a person.
        assert!(audience(&[], false).interactive);
    }
}
