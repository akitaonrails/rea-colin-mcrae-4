//! Where the game comes from: an installed game (a folder with
//! `cmr4.exe`), or the one installer this launcher knows, which it unpacks
//! once into a folder of its own with the bundled innoextract and never
//! runs (`rea_launcher::installer`, sdk `docs/playbooks/installers.md`).
//!
//! Everything here is portable and tested on the host; the binary's
//! `unpack` module runs innoextract with the progress window.

use std::fs;
use std::path::{Path, PathBuf};

use rea_launcher::identify::{self, Known, validate_pe32};
use rea_launcher::installer::{self, Installer, InstallerBuild};
use rea_launcher::paths;

use crate::{GAME_EXE, edition};

/// Magipack's repack of Colin McRae Rally 04 (EN, FR, DE, ES, IT; "Patch
/// v1.01"), the owner's copy, verified on 2026-11-09 (`docs/RESEARCH.md`,
/// "Installers"): unpacked with these lists, it is byte-identical to an
/// installed game apart from the player's own saves.
pub const MAGIPACK: InstallerBuild = InstallerBuild {
    name: "the Magipack repack",
    setup: "Colin McRae Rally 04_Repack_Setup.exe",
    parts: &[
        ("Colin McRae Rally 04_Repack_Setup-1.bin", 735_348_224),
        ("Colin McRae Rally 04_Repack_Setup-2.bin", 736_000_000),
        ("Colin McRae Rally 04_Repack_Setup-3.bin", 461_475_044),
    ],
    unpacked: 2_675_404_801,
    root: "app",
    // An optional livery (the component `base\p206`, off by default)
    // writes the same two `p206.big` files as the default `base`; the
    // default install has `base`'s copies (3,641,344 and 772,096 bytes).
    prefer: &["base"],
    // The repack's own dgVoodoo setup: the launcher bundles the tested
    // dgVoodoo2 2.80 D3D9.dll and writes its own dgVoodoo.conf.
    drop: &["D3D9.dll", "dgVoodoo.conf", "dgVoodooCpl.exe"],
};

/// Every installer the launcher unpacks, by its setup program's SHA-256.
pub static INSTALLERS: &[Known<InstallerBuild>] = &[Known::new(
    "05ad4eb2d39672b1856413a74fb937d6fec9ebbd8dc09a3437ee3d2e89b9a96c",
    MAGIPACK,
)];

/// The folder's `cmr4.exe`, found ignoring case.
pub fn game_exe(dir: &Path) -> Option<PathBuf> {
    paths::find_ignoring_case(dir, GAME_EXE).filter(|path| path.is_file())
}

/// The installer in `dir` when it holds no `cmr4.exe` (an installed game
/// wins). `Ok(None)`: neither, and the launcher's usual "no cmr4.exe"
/// message applies. `Err`: an installer it doesn't know (left alone, with
/// its SHA-256 and where to report it), or a missing or short data part.
pub fn installer_in(
    dir: &Path,
    builds: &'static [Known<InstallerBuild>],
) -> Result<Option<Installer>, String> {
    if game_exe(dir).is_some() {
        return Ok(None);
    }
    installer::detect(dir, builds, edition::ISSUE_URL)
}

/// Where the game is unpacked: beside the private copy (`runtime`), on the
/// same drive, named after it (`…\CMR4Launcher\CMR4-game` by default).
pub fn unpacked_dir(runtime: &Path) -> PathBuf {
    let name = runtime.file_name().unwrap_or_default().to_string_lossy();
    runtime.with_file_name(format!("{name}-game"))
}

/// The launcher's check of an unpacked game: a `cmr4.exe` that is a build
/// it knows. Returns the build's name.
pub fn check_unpacked(dir: &Path) -> Result<&'static str, String> {
    let exe = game_exe(dir)
        .ok_or_else(|| format!("The installer gave no {GAME_EXE} in {}.", dir.display()))?;
    let bytes =
        fs::read(&exe).map_err(|error| format!("Cannot read {}: {error}", exe.display()))?;
    validate_pe32(&bytes).map_err(|error| format!("{}: {error}", exe.display()))?;
    identify::identify(&bytes, &edition::EDITIONS)
        .copied()
        .map_err(|_| {
            format!(
                "The {GAME_EXE} the installer gave is not a build this launcher knows (SHA-256 {}).",
                identify::hex(&identify::sha256(&bytes))
            )
        })
}

/// The settings window's note for a known installer.
pub fn found_note(found: &Installer) -> String {
    format!(
        "Found {}. Play unpacks the game from it once ({} free needed); the installer is never run and its files are only read.",
        found.build.name,
        gigabytes(installer::space_needed(found.build))
    )
}

/// The settings window's note for a folder the launcher unpacked.
pub fn unpacked_note(build: &str) -> String {
    format!("Found {build}, unpacked from your installer. Ready to play.")
}

/// Whether `dir` is a folder the launcher unpacked (it holds the SDK's
/// marker).
pub fn is_launchers_copy(dir: &Path) -> bool {
    dir.join(installer::MARKER).is_file()
}

/// What `--check` says about an installer in `game`, to be unpacked into
/// `dest` (`free`: the free bytes there, None when unknown). An error when
/// the first Play couldn't unpack it for lack of space.
pub fn check_text(
    found: &Installer,
    game: &Path,
    dest: &Path,
    free: Option<u64>,
) -> Result<String, String> {
    let head = format!(
        "Game folder: {}\nFound {}: {}, SHA-256 {}.",
        game.display(),
        found.build.name,
        found
            .setup
            .file_name()
            .unwrap_or_default()
            .to_string_lossy(),
        identify::hex(&found.sha256)
    );
    if installer::is_unpacked(dest, found) {
        return Ok(format!(
            "Ready to play.\n\n{head}\nIt is already unpacked into {}; Play uses that copy.",
            dest.display()
        ));
    }
    let parent = dest.parent().unwrap_or(dest);
    installer::check_space(found.build, free, parent).map_err(|error| {
        format!("{head}\n\n{error} Free some space, or choose another folder for the launcher's copy (runtime).")
    })?;
    let space = match free {
        Some(free) => format!(
            "it needs about {} free there ({} free now)",
            gigabytes(installer::space_needed(found.build)),
            gigabytes(free)
        ),
        None => format!(
            "it needs about {} free there",
            gigabytes(installer::space_needed(found.build))
        ),
    };
    Ok(format!(
        "Ready to play from the installer.\n\n{head}\nThe first Play unpacks the game once into {}, without running the installer; {space}. The installer's files are only read. Then the game folder setting points at the unpacked copy.\n\nNothing was unpacked by this check.",
        dest.display()
    ))
}

/// What the player is told after an unpack into `dest`. `from_cli`: the
/// game folder came from `--game`, so the settings file keeps what it had.
pub fn unpacked_message(dest: &Path, from_cli: bool) -> String {
    if from_cli {
        format!(
            "The game was unpacked into {}. The game folder came from --game, so the settings were not changed: next time use --game \"{}\". The installer files are no longer needed.",
            dest.display(),
            dest.display()
        )
    } else {
        format!(
            "The game was unpacked into {}. The game folder setting now points there; the installer files are no longer needed.",
            dest.display()
        )
    }
}

fn gigabytes(bytes: u64) -> String {
    format!("{:.1} GB", bytes as f64 / 1e9)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use rea_launcher::pe::{Format, synthetic};

    /// A stand-in for [`MAGIPACK`]: two small sparse parts, the setup
    /// program's bytes are `b"stand-in setup"`.
    pub(crate) static STAND_IN: InstallerBuild = InstallerBuild {
        name: "the stand-in repack",
        setup: "Stand-in_Setup.exe",
        parts: &[
            ("Stand-in_Setup-1.bin", 4096),
            ("Stand-in_Setup-2.bin", 100),
        ],
        unpacked: 2_000_000_000,
        root: "app",
        prefer: &["base"],
        drop: &["D3D9.dll"],
    };

    /// The stand-in's table, under the digest of its setup bytes.
    pub(crate) fn stand_in_table() -> &'static [Known<InstallerBuild>] {
        Box::leak(Box::new([Known {
            sha256: identify::sha256(b"stand-in setup"),
            value: STAND_IN,
        }]))
    }

    /// Writes the stand-in installer into `dir`: the setup program and
    /// sparse data parts of the right sizes.
    pub(crate) fn write_stand_in(dir: &Path) {
        fs::create_dir_all(dir).unwrap();
        fs::write(dir.join(STAND_IN.setup), b"stand-in setup").unwrap();
        for (name, size) in STAND_IN.parts {
            fs::File::create(dir.join(name))
                .unwrap()
                .set_len(*size)
                .unwrap();
        }
    }

    /// An Inno Setup installer nobody knows: an `.exe` with the signature.
    pub(crate) fn write_unknown_inno(dir: &Path) {
        fs::create_dir_all(dir).unwrap();
        let mut bytes = b"MZ fake installer ".to_vec();
        bytes.extend_from_slice(installer::INNO_SIGNATURE);
        bytes.extend_from_slice(b"5.5.0)");
        fs::write(dir.join("setup.exe"), bytes).unwrap();
    }

    /// An empty folder with a hostile name. The whole root is cleared, not
    /// only the folder: tests put siblings next to it (the unpack's
    /// `CMR4-game`), and Wine reuses process ids, so a later run in the
    /// same prefix found the earlier run's unpacked marker.
    fn temp(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!("cmr4-source-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let dir = root.join("Jogos d'Ação");
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn the_magipack_repack_is_the_one_known_installer() {
        assert_eq!(INSTALLERS.len(), 1);
        let known = &INSTALLERS[0];
        assert_eq!(
            identify::hex(&known.sha256),
            "05ad4eb2d39672b1856413a74fb937d6fec9ebbd8dc09a3437ee3d2e89b9a96c"
        );
        let build = &known.value;
        assert_eq!(build.setup, "Colin McRae Rally 04_Repack_Setup.exe");
        assert_eq!(
            build.parts,
            [
                ("Colin McRae Rally 04_Repack_Setup-1.bin", 735_348_224),
                ("Colin McRae Rally 04_Repack_Setup-2.bin", 736_000_000),
                ("Colin McRae Rally 04_Repack_Setup-3.bin", 461_475_044),
            ]
        );
        assert_eq!(build.unpacked, 2_675_404_801);
        assert_eq!(build.root, "app");
        // The default component's p206.big wins over the optional livery's.
        assert_eq!(build.prefer, ["base"]);
        // The repack's dgVoodoo goes; the launcher bundles its own.
        assert_eq!(build.drop, ["D3D9.dll", "dgVoodoo.conf", "dgVoodooCpl.exe"]);
        for name in build.drop {
            assert!(
                crate::components::DGVOODOO_DLLS.contains(name) || name.starts_with("dgVoodoo"),
                "{name} is not something the launcher bundles"
            );
        }
    }

    #[test]
    fn an_installed_game_wins_over_an_installer_beside_it() {
        let dir = temp("installed");
        write_stand_in(&dir);
        fs::write(dir.join("CMR4.EXE"), b"game").unwrap();
        assert_eq!(installer_in(&dir, stand_in_table()), Ok(None));
        assert!(game_exe(&dir).is_some());
    }

    #[test]
    fn the_known_installer_is_found_and_described() {
        let dir = temp("known");
        write_stand_in(&dir);
        let found = installer_in(&dir, stand_in_table()).unwrap().unwrap();
        assert_eq!(found.build, &STAND_IN);
        assert_eq!(
            found_note(&found),
            "Found the stand-in repack. Play unpacks the game from it once (2.2 GB free needed); the installer is never run and its files are only read."
        );
    }

    #[test]
    fn a_folder_with_neither_keeps_the_usual_message() {
        let dir = temp("empty");
        fs::write(dir.join("readme.txt"), b"hi").unwrap();
        assert_eq!(installer_in(&dir, stand_in_table()), Ok(None));
    }

    #[test]
    fn an_unknown_installer_is_left_alone_and_reported() {
        let dir = temp("unknown");
        write_unknown_inno(&dir);
        let error = installer_in(&dir, stand_in_table()).unwrap_err();
        assert!(
            error.starts_with(
                "setup.exe is an installer this launcher doesn't know, so it was left alone."
            ),
            "{error}"
        );
        assert!(error.contains("SHA-256: "), "{error}");
        assert!(error.contains(edition::ISSUE_URL), "{error}");
    }

    #[test]
    fn a_missing_part_is_named() {
        let dir = temp("part");
        write_stand_in(&dir);
        fs::remove_file(dir.join("Stand-in_Setup-2.bin")).unwrap();
        let error = installer_in(&dir, stand_in_table()).unwrap_err();
        assert!(
            error.starts_with("Stand-in_Setup-2.bin is missing in "),
            "{error}"
        );
    }

    #[test]
    fn the_unpacked_game_sits_beside_the_private_copy() {
        assert_eq!(
            unpacked_dir(Path::new(r"C:\Users\p\AppData\Local\CMR4Launcher\CMR4")),
            Path::new(r"C:\Users\p\AppData\Local\CMR4Launcher\CMR4-game")
        );
        assert_eq!(
            unpacked_dir(Path::new("/d/Rally copy")),
            Path::new("/d/Rally copy-game")
        );
    }

    #[test]
    fn the_unpacked_game_must_be_a_known_build() {
        let dir = temp("check");
        assert!(
            check_unpacked(&dir)
                .unwrap_err()
                .contains("gave no cmr4.exe")
        );
        let unknown = synthetic::image(
            Format::Pe32,
            0x40_0000,
            &[(".text", 0x1000, &[0xc3; 16], synthetic::CODE)],
        );
        fs::write(dir.join("cmr4.exe"), unknown).unwrap();
        let error = check_unpacked(&dir).unwrap_err();
        assert!(
            error.contains("is not a build this launcher knows (SHA-256 "),
            "{error}"
        );
        fs::write(dir.join("cmr4.exe"), b"not a program").unwrap();
        assert!(check_unpacked(&dir).is_err());
    }

    #[test]
    fn check_says_what_play_will_do_and_the_space_it_needs() {
        let dir = temp("check-text");
        write_stand_in(&dir);
        let found = installer_in(&dir, stand_in_table()).unwrap().unwrap();
        let dest = dir.parent().unwrap().join("CMR4-game");
        let text = check_text(&found, &dir, &dest, Some(50_000_000_000)).unwrap();
        assert!(
            text.starts_with("Ready to play from the installer.\n\n"),
            "{text}"
        );
        assert!(text.contains("Found the stand-in repack: Stand-in_Setup.exe, SHA-256 "));
        assert!(
            text.contains("without running the installer; it needs about 2.2 GB free there (50.0 GB free now)."),
            "{text}"
        );
        assert!(
            text.ends_with("Nothing was unpacked by this check."),
            "{text}"
        );
        assert!(!dest.exists(), "--check never unpacks");
        let unknown = check_text(&found, &dir, &dest, None).unwrap();
        assert!(
            unknown.contains("it needs about 2.2 GB free there."),
            "{unknown}"
        );

        let error = check_text(&found, &dir, &dest, Some(1_000_000_000)).unwrap_err();
        assert!(
            error.contains("Unpacking the game needs about 2.2 GB free in "),
            "{error}"
        );
        assert!(error.contains("there are 1.0 GB."), "{error}");

        // Unpacked before: Play reuses it.
        fs::create_dir_all(&dest).unwrap();
        fs::write(
            dest.join(installer::MARKER),
            identify::hex(&identify::sha256(b"stand-in setup")),
        )
        .unwrap();
        let text = check_text(&found, &dir, &dest, Some(0)).unwrap();
        assert!(
            text.ends_with(&format!(
                "It is already unpacked into {}; Play uses that copy.",
                dest.display()
            )),
            "{text}"
        );
        assert!(is_launchers_copy(&dest));
        assert!(!is_launchers_copy(&dir));
    }

    #[test]
    fn the_player_is_told_where_the_game_went() {
        let dest = Path::new(r"C:\Data\CMR4-game");
        assert_eq!(
            unpacked_message(dest, false),
            r"The game was unpacked into C:\Data\CMR4-game. The game folder setting now points there; the installer files are no longer needed."
        );
        assert_eq!(
            unpacked_message(dest, true),
            r#"The game was unpacked into C:\Data\CMR4-game. The game folder came from --game, so the settings were not changed: next time use --game "C:\Data\CMR4-game". The installer files are no longer needed."#
        );
    }
}
