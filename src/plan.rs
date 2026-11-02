//! What a launch writes, worked out without touching anything: the files
//! in the runtime, the registry values, the game's environment and the
//! patch sets. `main` carries out exactly this plan, so the end-to-end
//! option tests and the golden snapshots (`tests/snapshots/`) see what a
//! launch does.

use std::fmt::Write as _;
use std::path::Path;

use crate::components::{self, DGVOODOO_DLLS, DXVK_DLLS, Platform, XIDI_CONF, XIDI_DLL};
use crate::config::{Resolution, Settings};
use crate::registry::{self, RegWrite};

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
    let mut env = Vec::new();
    let mut winebus = Vec::new();
    if platform.wine {
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
