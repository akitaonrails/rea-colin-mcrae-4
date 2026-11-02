//! A private writable copy of the game; the installed source is only read.

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub const MANIFEST: &str = ".cmr4-manifest";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stamp {
    pub size: u64,
    pub mtime: u64,
}

pub type Listing = BTreeMap<String, Stamp>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    pub copy: Vec<String>,
    pub manifest: Listing,
}

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

pub fn plan(source: &Listing, runtime_has: &dyn Fn(&str) -> bool, manifest: &Listing) -> Plan {
    let mut copy = Vec::new();
    let mut next = Listing::new();
    for (rel, stamp) in source {
        let present = runtime_has(rel);
        let changed = manifest.get(rel) != Some(stamp);
        if !present || (changed && !is_persistent(rel)) {
            copy.push(rel.clone());
            next.insert(rel.clone(), *stamp);
        } else {
            next.insert(rel.clone(), *manifest.get(rel).unwrap_or(stamp));
        }
    }
    Plan {
        copy,
        manifest: next,
    }
}

pub fn parse_manifest(text: &str) -> Listing {
    text.lines()
        .filter_map(|line| {
            let mut parts = line.splitn(3, '\t');
            let size = parts.next()?.parse().ok()?;
            let mtime = parts.next()?.parse().ok()?;
            let rel = parts.next()?.to_owned();
            Some((rel, Stamp { size, mtime }))
        })
        .collect()
}

pub fn format_manifest(listing: &Listing) -> String {
    listing
        .iter()
        .map(|(rel, stamp)| format!("{}\t{}\t{}\n", stamp.size, stamp.mtime, rel))
        .collect()
}

fn mtime_secs(metadata: &fs::Metadata) -> u64 {
    metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |duration| duration.as_secs())
}

pub fn list(root: &Path) -> io::Result<Listing> {
    fn walk(root: &Path, dir: &Path, out: &mut Listing) -> io::Result<()> {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            let metadata = entry.metadata()?;
            if metadata.is_dir() {
                walk(root, &path, out)?;
            } else if metadata.is_file() {
                let rel = path
                    .strip_prefix(root)
                    .expect("walk stays below root")
                    .components()
                    .map(|part| part.as_os_str().to_string_lossy())
                    .collect::<Vec<_>>()
                    .join("/");
                out.insert(
                    rel,
                    Stamp {
                        size: metadata.len(),
                        mtime: mtime_secs(&metadata),
                    },
                );
            }
        }
        Ok(())
    }
    let mut out = Listing::new();
    walk(root, root, &mut out)?;
    Ok(out)
}

pub fn native(root: &Path, rel: &str) -> PathBuf {
    rel.split('/')
        .fold(root.to_path_buf(), |path, part| path.join(part))
}

fn normalize(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                if !normalized.pop() && !normalized.has_root() {
                    normalized.push("..");
                }
            }
            _ => normalized.push(component.as_os_str()),
        }
    }
    normalized
}

/// Resolves a path even when its final components do not exist. Canonicalizing
/// the nearest existing ancestor resolves symlinks and Windows path aliases;
/// normalizing the appended suffix then handles `.` and `..` lexically.
pub fn resolve(path: &Path, cwd: &Path) -> Result<PathBuf, String> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        cwd.join(path)
    };
    let mut ancestor = absolute.as_path();
    let mut suffix = Vec::new();
    loop {
        match fs::canonicalize(ancestor) {
            Ok(base) => {
                let mut resolved = base;
                for component in suffix.iter().rev() {
                    resolved.push(component);
                }
                return Ok(normalize(&resolved));
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                let Some(component) = ancestor.components().next_back() else {
                    return Err(format!("cannot resolve {}", path.display()));
                };
                match component {
                    std::path::Component::Normal(name) => suffix.push(name.to_os_string()),
                    std::path::Component::ParentDir => suffix.push("..".into()),
                    std::path::Component::CurDir => {}
                    _ => return Err(format!("cannot resolve {}: {error}", path.display())),
                }
                ancestor = ancestor
                    .parent()
                    .ok_or_else(|| format!("cannot resolve {}: {error}", path.display()))?;
            }
            Err(error) => return Err(format!("cannot resolve {}: {error}", path.display())),
        }
    }
}

#[cfg(windows)]
fn strip_os_prefix(value: &std::ffi::OsStr, prefix: &[u16]) -> Option<std::ffi::OsString> {
    use std::os::windows::ffi::{OsStrExt, OsStringExt};
    let wide: Vec<u16> = value.encode_wide().collect();
    wide.starts_with(prefix)
        .then(|| std::ffi::OsString::from_wide(&wide[prefix.len()..]))
}

#[cfg(unix)]
fn strip_os_prefix(value: &std::ffi::OsStr, prefix: &[u8]) -> Option<std::ffi::OsString> {
    use std::os::unix::ffi::{OsStrExt, OsStringExt};
    let bytes = value.as_bytes();
    bytes
        .starts_with(prefix)
        .then(|| std::ffi::OsString::from_vec(bytes[prefix.len()..].to_vec()))
}

/// Converts Win32 extended-length paths back to the ordinary drive/UNC form
/// understood by CMR04, without changing the path's textual content otherwise.
pub fn simplify_extended_length(path: &Path) -> PathBuf {
    #[cfg(windows)]
    const EXTENDED: &[u16] = &[92, 92, 63, 92];
    #[cfg(windows)]
    const UNC: &[u16] = &[85, 78, 67, 92];
    #[cfg(unix)]
    const EXTENDED: &[u8] = b"\\\\?\\";
    #[cfg(unix)]
    const UNC: &[u8] = b"UNC\\";
    if let Some(rest) = strip_os_prefix(path.as_os_str(), EXTENDED) {
        if let Some(unc) = strip_os_prefix(&rest, UNC) {
            let mut simplified = std::ffi::OsString::from("\\\\");
            simplified.push(unc);
            return PathBuf::from(simplified);
        }
        return PathBuf::from(rest);
    }
    path.to_path_buf()
}

fn check_separate_from(game: &Path, runtime: &Path, cwd: &Path) -> Result<(), String> {
    let absolute_game = if game.is_absolute() {
        game.to_path_buf()
    } else {
        cwd.join(game)
    };
    let game = fs::canonicalize(&absolute_game)
        .map_err(|error| format!("game folder {}: {error}", game.display()))?;
    let runtime = resolve(runtime, cwd)?;
    if runtime.starts_with(&game) || game.starts_with(&runtime) {
        return Err("the runtime folder must be outside the installed game folder".into());
    }
    Ok(())
}

pub fn check_separate(game: &Path, runtime: &Path) -> Result<(), String> {
    let cwd = std::env::current_dir().map_err(|error| format!("current directory: {error}"))?;
    check_separate_from(game, runtime, &cwd)
}

fn make_writable(path: &Path) -> io::Result<()> {
    let mut permissions = fs::metadata(path)?.permissions();
    #[allow(clippy::permissions_set_readonly_false)]
    permissions.set_readonly(false);
    fs::set_permissions(path, permissions)
}

/// Where a replacement for `path` is prepared: beside it, so the final
/// rename stays on one volume.
fn temp_beside(path: &Path) -> PathBuf {
    let mut name = std::ffi::OsString::from(".");
    name.push(path.file_name().unwrap_or_default());
    name.push(".cmr4-new");
    path.with_file_name(name)
}

/// Replaces `path` with `bytes` through a temp file and a rename, so a crash
/// or a full disk mid-write leaves the old file whole instead of a torn one.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    if let Some(dir) = path.parent().filter(|dir| !dir.as_os_str().is_empty()) {
        fs::create_dir_all(dir)?;
    }
    let tmp = temp_beside(path);
    let _ = fs::remove_file(&tmp);
    let result = fs::File::create(&tmp)
        .and_then(|mut file| {
            io::Write::write_all(&mut file, bytes)?;
            file.sync_all()
        })
        .and_then(|()| match fs::rename(&tmp, path) {
            // Windows refuses to rename over a read-only file: make it
            // writable (it is the launcher's own copy), then rename.
            Err(error) if error.kind() == io::ErrorKind::PermissionDenied && path.exists() => {
                make_writable(path)?;
                fs::rename(&tmp, path)
            }
            other => other,
        });
    if result.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    result
}

/// Copies new files and changed non-persistent source files. It never deletes
/// runtime-only files and never overwrites persistent game-owned data.
pub fn sync(game: &Path, runtime: &Path) -> io::Result<usize> {
    let source = list(game)?;
    fs::create_dir_all(runtime)?;
    let manifest_path = runtime.join(MANIFEST);
    let manifest = fs::read_to_string(&manifest_path)
        .map(|text| parse_manifest(&text))
        .unwrap_or_default();
    let plan = plan(&source, &|rel| native(runtime, rel).is_file(), &manifest);
    for rel in &plan.copy {
        let from = native(game, rel);
        let to = native(runtime, rel);
        if let Some(parent) = to.parent() {
            fs::create_dir_all(parent)?;
        }
        if to.exists() {
            make_writable(&to)?;
        }
        fs::copy(&from, &to)?;
        make_writable(&to)?;
        let stamp = source[rel];
        let file = fs::OpenOptions::new().write(true).open(&to)?;
        file.set_modified(SystemTime::UNIX_EPOCH + Duration::from_secs(stamp.mtime))?;
    }
    write_atomic(&manifest_path, format_manifest(&plan.manifest).as_bytes())?;
    Ok(plan.copy.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stamp(size: u64, mtime: u64) -> Stamp {
        Stamp { size, mtime }
    }

    fn listing(items: &[(&str, Stamp)]) -> Listing {
        items
            .iter()
            .map(|(rel, stamp)| (rel.to_string(), *stamp))
            .collect()
    }

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
        let plan = plan(&new, &|_| true, &old);
        assert_eq!(plan.copy, ["Game/a.dat"]);
        assert_eq!(plan.manifest["OPTIONS"], stamp(1, 1));
    }

    #[test]
    fn missing_persistent_file_is_restored_once() {
        let source = listing(&[("OPTIONS", stamp(1, 1))]);
        assert_eq!(plan(&source, &|_| false, &source).copy, ["OPTIONS"]);
    }

    #[test]
    fn manifest_round_trips_with_spaces() {
        let listing = listing(&[("Game/a file.dat", stamp(10, 20))]);
        assert_eq!(parse_manifest(&format_manifest(&listing)), listing);
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
        let source_before = list(&game).unwrap();

        assert_eq!(sync(&game, &runtime).unwrap(), 4);
        assert_eq!(sync(&game, &runtime).unwrap(), 0);
        assert_eq!(list(&game).unwrap(), source_before);
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
        let source_before_sync = list(&game).unwrap();

        assert_eq!(sync(&game, &runtime).unwrap(), 1);
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
            list(&game).unwrap(),
            source_before_sync,
            "source content unchanged"
        );

        fs::remove_dir_all(game).unwrap();
        fs::remove_dir_all(runtime).unwrap();
    }

    #[test]
    fn overlapping_folders_are_refused() {
        let game = temp("overlap");
        fs::create_dir_all(&game).unwrap();
        assert!(check_separate(&game, &game.join("runtime")).is_err());
        assert!(check_separate(&game, &temp("elsewhere")).is_ok());
        fs::remove_dir_all(game).unwrap();
    }

    #[test]
    fn nonexistent_relative_and_dotdot_children_are_refused_without_creation() {
        let game = temp("relative-overlap");
        fs::create_dir_all(&game).unwrap();
        let accepted = resolve(Path::new("../outside-runtime"), &game).unwrap();
        assert!(accepted.is_absolute());
        assert_eq!(accepted, game.parent().unwrap().join("outside-runtime"));
        assert!(!accepted.exists());
        assert!(check_separate_from(&game, Path::new("child"), &game).is_err());
        assert!(check_separate_from(&game, Path::new("missing/../child"), &game).is_err());
        assert!(!game.join("child").exists());
        assert!(check_separate_from(&game, Path::new("../outside"), &game).is_ok());
        fs::remove_dir_all(game).unwrap();
    }

    #[test]
    fn extended_length_windows_paths_become_drive_and_unc_paths() {
        assert_eq!(
            simplify_extended_length(Path::new(r"\\?\C:\Games\CMR04")),
            PathBuf::from(r"C:\Games\CMR04")
        );
        assert_eq!(
            simplify_extended_length(Path::new(r"\\?\UNC\server\share\CMR04")),
            PathBuf::from(r"\\server\share\CMR04")
        );
        let ordinary = Path::new(r"C:\Games\CMR04");
        assert_eq!(simplify_extended_length(ordinary), ordinary);
    }

    #[cfg(unix)]
    #[test]
    fn aliased_existing_parent_is_resolved_before_containment_check() {
        let game = temp("alias-game");
        let alias = temp("alias-link");
        fs::create_dir_all(&game).unwrap();
        std::os::unix::fs::symlink(&game, &alias).unwrap();
        assert!(check_separate(&game, &alias.join("new-runtime")).is_err());
        assert!(!game.join("new-runtime").exists());
        fs::remove_file(alias).unwrap();
        fs::remove_dir_all(game).unwrap();
    }

    #[test]
    fn write_atomic_replaces_whole_files_and_leaves_no_temp() {
        let dir = temp("atomic");
        let path = dir.join("sub").join("OPTIONS");
        write_atomic(&path, b"first").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"first");
        // A read-only previous copy (as copied from read-only media) is replaced.
        let mut permissions = fs::metadata(&path).unwrap().permissions();
        permissions.set_readonly(true);
        fs::set_permissions(&path, permissions).unwrap();
        write_atomic(&path, b"second, longer").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"second, longer");
        let names: Vec<_> = fs::read_dir(path.parent().unwrap())
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect();
        assert_eq!(names, ["OPTIONS"]);
        // A failed write leaves the old file whole: the temp name is taken
        // by a directory, so the new content can't be prepared.
        fs::create_dir_all(temp_beside(&path).join("blocker")).unwrap();
        assert!(write_atomic(&path, b"torn").is_err());
        assert_eq!(fs::read(&path).unwrap(), b"second, longer");
        fs::remove_dir_all(dir).unwrap();
    }
}
