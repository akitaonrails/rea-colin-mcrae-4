//! What a launch writes, worked out without touching anything: the files
//! in the runtime, the registry values, the game's environment and the
//! patch sets. `main` carries out exactly this plan, so the end-to-end
//! option tests and the golden snapshots (`tests/snapshots/`) see what a
//! launch does.

use std::fmt::Write as _;
use std::path::Path;

use crate::components::{
    self, CLOSE_HOOK_CONFIG, CLOSE_HOOK_DLL, DGVOODOO_DLLS, DXVK_DLLS, Platform, XIDI_CONF,
    XIDI_DLL,
};
use crate::config::{Resolution, Settings};
use crate::registry::{self, RegWrite};

/// Windows only: where the fast-GPU preference for the runtime's
/// `cmr4.exe` goes (`rea_launcher_win32::gpu::prefer_fast_gpu`), set only
/// when the exe has no choice yet. README "Where things live" and
/// "Uninstall" name it.
pub const GPU_PREFERENCES_KEY: &str = r"HKCU\Software\Microsoft\DirectX\UserGpuPreferences";
/// The value: the high-performance GPU.
pub const FAST_GPU: &str = "GpuPreference=2;";

/// The `launcher.log` line for the fast-GPU preference of `exe`: `wrote`
/// when the launcher set it in this run, otherwise the existing choice
/// was kept.
pub fn fast_gpu_log_line(exe: &Path, wrote: bool) -> String {
    if wrote {
        format!(
            "registry: {GPU_PREFERENCES_KEY}: {} = {FAST_GPU} (high-performance GPU)",
            exe.display()
        )
    } else {
        format!(
            "registry: {GPU_PREFERENCES_KEY}: {} already has a GPU choice; kept",
            exe.display()
        )
    }
}

/// Everything one launch changes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    /// The patch sets applied to the runtime's `cmr4.exe`.
    pub patch_sets: Vec<&'static str>,
    /// The mode the game itself renders (4:3 under the widescreen fix).
    pub game_mode: Resolution,
    /// Bundled DLLs installed into the runtime.
    pub wrappers: Vec<&'static str>,
    /// Launcher DLLs taken out of the runtime (the game's own come back).
    pub taken_out: Vec<&'static str>,
    /// Generated files: (runtime-relative name, contents); None removes
    /// the launcher's own copy.
    pub files: Vec<(&'static str, Option<String>)>,
    /// Values under `registry::KEY` (32-bit view, or the VirtualStore).
    pub registry: Vec<RegWrite>,
    /// Seeded once under Wine's bus key, kept when the player set it.
    pub winebus: Vec<(&'static str, u32)>,
    /// The game's environment changes.
    pub env: Vec<(&'static str, String)>,
    /// Wine only: DLLs set to `native,builtin` in `cmr4.exe`'s per-program
    /// overrides (`HKCU\Software\Wine\AppDefaults\cmr4.exe\DllOverrides`),
    /// where no value exists yet.
    pub app_dll_overrides: Vec<&'static str>,
    /// Wine only: our per-program overrides taken back (a component
    /// turned off); a player's own value is kept.
    pub app_dll_overrides_removed: Vec<&'static str>,
    /// Arguments `cmr4.exe` gets (none: it takes none).
    pub args: Vec<String>,
    /// Windows only: the fast-GPU preference for the runtime's exe.
    pub prefer_fast_gpu: bool,
    /// The game profile is pointed at the connected pad when untouched.
    pub gamepad_profile: bool,
}

/// The plan for `settings` (resolved, with a resolution), with the
/// widescreen fix applied to the exe or not (`widescreen`), the
/// `runtime` folder, where the game runs, the registry's current
/// Width/Height and the player's own `WINEDLLOVERRIDES`.
pub fn plan(
    settings: &Settings,
    widescreen: bool,
    runtime: &Path,
    platform: &Platform,
    existing_size: Option<(u32, u32)>,
    inherited_overrides: Option<&str>,
) -> Plan {
    let controller = settings.controller();
    let game_mode = registry::game_mode(settings.resolution(), widescreen);
    let mut wrappers = DGVOODOO_DLLS.to_vec();
    wrappers.push(CLOSE_HOOK_DLL);
    let mut taken_out = Vec::new();
    if platform.wine {
        wrappers.extend(DXVK_DLLS);
    } else {
        taken_out.extend(DXVK_DLLS);
    }
    let mut files = vec![(
        "dgVoodoo.conf",
        Some(components::dgvoodoo_conf(settings, widescreen, platform)),
    )];
    if controller {
        wrappers.push(XIDI_DLL);
        files.push(("Xidi.ini", Some(XIDI_CONF.to_owned())));
    } else {
        taken_out.push(XIDI_DLL);
        files.push(("Xidi.ini", None));
    }
    // The close hook (components::CLOSE_HOOK_DLL, in the runtime exe's
    // imports) reads its setup from the game's environment, on Windows and
    // under Wine alike, and attaches itself to the game's window thread.
    let mut env = vec![
        (rea_close_hook::ENV, CLOSE_HOOK_CONFIG.to_owned()),
        (rea_close_hook::ATTACH_ENV, "1".to_owned()),
    ];
    let mut winebus = Vec::new();
    let mut app_dll_overrides = Vec::new();
    let mut app_dll_overrides_removed = Vec::new();
    if platform.wine {
        app_dll_overrides = components::app_override_dlls(controller);
        if !controller {
            app_dll_overrides_removed.push("dinput8");
        }
        env.push((
            rea_launcher::wine::DLL_OVERRIDES,
            components::wine_overrides(inherited_overrides, controller),
        ));
        if controller {
            winebus.extend(registry::WINEBUS_DEFAULTS);
        }
    }
    Plan {
        patch_sets: if widescreen {
            vec!["widescreen"]
        } else {
            Vec::new()
        },
        game_mode,
        wrappers,
        taken_out,
        files,
        registry: registry::writes(game_mode, runtime, existing_size),
        winebus,
        env,
        app_dll_overrides,
        app_dll_overrides_removed,
        args: Vec::new(),
        prefer_fast_gpu: !platform.wine,
        gamepad_profile: controller,
    }
}

impl Plan {
    /// The plan as text, for the golden snapshots and the log.
    pub fn render(&self) -> String {
        let mut out = String::new();
        let list = |items: &[&str]| {
            if items.is_empty() {
                "(none)".to_owned()
            } else {
                items.join(", ")
            }
        };
        let _ = writeln!(out, "patch sets: {}", list(&self.patch_sets));
        let _ = writeln!(out, "game renders: {}", self.game_mode.value());
        let _ = writeln!(out, "wrappers installed: {}", list(&self.wrappers));
        let _ = writeln!(out, "wrappers taken out: {}", list(&self.taken_out));
        let _ = writeln!(out, "arguments: {}", {
            let args: Vec<&str> = self.args.iter().map(String::as_str).collect();
            list(&args)
        });
        let _ = writeln!(out, "prefer fast GPU: {}", self.prefer_fast_gpu);
        let _ = writeln!(out, "gamepad profile setup: {}", self.gamepad_profile);
        let _ = writeln!(out, "\n[registry {}]", registry::KEY);
        for write in &self.registry {
            let _ = writeln!(out, "{}", write.describe());
        }
        let _ = writeln!(out, "\n[winebus, seeded when absent]");
        for (name, value) in &self.winebus {
            let _ = writeln!(out, "{name} = dword:{value}");
        }
        if !self.app_dll_overrides.is_empty() || !self.app_dll_overrides_removed.is_empty() {
            let _ = writeln!(
                out,
                "\n[Wine per-program DLL overrides for cmr4.exe, set when absent]"
            );
            for dll in &self.app_dll_overrides {
                let _ = writeln!(out, "{dll} = native,builtin");
            }
            for dll in &self.app_dll_overrides_removed {
                let _ = writeln!(out, "{dll}: ours removed");
            }
        }
        let _ = writeln!(out, "\n[environment]");
        for (name, value) in &self.env {
            let _ = writeln!(out, "{name}={value}");
        }
        for (name, contents) in &self.files {
            match contents {
                Some(text) => {
                    let _ = write!(out, "\n[file {name}]\n{text}");
                }
                None => {
                    let _ = writeln!(out, "\n[file {name}: removed when it is the launcher's]");
                }
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_fast_gpu_preference_is_named_in_the_log() {
        // Windows 10 test (rea-launcher-sdk#1, finding 8): the value was
        // written with no word in launcher.log.
        let exe = Path::new(r"C:\Users\p\AppData\Local\CMR4Launcher\CMR4\cmr4.exe");
        assert_eq!(
            fast_gpu_log_line(exe, true),
            r"registry: HKCU\Software\Microsoft\DirectX\UserGpuPreferences: C:\Users\p\AppData\Local\CMR4Launcher\CMR4\cmr4.exe = GpuPreference=2; (high-performance GPU)"
        );
        assert_eq!(
            fast_gpu_log_line(exe, false),
            r"registry: HKCU\Software\Microsoft\DirectX\UserGpuPreferences: C:\Users\p\AppData\Local\CMR4Launcher\CMR4\cmr4.exe already has a GPU choice; kept"
        );
    }
}
