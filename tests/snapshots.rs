//! Golden snapshots of everything the launcher produces (`tests/snapshots/`):
//! `--help`, the first-run `launcher.cfg`, and for each look and a few
//! option combinations the plan of a launch (patch sets, registry values,
//! environment, arguments, generated files). An SDK update or a refactor
//! that changes any of it fails here; a diff must match an intended change
//! or an SDK Migration item, never be regenerated blindly.
//!
//! Regenerate (host only, then review every diff by eye):
//! `CMR4_UPDATE_SNAPSHOTS=1 cargo test --test snapshots`.

use std::path::Path;

use cmr4_launcher::components::Platform;
use cmr4_launcher::config::{self, Settings};
use cmr4_launcher::plan;

const SCREEN: Option<(u32, u32)> = Some((3840, 2160));
const RUNTIME: &str = r"C:\Users\Player\AppData\Local\CMR4Launcher\CMR4";

/// The launch plan for a `launcher.cfg` and flags on a 4K screen.
/// `known`: the exe is an identified build (the fix can apply).
fn launch(
    cfg: &str,
    flags: &[&str],
    known: bool,
    platform: Platform,
    inherited: Option<&str>,
) -> String {
    let cli = config::parse_args(flags.iter().map(|f| f.to_string())).unwrap();
    let (file, warnings) = Settings::parse_file(cfg);
    assert!(warnings.is_empty(), "{warnings:?}");
    let mut settings = file
        .resolve_look(SCREEN)
        .merged(cli.settings.resolve_look(SCREEN));
    settings.resolution = settings
        .resolution
        .or(Some(config::default_resolution(SCREEN)));
    let widescreen = known && settings.widescreen();
    let plan = plan::plan(
        &settings,
        widescreen,
        Path::new(RUNTIME),
        &platform,
        &Default::default(),
        inherited,
    );
    format!(
        "# launcher.cfg: {cfg:?}\n# flags: {flags:?}\n# known build: {known}; platform: {platform:?}; inherited WINEDLLOVERRIDES: {inherited:?}\n# screen: 3840x2160\n\n{}",
        plan.render()
    )
}

/// Compares `actual` with `tests/snapshots/<name>` (the SDK's
/// `rea_launcher::testing`: the crate version stored as `<version>`, CRLF
/// read as LF). `CMR4_UPDATE_SNAPSHOTS=1` (or the SDK's
/// `REA_UPDATE_SNAPSHOTS=1`) writes them instead.
fn check(name: &str, actual: &str) {
    rea_launcher::snapshots!()
        .update_var("CMR4_UPDATE_SNAPSHOTS")
        .assert(name, actual);
}

const WINE: Platform = Platform::new(true, false, None);

#[test]
fn help_and_template() {
    check("help.txt", &config::usage());
    check("template.cfg", &config::template());
}

#[test]
fn looks() {
    for (name, flag) in [
        ("look-original.txt", "original"),
        ("look-widescreen.txt", "widescreen"),
        ("look-steam-deck.txt", "steam_deck"),
    ] {
        check(name, &launch("", &["--look", flag], true, WINE, None));
    }
}

#[test]
fn option_combinations() {
    check(
        "windows-1080p-60fps-vsync.txt",
        &launch(
            "resolution = 1920x1080\nfps_limit = 60\nvsync = on\n",
            &[],
            true,
            Platform::new(false, false, None),
            None,
        ),
    );
    check(
        "wine-16x10-no-gamepad-second-monitor.txt",
        &launch(
            "resolution = 1920x1200\n",
            &["--controller", "off"],
            true,
            Platform::new(true, false, Some(2)),
            Some("d3d9.dll=b;xinput1_3=n"),
        ),
    );
    check(
        "gamescope-unknown-build-16x9.txt",
        &launch(
            "resolution = 2560x1440\nfps_limit = off\n",
            &[],
            false,
            Platform::new(true, true, None),
            None,
        ),
    );
}
