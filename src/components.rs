//! Bundled wrappers and their owned configuration files.
//!
//! What the launcher installed is recorded by SHA-256 in the runtime's
//! `rea_launcher::ledger` (`.cmr4-launcher-components`, the file and format
//! older launchers wrote); which wrappers and configs exist is CMR4's.

use std::fs;
use std::path::Path;
#[cfg(test)]
use std::path::PathBuf;

use rea_launcher::fsx;
use rea_launcher::ledger::{Ledger, Outcome};
use rea_launcher::wine::merge_dll_overrides;

use crate::config::{Resolution, Settings};
use crate::registry;

pub const DGVOODOO_DLLS: [&str; 1] = ["D3D9.dll"];
pub const DXVK_DLLS: [&str; 2] = ["d3d11.dll", "dxgi.dll"];
pub const XIDI_DLL: &str = "dinput8.dll";
/// The SDK's close hook (`rea-launcher-close-hook`), which lets Alt+F4
/// and window close end the game. It is loaded from the package and never
/// copied into the runtime.
pub const CLOSE_HOOK_DLL: &str = "rea_close_hook.dll";
/// Its configuration, set on the game as `REA_CLOSE_HOOK`: CMR04's window
/// procedure swallows `WM_CLOSE` and its loop ignores `WM_QUIT`, and the
/// game has no clean shutdown path to call, so a close request (the
/// window's close / the compositor's, which Wine turns into `SC_CLOSE`, or
/// Alt+F4) for its main window `PC_CMR4` ends the process at once. A plain
/// `WM_CLOSE` is left alone: the intro movies' window is closed with it on
/// the same thread.
pub const CLOSE_HOOK_CONFIG: &str = "class=PC_CMR4;strategy=terminate";
const OWNERSHIP_FILE: &str = ".cmr4-launcher-components";

/// Xidi's view of the pad, following the CMR3 launcher's rally mapper.
///
/// CMR04 reads a DirectInput joystick and stores its bindings by that
/// controller's layout, so the layout stays Xidi's StandardGamepad (stick on
/// X/Y, hat, buttons 1-12) and nothing becomes a keyboard key. Only the
/// pedals move: in Analogue mode the game drives accelerate and brake from
/// the Y axis, so RT and LT feed that axis and the physical left-stick Y is
/// ignored, so a resting thumb cannot open the throttle.
pub const XIDI_CONF: &str = r"[Mapper]
Type = CMR04Gamepad

[CustomMapper:CMR04Gamepad]
Template = StandardGamepad
StickLeftX = Axis(X)
StickLeftY = Null
TriggerRT = Axis(Y, -)
TriggerLT = Axis(Y, +)

[Properties]
UseBuiltInProperties = yes
DeadzonePercentStickLeft = 5
DeadzonePercentStickRight = 5
DeadzonePercentTriggerLT = 0
DeadzonePercentTriggerRT = 0
SaturationPercentStickLeft = 100
SaturationPercentStickRight = 100
SaturationPercentTriggerLT = 100
SaturationPercentTriggerRT = 100

[Log]
Enabled = no
Level = 1

[Import]
dinput8.dll = C:\Windows\system32\dinput8.dll

[Workarounds]
ActiveVirtualControllerMask = 0x01
";

pub const WINE_OVERRIDES: &str = "dinput8,dsound,d3d9,d3d11,dxgi=n,b;mscoree=d";

pub fn validate(components: &Path, controller: bool, wine: bool) -> Result<(), String> {
    let mut names = DGVOODOO_DLLS.to_vec();
    if controller {
        names.push(XIDI_DLL);
    }
    if wine {
        names.extend(DXVK_DLLS);
    }
    let missing: Vec<_> = names
        .into_iter()
        .filter(|name| !components.join(name).is_file())
        .collect();
    if missing.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "missing required component(s) {} in {}",
            missing.join(", "),
            components.display()
        ))
    }
}

/// Where the game runs: under Wine, inside gamescope, on which monitor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Platform {
    pub wine: bool,
    pub gamescope: bool,
    /// 1-based dgVoodoo `FullScreenOutput`, or None for its default.
    pub output: Option<u32>,
}

impl Platform {
    pub const fn new(wine: bool, gamescope: bool, output: Option<u32>) -> Platform {
        Platform {
            wine,
            gamescope,
            output,
        }
    }
}

/// `name: SHA-256 (version)` of each bundled DLL, for `launcher.log`.
pub fn describe(components: &Path) -> Vec<String> {
    let version = |name: &str| match name {
        "D3D9.dll" => "dgVoodoo2 2.80",
        "d3d11.dll" | "dxgi.dll" => "DXVK 3.1.1",
        XIDI_DLL => "Xidi 4.3.1",
        CLOSE_HOOK_DLL => "rea-launcher-close-hook",
        _ => "",
    };
    DGVOODOO_DLLS
        .into_iter()
        .chain(DXVK_DLLS)
        .chain([XIDI_DLL, CLOSE_HOOK_DLL])
        .map(
            |name| match rea_launcher::identify::sha256_file(&components.join(name)) {
                Ok(digest) => format!(
                    "{name}: {} ({})",
                    rea_launcher::identify::hex(&digest),
                    version(name)
                ),
                Err(_) => format!("{name}: not bundled"),
            },
        )
        .collect()
}

/// `dgVoodoo.conf`. `widescreen`: whether the runtime's `cmr4.exe` carries
/// the widescreen fix (a 16:9 resolution on a known build).
pub fn dgvoodoo_conf(settings: &Settings, widescreen: bool, platform: &Platform) -> String {
    let Platform {
        wine,
        gamescope,
        output,
    } = *platform;
    let game = registry::game_mode(settings.resolution(), widescreen);
    let (resolution, enumerated) = match (settings.resolution(), game) {
        (Resolution::Custom(width, height), Resolution::Custom(game_w, game_h)) => (
            format!("h:{width}, v:{height}"),
            format!("{game_w}x{game_h}@60"),
        ),
        _ => ("unforced".into(), String::new()),
    };
    // The anamorphic 4:3 frame must fill 16:9; everything else keeps its shape.
    let scaling = if game != settings.resolution() {
        "stretched"
    } else {
        "stretched_ar"
    };
    let output = output.map_or("default".into(), |n| n.to_string());
    let fullscreen_attributes = if wine && !gamescope { "fake" } else { "" };
    format!(
        "[General]\n\
         FullScreenOutput                    = {output}\n\
         FullScreenMode                      = true\n\
         ScalingMode                         = {scaling}\n\n\
         [GeneralExt]\n\
         FullscreenAttributes                = {fullscreen_attributes}\n\
         FPSLimit                            = {}\n\n\
         [DirectX]\n\
         Resolution                          = {resolution}\n\
         Filtering                           = appdriven\n\
         Antialiasing                        = appdriven\n\
         ForceVerticalSync                   = {}\n\
         dgVoodooWatermark                   = false\n\n\
         [DirectXExt]\n\
         ExtraEnumeratedResolutions          = {enumerated}\n",
        settings.fps_limit().unwrap_or(0),
        if settings.vsync() { "true" } else { "false" },
    )
}

fn ledger_error(name: &str) -> impl FnOnce(std::io::Error) -> String + '_ {
    move |error| format!("cannot update launcher component {name}: {error}")
}

fn install_wrapper(
    ledger: &mut Ledger,
    runtime: &Path,
    components: &Path,
    name: &str,
) -> Result<(), String> {
    let bundled = components.join(name);
    let bytes = fs::read(&bundled)
        .map_err(|error| format!("cannot read bundled component {name}: {error}"))?;
    match ledger
        .install(name, &bytes, &[])
        .map_err(ledger_error(name))?
    {
        Outcome::NotOurs => Err(format!(
            "runtime-only component {name} is not launcher-owned; refusing to replace {}",
            runtime.join(name).display()
        )),
        _ => Ok(()),
    }
}

/// Takes a launcher-installed wrapper out: the game's own copy comes back,
/// or the file goes. The game's own untouched DLL (some ship Xidi itself)
/// is already the "off" state; a changed one is refused with `refusal`.
fn take_out(ledger: &mut Ledger, name: &str, refusal: &str) -> Result<(), String> {
    match ledger.take_out(name, &[]).map_err(ledger_error(name))? {
        Outcome::NotOurs => Err(refusal.to_owned()),
        _ => Ok(()),
    }
}

/// Installs only launcher-owned wrappers. Other runtime DLLs, including a
/// user's DSOAL `dsound.dll`, are deliberately untouched.
pub fn install(
    game: &Path,
    runtime: &Path,
    components: &Path,
    settings: &Settings,
    widescreen: bool,
    platform: &Platform,
) -> Result<(), String> {
    let wine = platform.wine;
    validate(components, settings.controller(), wine)?;
    let mut ledger = Ledger::load(game, runtime, OWNERSHIP_FILE);
    for name in DGVOODOO_DLLS {
        install_wrapper(&mut ledger, runtime, components, name)?;
    }
    for name in DXVK_DLLS {
        if wine {
            install_wrapper(&mut ledger, runtime, components, name)?;
        } else {
            take_out(
                &mut ledger,
                name,
                &format!("runtime {name} is not launcher-owned; refusing to remove a modified DLL"),
            )?;
        }
    }
    fsx::write_atomic(
        &runtime.join("dgVoodoo.conf"),
        dgvoodoo_conf(settings, widescreen, platform),
    )
    .map_err(|error| format!("cannot write dgVoodoo.conf: {error}"))?;

    if settings.controller() {
        install_wrapper(&mut ledger, runtime, components, XIDI_DLL)?;
        fsx::write_atomic(&runtime.join("Xidi.ini"), XIDI_CONF)
            .map_err(|error| format!("cannot write Xidi.ini: {error}"))?;
    } else {
        take_out(
            &mut ledger,
            XIDI_DLL,
            &format!(
                "runtime {XIDI_DLL} is not the launcher-installed Xidi; refusing to replace or remove a modified DLL"
            ),
        )?;
        if fs::read(runtime.join("Xidi.ini")).is_ok_and(|bytes| bytes == XIDI_CONF.as_bytes()) {
            fsx::remove(&runtime.join("Xidi.ini"))
                .map_err(|error| format!("cannot remove owned Xidi.ini: {error}"))?;
        }
    }
    Ok(())
}

/// Replaces only rules for DLLs this launcher owns and keeps all unrelated
/// user rules. The required rule stays first and deterministic.
pub fn wine_overrides(existing: Option<&str>, controller: bool) -> String {
    let ours = if controller {
        WINE_OVERRIDES
    } else {
        "dsound,d3d9,d3d11,dxgi=n,b;mscoree=d"
    };
    merge_dll_overrides(ours, existing)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "cmr4-components-test-{}-{name}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&path);
        path
    }

    fn write_components(path: &Path) {
        fs::create_dir_all(path).unwrap();
        for name in DGVOODOO_DLLS.into_iter().chain(DXVK_DLLS).chain([XIDI_DLL]) {
            fs::write(path.join(name), format!("bundle {name}")).unwrap();
        }
    }

    fn ini_section(text: &str, wanted: &str) -> std::collections::BTreeMap<String, String> {
        let mut current = "";
        let mut values = std::collections::BTreeMap::new();
        for line in text.lines() {
            let line = line.trim();
            if let Some(section) = line
                .strip_prefix('[')
                .and_then(|line| line.strip_suffix(']'))
            {
                current = section;
            } else if current == wanted
                && let Some((key, value)) = line.split_once('=')
            {
                values.insert(key.trim().to_owned(), value.trim().to_owned());
            }
        }
        values
    }

    #[test]
    fn dgvoodoo_values_are_in_their_real_sections() {
        let settings = Settings {
            resolution: Some(Resolution::Custom(3840, 2160)),
            fps_limit: Some(60),
            vsync: Some(true),
            ..Settings::default()
        };
        let text = dgvoodoo_conf(
            &settings,
            settings.widescreen(),
            &Platform::new(false, false, None),
        );
        let general = ini_section(&text, "GeneralExt");
        let directx = ini_section(&text, "DirectX");
        let extended = ini_section(&text, "DirectXExt");
        assert_eq!(general["FPSLimit"], "60");
        assert_eq!(directx["Resolution"], "h:3840, v:2160");
        assert_eq!(directx["Antialiasing"], "appdriven");
        assert_eq!(directx["Filtering"], "appdriven");
        assert_eq!(directx["ForceVerticalSync"], "true");
        assert_eq!(directx["dgVoodooWatermark"], "false");
        // Widescreen is on by default: the game renders 4:3 and dgVoodoo stretches it.
        assert_eq!(extended["ExtraEnumeratedResolutions"], "2880x2160@60");
        assert_eq!(ini_section(&text, "General")["ScalingMode"], "stretched");
        assert!(!directx.contains_key("FPSLimit"));
        assert!(!directx.contains_key("ExtraEnumeratedResolutions"));
    }

    #[test]
    fn dgvoodoo_display_policy_matches_platform_context() {
        let settings = Settings::default();
        for (wine, gamescope, attributes) in
            [(false, false, ""), (true, true, ""), (true, false, "fake")]
        {
            let text = dgvoodoo_conf(&settings, true, &Platform::new(wine, gamescope, None));
            let general = ini_section(&text, "General");
            let extended = ini_section(&text, "GeneralExt");
            assert_eq!(general["FullScreenMode"], "true");
            assert_eq!(general["ScalingMode"], "stretched");
            assert_eq!(general["FullScreenOutput"], "default");
            assert_eq!(extended["FullscreenAttributes"], attributes);
            assert_eq!(extended["FPSLimit"], "0");
        }
    }

    #[test]
    fn other_shapes_keep_their_native_mode_and_aspect() {
        let settings = Settings {
            resolution: Some(Resolution::Custom(2560, 1600)),
            ..Settings::default()
        };
        let text = dgvoodoo_conf(
            &settings,
            settings.widescreen(),
            &Platform::new(true, false, Some(2)),
        );
        let general = ini_section(&text, "General");
        assert_eq!(general["ScalingMode"], "stretched_ar");
        assert_eq!(general["FullScreenOutput"], "2");
        assert_eq!(
            ini_section(&text, "DirectX")["Resolution"],
            "h:2560, v:1600"
        );
        assert_eq!(
            ini_section(&text, "DirectXExt")["ExtraEnumeratedResolutions"],
            "2560x1600@60"
        );
    }

    #[test]
    fn original_mode_is_unforced() {
        let settings = Settings {
            resolution: Some(Resolution::Original),
            ..Settings::default()
        };
        let text = dgvoodoo_conf(
            &settings,
            settings.widescreen(),
            &Platform::new(false, false, None),
        );
        assert_eq!(ini_section(&text, "DirectX")["Resolution"], "unforced");
        assert_eq!(
            ini_section(&text, "DirectXExt")["ExtraEnumeratedResolutions"],
            ""
        );
        assert_eq!(ini_section(&text, "GeneralExt")["FPSLimit"], "0");
    }

    #[test]
    fn cli_over_cfg_reaches_dgvoodoo_and_the_registry_mode() {
        let (saved, warnings) =
            Settings::parse_file("resolution = 1920x1080\nfps_limit = 60\nvsync = on\n");
        assert!(warnings.is_empty());
        let cli = crate::config::parse_args(
            [
                "--resolution",
                "2560x1440",
                "--fps-limit",
                "144",
                "--vsync",
                "off",
            ]
            .map(String::from),
        )
        .unwrap();
        let settings = saved.merged(cli.settings);
        let text = dgvoodoo_conf(
            &settings,
            settings.widescreen(),
            &Platform::new(false, false, None),
        );
        assert_eq!(
            ini_section(&text, "DirectX")["Resolution"],
            "h:2560, v:1440"
        );
        assert_eq!(ini_section(&text, "DirectX")["ForceVerticalSync"], "false");
        assert_eq!(ini_section(&text, "GeneralExt")["FPSLimit"], "144");
        assert_eq!(
            ini_section(&text, "DirectXExt")["ExtraEnumeratedResolutions"],
            "1920x1440@60"
        );
        assert_eq!(
            registry::game_mode(settings.resolution(), true),
            Resolution::Custom(1920, 1440)
        );
    }

    #[test]
    fn an_unknown_build_at_16_by_9_renders_natively_without_stretching() {
        // No fix in the exe: the game draws 16:9 itself, dgVoodoo keeps it.
        let settings = Settings::default();
        let text = dgvoodoo_conf(&settings, false, &Platform::default());
        assert_eq!(ini_section(&text, "General")["ScalingMode"], "stretched_ar");
        assert_eq!(
            ini_section(&text, "DirectXExt")["ExtraEnumeratedResolutions"],
            "1920x1080@60"
        );
    }

    #[test]
    fn steam_deck_preset_keeps_its_native_16_by_10_mode() {
        let settings = Settings {
            resolution: Some(Resolution::Custom(1280, 800)),
            fps_limit: Some(60),
            ..Settings::default()
        };
        // gamescope on the Deck: real fullscreen, one output.
        let text = dgvoodoo_conf(
            &settings,
            settings.widescreen(),
            &Platform::new(true, true, None),
        );
        assert_eq!(ini_section(&text, "General")["ScalingMode"], "stretched_ar");
        assert_eq!(ini_section(&text, "GeneralExt")["FullscreenAttributes"], "");
        assert_eq!(ini_section(&text, "GeneralExt")["FPSLimit"], "60");
        assert_eq!(ini_section(&text, "DirectX")["Resolution"], "h:1280, v:800");
        assert_eq!(
            ini_section(&text, "DirectXExt")["ExtraEnumeratedResolutions"],
            "1280x800@60"
        );
    }

    #[test]
    fn xidi_keeps_a_gamepad_layout_and_puts_the_triggers_on_the_pedal_axis() {
        for line in [
            "Template = StandardGamepad",
            "StickLeftX = Axis(X)",
            "StickLeftY = Null",
            "TriggerRT = Axis(Y, -)",
            "TriggerLT = Axis(Y, +)",
            "dinput8.dll = C:\\Windows\\system32\\dinput8.dll",
            "ActiveVirtualControllerMask = 0x01",
        ] {
            assert!(XIDI_CONF.contains(line), "{line}");
        }
        // Keys on top of the pad would change nothing CMR04 needs and, as the
        // keyboard-only mapping showed, hide the pad's axes from the game.
        assert!(!XIDI_CONF.contains("Keyboard("));
        // Xidi 4.3.1 rejects unknown [Log] keys (seen in the CMR3 launcher).
        assert!(!XIDI_CONF.contains("Append"));
    }

    /// The exact Xidi.ini verified in the game on 2026-11-02 (see
    /// docs/RUNTIME_NOTES.md). Any change to the mapping must update the
    /// fixture on purpose and be checked in the game again.
    const XIDI_GOLDEN: &str = include_str!("../tests/fixtures/xidi.ini");

    #[test]
    fn generated_xidi_ini_matches_the_golden_fixture() {
        assert_eq!(XIDI_CONF, XIDI_GOLDEN);
        let game = temp("golden-game");
        let runtime = temp("golden-runtime");
        let components = temp("golden-components");
        fs::create_dir_all(&game).unwrap();
        fs::create_dir_all(&runtime).unwrap();
        write_components(&components);
        install(
            &game,
            &runtime,
            &components,
            &Settings::default(),
            true,
            &Platform::new(true, false, None),
        )
        .unwrap();
        assert_eq!(
            fs::read_to_string(runtime.join("Xidi.ini")).unwrap(),
            XIDI_GOLDEN
        );
        for path in [game, runtime, components] {
            fs::remove_dir_all(path).unwrap();
        }
    }

    #[test]
    fn the_log_names_each_bundled_dll_with_its_digest_and_version() {
        let components = temp("describe");
        write_components(&components);
        let lines = describe(&components);
        assert_eq!(lines.len(), 5);
        assert!(lines[0].starts_with("D3D9.dll: ") && lines[0].ends_with("(dgVoodoo2 2.80)"));
        assert!(
            lines
                .iter()
                .any(|l| l.starts_with("dinput8.dll: ") && l.ends_with("(Xidi 4.3.1)"))
        );
        assert_eq!(lines[4], "rea_close_hook.dll: not bundled");
        fs::remove_dir_all(components).unwrap();
    }

    #[test]
    fn the_close_hook_ends_the_game_on_close_and_alt_f4_only() {
        use rea_close_hook::msg::{
            ALT_DOWN, SC_CLOSE, VK_F4, WM_CLOSE, WM_SYSCOMMAND, WM_SYSKEYDOWN,
        };
        use rea_close_hook::{Config, Strategy};
        let config = Config::parse(CLOSE_HOOK_CONFIG).unwrap();
        assert_eq!(config.strategy, Strategy::Terminate);
        assert_eq!(config.code, 0);
        assert!(config.is_game_class("PC_CMR4"));
        assert!(!config.is_game_class("MoviePlayer"));
        let requests = config.requests;
        assert!(requests.matches(WM_SYSCOMMAND, SC_CLOSE, 0));
        // The low four bits of SC_* are reserved and may be set.
        assert!(requests.matches(WM_SYSCOMMAND, SC_CLOSE | 0x2, 0));
        assert!(requests.matches(WM_SYSKEYDOWN, VK_F4, ALT_DOWN | 1));
        assert!(!requests.matches(WM_SYSKEYDOWN, VK_F4, 1)); // F4 without Alt
        assert!(!requests.matches(WM_SYSKEYDOWN, 0x09, ALT_DOWN)); // Alt+Tab
        assert!(!requests.matches(WM_SYSCOMMAND, 0xF140, 0)); // SC_SCREENSAVE
        // The intro movies' window is closed with WM_CLOSE: never the game.
        assert!(!requests.matches(WM_CLOSE, 0, 0));
    }

    #[test]
    fn missing_components_fail_clearly_and_check_can_skip_them() {
        let components = temp("missing");
        fs::create_dir_all(&components).unwrap();
        let error = validate(&components, true, true).unwrap_err();
        assert!(error.contains("D3D9.dll"));
        assert!(error.contains("dinput8.dll"));
        assert!(error.contains("dxgi.dll"));
        assert!(validate(&components, false, true).is_err());
        assert!(validate(&components, true, false).is_err());
        for name in DGVOODOO_DLLS.into_iter().chain([XIDI_DLL]) {
            fs::write(components.join(name), name).unwrap();
        }
        assert_eq!(validate(&components, true, false), Ok(()));
        assert!(
            validate(&components, true, true)
                .unwrap_err()
                .contains("dxgi.dll")
        );
        fs::remove_dir_all(components).unwrap();
    }

    #[test]
    fn first_time_runtime_only_wrapper_conflicts_are_refused() {
        let game = temp("conflict-game");
        let runtimes = temp("conflict-runtimes");
        let components = temp("conflict-components");
        fs::create_dir_all(&game).unwrap();
        write_components(&components);
        for name in DGVOODOO_DLLS.into_iter().chain(DXVK_DLLS).chain([XIDI_DLL]) {
            let runtime = runtimes.join(name);
            fs::create_dir_all(&runtime).unwrap();
            fs::write(runtime.join(name), b"user DLL").unwrap();
            let error = install(
                &game,
                &runtime,
                &components,
                &Settings::default(),
                true,
                &Platform::new(false, false, None),
            )
            .unwrap_err();
            assert!(error.contains("not launcher-owned"), "{error}");
            assert_eq!(fs::read(runtime.join(name)).unwrap(), b"user DLL");
        }
        for path in [game, runtimes, components] {
            fs::remove_dir_all(path).unwrap();
        }
    }

    #[test]
    fn unmodified_source_copies_and_owned_wrapper_updates_work() {
        let game = temp("repack-game");
        let runtime = temp("repack-runtime");
        let components = temp("repack-components");
        fs::create_dir_all(&game).unwrap();
        fs::create_dir_all(&runtime).unwrap();
        write_components(&components);
        for name in DGVOODOO_DLLS.into_iter().chain(DXVK_DLLS).chain([XIDI_DLL]) {
            let source = format!("source {name}");
            fs::write(game.join(name), &source).unwrap();
            fs::write(runtime.join(name), &source).unwrap();
        }
        install(
            &game,
            &runtime,
            &components,
            &Settings::default(),
            true,
            &Platform::new(true, false, None),
        )
        .unwrap();
        let ledger = Ledger::load(&game, &runtime, OWNERSHIP_FILE);
        let owned: Vec<&str> = ledger.names().collect();
        for name in DGVOODOO_DLLS.into_iter().chain(DXVK_DLLS).chain([XIDI_DLL]) {
            assert!(owned.contains(&name), "{name}");
            assert_eq!(
                fs::read(runtime.join(name)).unwrap(),
                format!("bundle {name}").as_bytes()
            );
        }
        fs::write(components.join("D3D9.dll"), b"updated bundle").unwrap();
        install(
            &game,
            &runtime,
            &components,
            &Settings::default(),
            true,
            &Platform::new(true, false, None),
        )
        .unwrap();
        assert_eq!(
            fs::read(runtime.join("D3D9.dll")).unwrap(),
            b"updated bundle"
        );
        for path in [game, runtime, components] {
            fs::remove_dir_all(path).unwrap();
        }
    }

    #[test]
    fn a_ledger_written_by_an_older_launcher_still_proves_ownership() {
        let game = temp("old-ledger-game");
        let runtime = temp("old-ledger-runtime");
        let components = temp("old-ledger-components");
        fs::create_dir_all(&game).unwrap();
        fs::create_dir_all(&runtime).unwrap();
        write_components(&components);
        // v0.2.0's `.cmr4-launcher-components`: `sha256\tname` lines.
        fs::write(runtime.join("D3D9.dll"), b"dgVoodoo 2.80").unwrap();
        fs::write(
            runtime.join(OWNERSHIP_FILE),
            format!(
                "{}\tD3D9.dll\n",
                rea_launcher::identify::hex(&rea_launcher::identify::sha256(b"dgVoodoo 2.80"))
            ),
        )
        .unwrap();
        install(
            &game,
            &runtime,
            &components,
            &Settings::default(),
            true,
            &Platform::new(false, false, None),
        )
        .unwrap();
        assert_eq!(
            fs::read(runtime.join("D3D9.dll")).unwrap(),
            b"bundle D3D9.dll"
        );
        let ledger = fs::read_to_string(runtime.join(OWNERSHIP_FILE)).unwrap();
        assert!(ledger.contains(&format!(
            "{}\tD3D9.dll\n",
            rea_launcher::identify::hex(&rea_launcher::identify::sha256(b"bundle D3D9.dll"))
        )));
        for path in [game, runtime, components] {
            fs::remove_dir_all(path).unwrap();
        }
    }

    #[test]
    fn divergent_source_and_runtime_dlls_conflict_without_overwriting() {
        let game = temp("divergent-game");
        let runtimes = temp("divergent-runtimes");
        let components = temp("divergent-components");
        fs::create_dir_all(&game).unwrap();
        write_components(&components);
        for name in DGVOODOO_DLLS.into_iter().chain(DXVK_DLLS).chain([XIDI_DLL]) {
            fs::write(game.join(name), format!("source {name}")).unwrap();
            let runtime = runtimes.join(name);
            fs::create_dir_all(&runtime).unwrap();
            fs::write(runtime.join(name), b"divergent runtime DLL").unwrap();
            let error = install(
                &game,
                &runtime,
                &components,
                &Settings::default(),
                true,
                &Platform::new(false, false, None),
            )
            .unwrap_err();
            assert!(error.contains("not launcher-owned"), "{error}");
            assert_eq!(
                fs::read(runtime.join(name)).unwrap(),
                b"divergent runtime DLL"
            );
        }
        for path in [game, runtimes, components] {
            fs::remove_dir_all(path).unwrap();
        }
    }

    #[test]
    fn dxvk_is_wine_only_and_owned_files_are_removed_for_native_windows() {
        let game = temp("dxvk-game");
        let runtime = temp("dxvk-runtime");
        let components = temp("dxvk-components");
        fs::create_dir_all(&game).unwrap();
        fs::create_dir_all(&runtime).unwrap();
        write_components(&components);
        let settings = Settings::default();
        install(
            &game,
            &runtime,
            &components,
            &settings,
            settings.widescreen(),
            &Platform::new(true, false, None),
        )
        .unwrap();
        for name in DXVK_DLLS {
            assert_eq!(
                fs::read(runtime.join(name)).unwrap(),
                format!("bundle {name}").as_bytes()
            );
        }
        assert_eq!(
            ini_section(
                &fs::read_to_string(runtime.join("dgVoodoo.conf")).unwrap(),
                "GeneralExt"
            )["FullscreenAttributes"],
            "fake"
        );
        install(
            &game,
            &runtime,
            &components,
            &settings,
            settings.widescreen(),
            &Platform::new(false, false, None),
        )
        .unwrap();
        for name in DXVK_DLLS {
            assert!(!runtime.join(name).exists());
        }
        for path in [game, runtime, components] {
            fs::remove_dir_all(path).unwrap();
        }
    }

    #[test]
    fn controller_toggle_removes_only_launcher_installed_xidi() {
        let game = temp("game");
        let runtime = temp("runtime");
        let components = temp("components");
        fs::create_dir_all(&game).unwrap();
        fs::create_dir_all(&runtime).unwrap();
        write_components(&components);
        fs::write(runtime.join("dsound.dll"), b"user DSOAL").unwrap();
        let on = Settings::default();
        install(
            &game,
            &runtime,
            &components,
            &on,
            on.widescreen(),
            &Platform::new(false, false, None),
        )
        .unwrap();
        assert_eq!(
            fs::read(runtime.join(XIDI_DLL)).unwrap(),
            b"bundle dinput8.dll"
        );
        assert!(runtime.join("Xidi.ini").is_file());

        let off = Settings {
            controller: Some(false),
            ..Settings::default()
        };
        install(
            &game,
            &runtime,
            &components,
            &off,
            off.widescreen(),
            &Platform::new(false, false, None),
        )
        .unwrap();
        assert!(!runtime.join(XIDI_DLL).exists());
        assert!(!runtime.join("Xidi.ini").exists());
        assert_eq!(fs::read(runtime.join("dsound.dll")).unwrap(), b"user DSOAL");

        for path in [game, runtime, components] {
            fs::remove_dir_all(path).unwrap();
        }
    }

    #[test]
    fn controller_off_restores_source_dinput8() {
        let game = temp("source-dinput-game");
        let runtime = temp("source-dinput-runtime");
        let components = temp("source-dinput-components");
        fs::create_dir_all(&game).unwrap();
        fs::create_dir_all(&runtime).unwrap();
        write_components(&components);
        install(
            &game,
            &runtime,
            &components,
            &Settings::default(),
            true,
            &Platform::new(false, false, None),
        )
        .unwrap();
        fs::write(game.join(XIDI_DLL), b"source proxy").unwrap();
        let off = Settings {
            controller: Some(false),
            ..Settings::default()
        };
        install(
            &game,
            &runtime,
            &components,
            &off,
            off.widescreen(),
            &Platform::new(false, false, None),
        )
        .unwrap();
        assert_eq!(fs::read(runtime.join(XIDI_DLL)).unwrap(), b"source proxy");
        // A second launch with the controller still off must accept the
        // game's own DLL (seen in the field: the game folder shipped Xidi).
        install(
            &game,
            &runtime,
            &components,
            &off,
            off.widescreen(),
            &Platform::new(false, false, None),
        )
        .unwrap();
        assert_eq!(fs::read(runtime.join(XIDI_DLL)).unwrap(), b"source proxy");
        for path in [game, runtime, components] {
            fs::remove_dir_all(path).unwrap();
        }
    }

    #[test]
    fn modified_xidi_is_not_deleted() {
        let game = temp("modified-game");
        let runtime = temp("modified-runtime");
        let components = temp("modified-components");
        fs::create_dir_all(&game).unwrap();
        fs::create_dir_all(&runtime).unwrap();
        write_components(&components);
        install(
            &game,
            &runtime,
            &components,
            &Settings::default(),
            true,
            &Platform::new(false, false, None),
        )
        .unwrap();
        fs::write(runtime.join(XIDI_DLL), b"user changed").unwrap();
        fs::write(game.join(XIDI_DLL), b"source proxy").unwrap();
        let off = Settings {
            controller: Some(false),
            ..Settings::default()
        };
        let error = install(
            &game,
            &runtime,
            &components,
            &off,
            off.widescreen(),
            &Platform::new(false, false, None),
        )
        .unwrap_err();
        assert!(error.contains("refusing to replace or remove"), "{error}");
        assert_eq!(fs::read(runtime.join(XIDI_DLL)).unwrap(), b"user changed");
        assert_eq!(fs::read(game.join(XIDI_DLL)).unwrap(), b"source proxy");
        for path in [game, runtime, components] {
            fs::remove_dir_all(path).unwrap();
        }
    }

    #[test]
    fn controller_on_forces_xidi_and_preserves_unrelated_rules() {
        assert_eq!(wine_overrides(None, true), WINE_OVERRIDES);
        assert_eq!(
            wine_overrides(Some("bnet=n,b;DINPUT8=b;foo=b;mscoree=n"), true),
            format!("{WINE_OVERRIDES};bnet=n,b;foo=b")
        );
        assert_eq!(
            wine_overrides(Some("d3d8,d3d9=b;dsound=n"), true),
            format!("{WINE_OVERRIDES};d3d8=b")
        );
        // Rules spelled with `.dll`, in any case, are ours to replace too.
        assert_eq!(
            wine_overrides(Some("d3d9.dll=b;DXGI.DLL,ddraw.dll=n;dinput8.Dll=b"), true),
            format!("{WINE_OVERRIDES};ddraw.dll=n")
        );
        assert_eq!(
            wine_overrides(Some("dinput8.dll=b;d3d11.dll=b"), false),
            "dsound,d3d9,d3d11,dxgi=n,b;mscoree=d;dinput8.dll=b"
        );
    }

    #[test]
    fn controller_off_preserves_user_dinput8_rule_only() {
        assert_eq!(
            wine_overrides(None, false),
            "dsound,d3d9,d3d11,dxgi=n,b;mscoree=d"
        );
        assert_eq!(
            wine_overrides(
                Some("dinput8=b;bnet=n,b;d3d8=b;dinput8,dsound=n;dxgi=b"),
                false
            ),
            "dsound,d3d9,d3d11,dxgi=n,b;mscoree=d;dinput8=b;bnet=n,b;d3d8=b;dinput8=n"
        );
    }
}
