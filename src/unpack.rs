//! Unpacking the supported installer (`cmr4_launcher::source`) with the
//! bundled innoextract, behind the SDK's progress window. The installer is
//! never run, and its files are only read.

use std::path::Path;

use cmr4_launcher::source;
use cmr4_launcher::window::{METRICS, PALETTE, PHOTO_W, TITLE};
use rea_launcher::installer::Installer;
use rea_launcher_ui::unpack::{self, Unpacking};
use rea_launcher_ui::window::{SIZE, WindowFit};
use rea_launcher_win32::innoextract::{self, Unpacked};
use rea_launcher_win32::xinput;

use crate::win32;

/// Unpacks `found` into `dest` once (when `dest` already holds it, only
/// the game check runs) with `components`' innoextract.exe. `window`: a
/// player is watching, so the progress window opens (when it can't, the
/// unpack is waited for all the same). The result's `game` is the
/// unpacked `cmr4.exe`'s build.
pub fn run(
    found: &Installer,
    dest: &Path,
    components: &Path,
    window: bool,
    log: &mut dyn FnMut(&str),
) -> Result<Unpacked<&'static str>, String> {
    let exe = components.join(innoextract::EXE);
    innoextract::unpack(&exe, found, dest, source::check_unpacked, log, |job| {
        if !window {
            return Err("nobody is watching this run".to_owned());
        }
        let wine_scale = win32::wine_dpi_scale();
        let screen = win32::screen_size();
        let fit = WindowFit::new(SIZE, win32::is_wine(), screen, wine_scale).with_native_fit(true);
        unpack::show(
            &PALETTE,
            &METRICS,
            // No photo: the settings window's portrait carries its credit
            // line, which this window has no room for.
            unpack::Window {
                title: TITLE,
                fit,
                hero_jpeg: None,
                photo_width: PHOTO_W,
            },
            Unpacking::new(TITLE, found.build.name, found.build.unpacked),
            job,
            xinput::buttons,
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rea_launcher::installer;

    /// The owner's real repack, unpacked through this code path under
    /// `sdk/scripts/wine-test` (no window). Opt-in: set
    /// `CMR4_REAL_INSTALLER` (the repack's folder, only read),
    /// `CMR4_REAL_UNPACK` (the destination, in the repository's `.work/`)
    /// and `CMR4_REAL_COMPONENTS` (a folder with the pinned innoextract.exe).
    /// The comparison with an installed copy is done outside (RESEARCH.md,
    /// "Installers").
    #[test]
    fn the_owners_repack_unpacks_into_a_known_game() {
        let vars = [
            "CMR4_REAL_INSTALLER",
            "CMR4_REAL_UNPACK",
            "CMR4_REAL_COMPONENTS",
        ]
        .map(|name| std::env::var_os(name).map(std::path::PathBuf::from));
        let [Some(folder), Some(dest), Some(components)] = vars else {
            eprintln!("skipped: CMR4_REAL_INSTALLER, CMR4_REAL_UNPACK, CMR4_REAL_COMPONENTS unset");
            return;
        };
        let found = source::installer_in(&folder, source::INSTALLERS)
            .unwrap()
            .expect("the Magipack repack");
        assert_eq!(found.build.name, "the Magipack repack");
        let start = std::time::Instant::now();
        let mut log = |line: &str| eprintln!("{line}");
        let Unpacked::Ready(done) = run(&found, &dest, &components, false, &mut log).unwrap()
        else {
            panic!("cancelled without a window")
        };
        eprintln!("unpacked in {:.0} s", start.elapsed().as_secs_f64());
        assert_eq!(done.game, "cmr4.exe 1.0.0.0");
        assert_eq!(
            done.preferred,
            [
                "Data/Cars/P206/Two_play/p206.big#base",
                "Data/Cars/P206/p206.big#base"
            ]
        );
        assert_eq!(
            done.dropped,
            ["D3D9.dll", "dgVoodoo.conf", "dgVoodooCpl.exe"]
        );
        for (file, size) in [
            ("Data/Cars/P206/p206.big", 3_641_344),
            ("Data/Cars/P206/Two_play/p206.big", 772_096),
        ] {
            assert_eq!(
                std::fs::metadata(dest.join(file)).unwrap().len(),
                size,
                "{file}"
            );
        }
        assert!(installer::is_unpacked(&dest, &found));
        // A second start reuses it.
        let again = run(&found, &dest, &components, false, &mut |_| {}).unwrap();
        assert!(matches!(again, Unpacked::Ready(_)));
    }
}
