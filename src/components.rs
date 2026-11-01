//! Bundled wrappers and their owned configuration files.

use std::fs;
use std::path::Path;
#[cfg(test)]
use std::path::PathBuf;

use crate::config::{Resolution, Settings};
use crate::patch::sha256;

pub const DGVOODOO_DLLS: [&str; 1] = ["D3D9.dll"];
pub const DXVK_DLLS: [&str; 2] = ["d3d11.dll", "dxgi.dll"];
pub const XIDI_DLL: &str = "dinput8.dll";
const OWNERSHIP_FILE: &str = ".cmr4-launcher-components";
type Ownership = std::collections::BTreeMap<String, String>;

pub const XIDI_CONF: &str = r"[Mapper]
Type = CMR04Gamepad

[CustomMapper:CMR04Gamepad]
Template = StandardGamepad
StickLeftX = Split( Keyboard(RightArrow), Keyboard(LeftArrow) )
StickLeftY = Split( Keyboard(DownArrow), Keyboard(UpArrow) )
TriggerRT = Keyboard(UpArrow)
TriggerLT = Keyboard(DownArrow)
DpadUp = Keyboard(UpArrow)
DpadDown = Keyboard(DownArrow)
DpadLeft = Keyboard(LeftArrow)
DpadRight = Keyboard(RightArrow)
ButtonA = Keyboard(Enter)
ButtonB = Keyboard(Esc)
ButtonX = Keyboard(Space)
ButtonY = Keyboard(C)
ButtonStart = Keyboard(Enter)
ButtonBack = Keyboard(Esc)

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

pub fn dgvoodoo_conf(settings: &Settings, wine: bool, gamescope: bool) -> String {
    let (resolution, enumerated) = match settings.resolution() {
        Resolution::Original => ("unforced".into(), String::new()),
        Resolution::Custom(width, height) => (
            format!("h:{width}, v:{height}"),
            format!("{width}x{height}@60"),
        ),
    };
    let fullscreen_attributes = if wine && !gamescope { "fake" } else { "" };
    format!(
        "[General]\n\
         FullScreenMode                      = true\n\
         ScalingMode                         = stretched_ar\n\n\
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
        dgvoodoo_conf(settings, wine, gamescope),
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
        let text = dgvoodoo_conf(&settings, false, false);
        let general = ini_section(&text, "GeneralExt");
        let directx = ini_section(&text, "DirectX");
        let extended = ini_section(&text, "DirectXExt");
        assert_eq!(general["FPSLimit"], "60");
        assert_eq!(directx["Resolution"], "h:3840, v:2160");
        assert_eq!(directx["Antialiasing"], "appdriven");
        assert_eq!(directx["Filtering"], "appdriven");
        assert_eq!(directx["ForceVerticalSync"], "true");
        assert_eq!(directx["dgVoodooWatermark"], "false");
        assert_eq!(extended["ExtraEnumeratedResolutions"], "3840x2160@60");
        assert!(!directx.contains_key("FPSLimit"));
        assert!(!directx.contains_key("ExtraEnumeratedResolutions"));
    }

    #[test]
    fn dgvoodoo_display_policy_matches_platform_context() {
        let settings = Settings::default();
        for (wine, gamescope, attributes) in
            [(false, false, ""), (true, true, ""), (true, false, "fake")]
        {
            let text = dgvoodoo_conf(&settings, wine, gamescope);
            let general = ini_section(&text, "General");
            let extended = ini_section(&text, "GeneralExt");
            assert_eq!(general["FullScreenMode"], "true");
            assert_eq!(general["ScalingMode"], "stretched_ar");
            assert_eq!(extended["FullscreenAttributes"], attributes);
            assert_eq!(extended["FPSLimit"], "0");
        }
    }

    #[test]
    fn original_mode_is_unforced() {
        let settings = Settings {
            resolution: Some(Resolution::Original),
            ..Settings::default()
        };
        let text = dgvoodoo_conf(&settings, false, false);
        assert_eq!(ini_section(&text, "DirectX")["Resolution"], "unforced");
        assert_eq!(
            ini_section(&text, "DirectXExt")["ExtraEnumeratedResolutions"],
            ""
        );
        assert_eq!(ini_section(&text, "GeneralExt")["FPSLimit"], "0");
    }

    #[test]
    fn exact_xidi_mapper_is_embedded() {
        for line in [
            "StickLeftX = Split( Keyboard(RightArrow), Keyboard(LeftArrow) )",
            "TriggerRT = Keyboard(UpArrow)",
            "TriggerLT = Keyboard(DownArrow)",
            "ButtonA = Keyboard(Enter)",
            "ButtonStart = Keyboard(Enter)",
            "ButtonB = Keyboard(Esc)",
            "ButtonBack = Keyboard(Esc)",
            "ButtonX = Keyboard(Space)",
            "ButtonY = Keyboard(C)",
            "dinput8.dll = C:\\Windows\\system32\\dinput8.dll",
            "ActiveVirtualControllerMask = 0x01",
        ] {
            assert!(XIDI_CONF.contains(line), "{line}");
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
        install(&game, &runtime, &components, &settings, true, false).unwrap();
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
        install(&game, &runtime, &components, &settings, false, false).unwrap();
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
        install(&game, &runtime, &components, &on, false, false).unwrap();
        assert_eq!(
            fs::read(runtime.join(XIDI_DLL)).unwrap(),
            b"bundle dinput8.dll"
        );
        assert!(runtime.join("Xidi.ini").is_file());

        let off = Settings {
            controller: Some(false),
            ..Settings::default()
        };
        install(&game, &runtime, &components, &off, false, false).unwrap();
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
        )
        .unwrap();
        fs::write(game.join(XIDI_DLL), b"source proxy").unwrap();
        let off = Settings {
            controller: Some(false),
            ..Settings::default()
        };
        install(&game, &runtime, &components, &off, false, false).unwrap();
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
        )
        .unwrap();
        fs::write(runtime.join(XIDI_DLL), b"user changed").unwrap();
        fs::write(game.join(XIDI_DLL), b"source proxy").unwrap();
        let off = Settings {
            controller: Some(false),
            ..Settings::default()
        };
        let error = install(&game, &runtime, &components, &off, false, false).unwrap_err();
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
