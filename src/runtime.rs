//! A private writable copy of the game; the installed source is only read.
//!
//! The copy engine is `rea_launcher::runtime`; this module holds what CMR4
//! knows about its own files.

use std::io;
use std::path::{Path, PathBuf};

use rea_launcher::runtime::{self as sdk, Policy, Stamp, SyncReport};
use rea_launcher::{fsx, paths};

pub const MANIFEST: &str = ".cmr4-manifest";

/// The temp suffix launchers before the shared SDK used (`.name.cmr4-new`).
pub const LEGACY_TEMP_SUFFIX: &str = ".cmr4-new";

/// CMR4 writes save/settings files beside the executable, and players may
/// install DLL mods such as DSOAL. Once copied, the runtime owns these files
/// even if the installed source later changes.
pub fn is_persistent(rel: &str) -> bool {
    let upper = rel.to_ascii_uppercase();
    let top = upper.split('/').next().unwrap_or_default();
    let name = upper.rsplit('/').next().unwrap_or_default();
    top == "OPTIONS"
        || top == "SAVE"
        || top == "SAVES"
        || name.starts_with("CHAMPIONSHIP_SLOT_")
        || name.starts_with("BESTTIMES_GHOSTS")
        || name.ends_with(".DLL")
}

/// CMR4's runtime: a plain copy of everything (nothing is hard-linked), with
/// the game-written files of [`is_persistent`] copied once.
pub struct Cmr4;

impl Policy for Cmr4 {
    fn game_written(&self, rel: &str, _source: Stamp) -> bool {
        is_persistent(rel)
    }
}

/// Copies new files and changed non-persistent source files. It never deletes
/// runtime-only files and never overwrites persistent game-owned data.
pub fn sync(game: &Path, runtime: &Path) -> io::Result<SyncReport> {
    sdk::sync(game, runtime, &Cmr4, MANIFEST)
}

/// Removes the `.name.cmr4-new` temp files an interrupted write of an older
/// launcher left in the runtime; nothing when it does not exist yet.
pub fn remove_legacy_temps(runtime: &Path) -> io::Result<Vec<PathBuf>> {
    if !runtime.is_dir() {
        return Ok(Vec::new());
    }
    fsx::remove_stale_temps(runtime, &[LEGACY_TEMP_SUFFIX])
}

/// The game folder, absolute and resolved; an error when it does not exist.
pub fn game_folder(game: &Path, cwd: &Path) -> Result<PathBuf, String> {
    let absolute = cwd.join(game);
    std::fs::metadata(&absolute)
        .map_err(|error| format!("game folder {}: {error}", game.display()))?;
    paths::resolve(&absolute, cwd).map_err(|error| format!("game folder {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{Duration, SystemTime};

    fn temp(name: &str) -> PathBuf {
        let path =
            std::env::temp_dir().join(format!("cmr4-launcher-test-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        path
    }

    #[test]
    fn persistent_names_cover_cmr4_saves_and_settings() {
        for rel in [
            "OPTIONS",
            "CHAMPIONSHIP_SLOT_0",
            "CHAMPIONSHIP_SLOT_12.DAT",
            "BESTTIMES_GHOSTS",
            "save/profile.dat",
            "dsound.dll",
        ] {
            assert!(is_persistent(rel), "{rel}");
        }
        assert!(!is_persistent("Game/track.dat"));
        assert!(!is_persistent("MY_OPTIONS.txt"));
    }

    #[test]
    fn changed_assets_are_copied_but_persistent_files_are_kept() {
        let stamp = |size, mtime| Stamp { size, mtime };
        let listing = |items: &[(&str, Stamp)]| -> sdk::Listing {
            items.iter().map(|(r, s)| (r.to_string(), *s)).collect()
        };
        let old = listing(&[
            ("Game/a.dat", stamp(1, 1)),
            ("OPTIONS", stamp(1, 1)),
            ("CHAMPIONSHIP_SLOT_0", stamp(1, 1)),
        ]);
        let new = listing(&[
            ("Game/a.dat", stamp(2, 9)),
            ("OPTIONS", stamp(2, 9)),
            ("CHAMPIONSHIP_SLOT_0", stamp(2, 9)),
        ]);
        let plan = sdk::plan(&new, &|_| true, &old, &Cmr4);
        assert_eq!(plan.copy, ["Game/a.dat"]);
        assert_eq!(plan.manifest["OPTIONS"], stamp(1, 1));
        // A missing persistent file is restored once.
        assert_eq!(sdk::plan(&old, &|_| false, &old, &Cmr4).copy.len(), 3);
    }

    #[test]
    fn sync_preserves_runtime_data_and_never_writes_the_source() {
        let game = temp("game");
        let runtime = temp("runtime");
        fs::create_dir_all(game.join("Game")).unwrap();
        fs::write(game.join("cmr4.exe"), b"exe").unwrap();
        fs::write(game.join("Game/a.dat"), b"old").unwrap();
        fs::write(game.join("OPTIONS"), b"source options").unwrap();
        fs::write(game.join("CHAMPIONSHIP_SLOT_0"), b"source save").unwrap();
        let source_before = sdk::list(&game).unwrap();

        let first = sync(&game, &runtime).unwrap();
        assert_eq!(
            (first.brought, first.linked),
            (4, 0),
            "copied, never linked"
        );
        assert_eq!(sync(&game, &runtime).unwrap().brought, 0);
        assert_eq!(sdk::list(&game).unwrap(), source_before);
        // The manifest keeps the name and format older launchers wrote.
        let manifest = fs::read_to_string(runtime.join(MANIFEST)).unwrap();
        assert!(manifest.lines().any(|line| line.ends_with("\tGame/a.dat")));
        fs::write(runtime.join("OPTIONS"), b"player options").unwrap();
        fs::write(runtime.join("CHAMPIONSHIP_SLOT_0"), b"player save").unwrap();
        fs::write(runtime.join("BESTTIMES_GHOSTS"), b"runtime only").unwrap();
        fs::write(game.join("OPTIONS"), b"new source options").unwrap();
        fs::write(game.join("CHAMPIONSHIP_SLOT_0"), b"new source save").unwrap();
        fs::write(game.join("Game/a.dat"), b"new!").unwrap();
        let file = fs::OpenOptions::new()
            .write(true)
            .open(game.join("Game/a.dat"))
            .unwrap();
        file.set_modified(SystemTime::now() + Duration::from_secs(10))
            .unwrap();
        drop(file);
        let source_before_sync = sdk::list(&game).unwrap();

        assert_eq!(sync(&game, &runtime).unwrap().brought, 1);
        assert_eq!(
            fs::read(runtime.join("OPTIONS")).unwrap(),
            b"player options"
        );
        assert_eq!(
            fs::read(runtime.join("CHAMPIONSHIP_SLOT_0")).unwrap(),
            b"player save"
        );
        assert_eq!(
            fs::read(runtime.join("BESTTIMES_GHOSTS")).unwrap(),
            b"runtime only"
        );
        assert_eq!(fs::read(runtime.join("Game/a.dat")).unwrap(), b"new!");
        assert_eq!(
            sdk::list(&game).unwrap(),
            source_before_sync,
            "source content unchanged"
        );

        fs::remove_dir_all(game).unwrap();
        fs::remove_dir_all(runtime).unwrap();
    }

    #[test]
    fn a_manifest_from_an_older_launcher_is_read() {
        let game = temp("old-manifest-game");
        let runtime = temp("old-manifest-runtime");
        fs::create_dir_all(&game).unwrap();
        fs::create_dir_all(&runtime).unwrap();
        fs::write(game.join("OPTIONS"), b"source options").unwrap();
        let stamp = sdk::list(&game).unwrap()["OPTIONS"];
        // What v0.2.0 left: its manifest, the player's OPTIONS, a torn temp.
        fs::write(
            runtime.join(MANIFEST),
            format!("{}\t{}\tOPTIONS\n", stamp.size, stamp.mtime),
        )
        .unwrap();
        fs::write(runtime.join("OPTIONS"), b"player options").unwrap();
        fs::write(runtime.join(".OPTIONS.cmr4-new"), b"torn").unwrap();
        assert_eq!(
            remove_legacy_temps(&runtime).unwrap(),
            [runtime.join(".OPTIONS.cmr4-new")]
        );
        assert_eq!(sync(&game, &runtime).unwrap().brought, 0);
        assert_eq!(
            fs::read(runtime.join("OPTIONS")).unwrap(),
            b"player options"
        );
        assert!(remove_legacy_temps(&temp("not-yet")).unwrap().is_empty());
        fs::remove_dir_all(game).unwrap();
        fs::remove_dir_all(runtime).unwrap();
    }

    #[test]
    fn the_game_folder_must_exist_and_the_runtime_stay_outside_it() {
        let game = temp("overlap");
        fs::create_dir_all(&game).unwrap();
        let resolved = game_folder(Path::new("."), &game).unwrap();
        // Drive form on Windows (no `\\?\` prefix): what CMR04 accepts.
        let real = paths::simplify_extended_length(&fs::canonicalize(&game).unwrap());
        assert_eq!(resolved, real);
        assert!(game_folder(Path::new("gone"), &game).is_err());
        assert!(paths::check_separate(&resolved, &resolved.join("runtime")).is_err());
        assert!(paths::check_separate(&resolved, &temp("elsewhere")).is_ok());
        fs::remove_dir_all(game).unwrap();
    }
}
