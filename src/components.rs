//! Bundled wrappers and their owned configuration files.

use std::fs;
use std::path::Path;
#[cfg(test)]
use std::path::PathBuf;

use crate::config::{Resolution, Settings};
use crate::patch::sha256;
use crate::registry;

pub const DGVOODOO_DLLS: [&str; 1] = ["D3D9.dll"];
pub const DXVK_DLLS: [&str; 2] = ["d3d11.dll", "dxgi.dll"];
pub const XIDI_DLL: &str = "dinput8.dll";
/// Launcher-built hook that lets Alt+F4 and window close end the game.
/// It is loaded from the package and never copied into the runtime.
pub const CLOSE_HOOK_DLL: &str = "cmr4close.dll";
const OWNERSHIP_FILE: &str = ".cmr4-launcher-components";
type Ownership = std::collections::BTreeMap<String, String>;

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

pub fn dgvoodoo_conf(
    settings: &Settings,
    wine: bool,
    gamescope: bool,
    output: Option<u32>,
) -> String {
    let game = registry::game_mode(settings.resolution(), settings.widescreen());
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

fn copy(source: &Path, destination: &Path) -> Result<(), String> {
    fs::copy(source, destination).map_err(|error| {
        format!(
            "cannot copy {} to {}: {error}",
            source.display(),
            destination.display()
        )
    })?;
    Ok(())
}

fn read_ownership(runtime: &Path) -> Ownership {
    fs::read_to_string(runtime.join(OWNERSHIP_FILE))
        .unwrap_or_default()
        .lines()
        .filter_map(|line| {
            let (digest, name) = line.split_once('\t')?;
            Some((name.to_owned(), digest.to_owned()))
        })
        .collect()
}

fn write_ownership(runtime: &Path, ownership: &Ownership) -> Result<(), String> {
    let text: String = ownership
        .iter()
        .map(|(name, digest)| format!("{digest}\t{name}\n"))
        .collect();
    fs::write(runtime.join(OWNERSHIP_FILE), text)
        .map_err(|error| format!("cannot record component ownership: {error}"))
}

fn is_owned(runtime: &Path, name: &str, ownership: &Ownership) -> Result<bool, String> {
    let path = runtime.join(name);
    if !path.is_file() {
        return Ok(false);
    }
    let Some(expected) = ownership.get(name) else {
        return Ok(false);
    };
    let current = fs::read(&path).map_err(|error| {
        format!(
            "cannot inspect runtime component {}: {error}",
            path.display()
        )
    })?;
    Ok(sha256(&current) == *expected)
}

fn is_unmodified_source_copy(game: &Path, runtime: &Path, name: &str) -> Result<bool, String> {
    let source = game.join(name);
    if !source.is_file() {
        return Ok(false);
    }
    let source_bytes = fs::read(&source).map_err(|error| {
        format!(
            "cannot inspect source component {}: {error}",
            source.display()
        )
    })?;
    let destination = runtime.join(name);
    let destination_bytes = fs::read(&destination).map_err(|error| {
        format!(
            "cannot inspect runtime component {}: {error}",
            destination.display()
        )
    })?;
    Ok(destination_bytes == source_bytes)
}

fn install_wrapper(
    game: &Path,
    runtime: &Path,
    components: &Path,
    name: &str,
    ownership: &mut Ownership,
) -> Result<(), String> {
    let destination = runtime.join(name);
    if destination.exists()
        && !is_owned(runtime, name, ownership)?
        && !is_unmodified_source_copy(game, runtime, name)?
    {
        return Err(format!(
            "runtime-only component {name} is not launcher-owned; refusing to replace {}",
            destination.display()
        ));
    }
    let bundled = components.join(name);
    copy(&bundled, &destination)?;
    let bytes = fs::read(&bundled)
        .map_err(|error| format!("cannot read bundled component {name}: {error}"))?;
    ownership.insert(name.to_owned(), sha256(&bytes));
    write_ownership(runtime, ownership)
}

fn disable_xidi(game: &Path, runtime: &Path, ownership: &mut Ownership) -> Result<(), String> {
    let destination = runtime.join(XIDI_DLL);
    // The game folder may ship its own dinput8.dll (even Xidi itself); an
    // untouched copy of it is already the "off" state.
    if destination.exists()
        && !is_owned(runtime, XIDI_DLL, ownership)?
        && is_unmodified_source_copy(game, runtime, XIDI_DLL)?
    {
        ownership.remove(XIDI_DLL);
        return write_ownership(runtime, ownership);
    }
    if destination.exists() && !is_owned(runtime, XIDI_DLL, ownership)? {
        return Err(format!(
            "runtime {XIDI_DLL} is not the launcher-installed Xidi; refusing to replace or remove a modified DLL"
        ));
    }
    if destination.exists() {
        let source = game.join(XIDI_DLL);
        if source.is_file() {
            copy(&source, &destination)?;
        } else {
            fs::remove_file(&destination)
                .map_err(|error| format!("cannot remove launcher-installed {XIDI_DLL}: {error}"))?;
        }
    }
    ownership.remove(XIDI_DLL);
    write_ownership(runtime, ownership)
}

fn disable_dxvk(game: &Path, runtime: &Path, ownership: &mut Ownership) -> Result<(), String> {
    for name in DXVK_DLLS {
        let destination = runtime.join(name);
        if destination.exists()
            && !is_owned(runtime, name, ownership)?
            && is_unmodified_source_copy(game, runtime, name)?
        {
            ownership.remove(name);
            continue;
        }
        if destination.exists() && !is_owned(runtime, name, ownership)? {
            return Err(format!(
                "runtime {name} is not launcher-owned; refusing to remove a modified DLL"
            ));
        }
        if destination.exists() {
            let source = game.join(name);
            if source.is_file() {
                copy(&source, &destination)?;
            } else {
                fs::remove_file(&destination)
                    .map_err(|error| format!("cannot remove launcher-installed {name}: {error}"))?;
            }
        }
        ownership.remove(name);
    }
    write_ownership(runtime, ownership)
}

/// Installs only launcher-owned wrappers. Other runtime DLLs, including a
/// user's DSOAL `dsound.dll`, are deliberately untouched.
pub fn install(
    game: &Path,
    runtime: &Path,
    components: &Path,
    settings: &Settings,
    wine: bool,
    gamescope: bool,
    output: Option<u32>,
) -> Result<(), String> {
    validate(components, settings.controller(), wine)?;
    let mut ownership = read_ownership(runtime);
    for name in DGVOODOO_DLLS {
        install_wrapper(game, runtime, components, name, &mut ownership)?;
    }
    if wine {
        for name in DXVK_DLLS {
            install_wrapper(game, runtime, components, name, &mut ownership)?;
        }
    } else {
        disable_dxvk(game, runtime, &mut ownership)?;
    }
    fs::write(
        runtime.join("dgVoodoo.conf"),
        dgvoodoo_conf(settings, wine, gamescope, output),
    )
    .map_err(|error| format!("cannot write dgVoodoo.conf: {error}"))?;

    if settings.controller() {
        install_wrapper(game, runtime, components, XIDI_DLL, &mut ownership)?;
        fs::write(runtime.join("Xidi.ini"), XIDI_CONF)
            .map_err(|error| format!("cannot write Xidi.ini: {error}"))?;
    } else {
        disable_xidi(game, runtime, &mut ownership)?;
        if fs::read(runtime.join("Xidi.ini")).is_ok_and(|bytes| bytes == XIDI_CONF.as_bytes()) {
            fs::remove_file(runtime.join("Xidi.ini"))
                .map_err(|error| format!("cannot remove owned Xidi.ini: {error}"))?;
        }
    }
    Ok(())
}

/// Replaces only rules for DLLs this launcher owns and keeps all unrelated
/// user rules. The required rule stays first and deterministic.
pub fn wine_overrides(existing: Option<&str>, controller: bool) -> String {
    let mut replaced = vec!["dsound", "d3d9", "d3d11", "dxgi", "mscoree"];
    if controller {
        replaced.push("dinput8");
    }
    let required = if controller {
        WINE_OVERRIDES
    } else {
        "dsound,d3d9,d3d11,dxgi=n,b;mscoree=d"
    };
    let mut result = vec![required.to_owned()];
    for rule in existing.into_iter().flat_map(|rules| rules.split(';')) {
        if rule.trim().is_empty() {
            continue;
        }
        let Some((names, action)) = rule.split_once('=') else {
            result.push(rule.to_owned());
            continue;
        };
        let kept: Vec<_> = names
            .split(',')
            .map(str::trim)
            .filter(|name| {
                !name.is_empty()
                    && !replaced
                        .iter()
                        .any(|replaced| name.eq_ignore_ascii_case(replaced))
            })
            .collect();
        if !kept.is_empty() {
            result.push(format!("{}={action}", kept.join(",")));
        }
    }
    result.join(";")
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
        let text = dgvoodoo_conf(&settings, false, false, None);
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
            let text = dgvoodoo_conf(&settings, wine, gamescope, None);
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
    fn widescreen_off_keeps_native_mode_and_aspect() {
        let settings = Settings {
            resolution: Some(Resolution::Custom(2560, 1600)),
            widescreen: Some(false),
            ..Settings::default()
        };
        let text = dgvoodoo_conf(&settings, true, false, Some(2));
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
        let text = dgvoodoo_conf(&settings, false, false, None);
        assert_eq!(ini_section(&text, "DirectX")["Resolution"], "unforced");
        assert_eq!(
            ini_section(&text, "DirectXExt")["ExtraEnumeratedResolutions"],
            ""
        );
        assert_eq!(ini_section(&text, "GeneralExt")["FPSLimit"], "0");
    }

    #[test]
    fn cli_over_cfg_reaches_dgvoodoo_and_the_registry_mode() {
        let (saved, warnings) = Settings::parse_file(
            "resolution = 1920x1080\nwidescreen = on\nfps_limit = 60\nvsync = on\n",
        );
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
        let text = dgvoodoo_conf(&settings, false, false, None);
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
            registry::game_mode(settings.resolution(), settings.widescreen()),
            Resolution::Custom(1920, 1440)
        );
    }

    #[test]
    fn steam_deck_preset_keeps_its_native_16_by_10_mode() {
        let settings = Settings {
            resolution: Some(Resolution::Custom(1280, 800)),
            widescreen: Some(false),
            fps_limit: Some(60),
            ..Settings::default()
        };
        // gamescope on the Deck: real fullscreen, one output.
        let text = dgvoodoo_conf(&settings, true, true, None);
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
            false,
            None,
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
                false,
                false,
                None,
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
            false,
            None,
        )
        .unwrap();
        let ownership = read_ownership(&runtime);
        for name in DGVOODOO_DLLS.into_iter().chain(DXVK_DLLS).chain([XIDI_DLL]) {
            assert!(ownership.contains_key(name));
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
            false,
            None,
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
                false,
                false,
                None,
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
        install(&game, &runtime, &components, &settings, true, false, None).unwrap();
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
        install(&game, &runtime, &components, &settings, false, false, None).unwrap();
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
        install(&game, &runtime, &components, &on, false, false, None).unwrap();
        assert_eq!(
            fs::read(runtime.join(XIDI_DLL)).unwrap(),
            b"bundle dinput8.dll"
        );
        assert!(runtime.join("Xidi.ini").is_file());

        let off = Settings {
            controller: Some(false),
            ..Settings::default()
        };
        install(&game, &runtime, &components, &off, false, false, None).unwrap();
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
            false,
            false,
            None,
        )
        .unwrap();
        fs::write(game.join(XIDI_DLL), b"source proxy").unwrap();
        let off = Settings {
            controller: Some(false),
            ..Settings::default()
        };
        install(&game, &runtime, &components, &off, false, false, None).unwrap();
        assert_eq!(fs::read(runtime.join(XIDI_DLL)).unwrap(), b"source proxy");
        // A second launch with the controller still off must accept the
        // game's own DLL (seen in the field: the game folder shipped Xidi).
        install(&game, &runtime, &components, &off, false, false, None).unwrap();
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
            false,
            false,
            None,
        )
        .unwrap();
        fs::write(runtime.join(XIDI_DLL), b"user changed").unwrap();
        fs::write(game.join(XIDI_DLL), b"source proxy").unwrap();
        let off = Settings {
            controller: Some(false),
            ..Settings::default()
        };
        let error = install(&game, &runtime, &components, &off, false, false, None).unwrap_err();
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
