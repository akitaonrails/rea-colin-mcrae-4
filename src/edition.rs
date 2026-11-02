//! Which `cmr4.exe` this is, and the one fix the launcher applies to it.
//!
//! Builds are identified by SHA-256 (`rea_launcher::identify`); the fix is
//! a `rea_launcher::patch::PatchSet`, applied from the pristine bytes to the
//! private runtime copy only. An unknown build is never patched: the game
//! runs as it is, without the widescreen view, and the player is asked to
//! report the digest.

use rea_launcher::identify::{self, Known, hex, hex32};
use rea_launcher::patch::{self, At, PatchSet, PatchSite};

/// `cmr4.exe` 1.0.0.0, 3,756,032 bytes, as installed (the build the
/// community 16:9 fix and this launcher were tested on).
pub const V1_0: [u8; 32] =
    hex32("7dca9f4cab16bc1a98df8871f3d20aec4cd931a818ea0899db86c83c1f712b41");
/// The same build with the widescreen fix applied (by this launcher, or
/// by hand with the community fix before).
pub const V1_0_WIDESCREEN: [u8; 32] =
    hex32("60e0267bebb5894aa4e2f2b791ca9e338c9469433c9b9192c5772174c1bcc0b2");

/// Every known `cmr4.exe`, by its own SHA-256.
pub static EDITIONS: [Known<&str>; 2] = [
    Known::new(
        "7dca9f4cab16bc1a98df8871f3d20aec4cd931a818ea0899db86c83c1f712b41",
        "cmr4.exe 1.0.0.0",
    ),
    Known::new(
        "60e0267bebb5894aa4e2f2b791ca9e338c9469433c9b9192c5772174c1bcc0b2",
        "cmr4.exe 1.0.0.0 with the 16:9 fix already applied",
    ),
];

/// The camera constant the community "16:9 Low" fix changes:
/// `mov dword [esp+4], 1.0f` → `0.75f` (VA 0x477826).
pub const ORIGINAL: [u8; 6] = [0x04, 0x00, 0x00, 0x80, 0x3f, 0xe8];
pub const PATCHED: [u8; 6] = [0x04, 0x00, 0x00, 0x40, 0x3f, 0xe8];

/// AuToMaNiAk005's 16:9 Low fix: the camera's horizontal factor 1.0 →
/// 0.75. Correct only when the game renders 4:3 and dgVoodoo stretches the
/// frame to 16:9 (`registry::game_mode`).
pub const WIDESCREEN: PatchSet = PatchSet {
    name: "widescreen",
    summary: "The full 16:9 view: the game draws a 4:3 frame that is stretched to fill a 16:9 screen",
    editions: &[V1_0],
    sites: &[PatchSite {
        at: At::Offset(0x07_7826),
        original: &ORIGINAL,
        patched: &PATCHED,
        note: "camera horizontal factor 1.0 → 0.75 (16:9 Low)",
    }],
    caves: &[],
    results: &[(V1_0, V1_0_WIDESCREEN)],
};

/// Where players report a build the launcher doesn't know.
pub const ISSUE_URL: &str =
    "https://github.com/akitaonrails/rea-colin-mcrae-4/issues/new?template=unknown-edition.yml";

/// What the launcher found and will run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Prepared {
    /// SHA-256 of the game's own `cmr4.exe`, lower-case hex.
    pub sha256: String,
    /// The build's name, or None for an unknown build.
    pub edition: Option<&'static str>,
    /// The bytes for the runtime's `cmr4.exe`.
    pub bytes: Vec<u8>,
    /// Whether `bytes` carry the widescreen fix.
    pub widescreen: bool,
}

impl Prepared {
    /// The line for `launcher.log`.
    pub fn describe(&self) -> String {
        format!(
            "cmr4.exe SHA-256 {}: {}; widescreen fix {}",
            self.sha256,
            self.edition.unwrap_or("unknown build"),
            if self.widescreen {
                "applied"
            } else {
                "not applied"
            },
        )
    }
}

/// The plain-language note for an unknown build, with the report link.
pub fn unknown_message(sha256: &str) -> String {
    format!(
        "This cmr4.exe is a version the launcher doesn't know yet, so the game runs without the widescreen fix (the picture keeps the game's own view). Please report it so it can be added: {ISSUE_URL} (include this code: {sha256})."
    )
}

/// The runtime's `cmr4.exe` from the game's `source` bytes: a known build
/// with the widescreen fix when `widescreen` is wanted, or without it; an
/// unknown build exactly as it is (never patched by guesswork).
pub fn prepare(source: &[u8], widescreen: bool) -> Prepared {
    let sha256 = hex(&identify::sha256(source));
    let edition = identify::identify(source, &EDITIONS).ok().copied();
    let fixed = if widescreen {
        patch::apply_sets(source, &[&WIDESCREEN])
    } else {
        patch::unpatch(source, &[&WIDESCREEN])
    };
    match fixed {
        Ok(bytes) => Prepared {
            sha256,
            edition: edition.or(Some("a known cmr4.exe")),
            bytes,
            widescreen,
        },
        Err(_) => Prepared {
            sha256,
            edition: None,
            bytes: source.to_vec(),
            widescreen: false,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A tiny stand-in with the site's original bytes at its offset (no
    /// game bytes): its hash is not a known build.
    fn synthetic() -> Vec<u8> {
        let mut bytes = vec![0x90; 0x07_7900];
        bytes[0x07_7826..0x07_782c].copy_from_slice(&ORIGINAL);
        bytes
    }

    #[test]
    fn the_site_has_golden_bytes_and_is_all_or_nothing() {
        patch::testing::assert_golden(&WIDESCREEN, &synthetic());
    }

    #[test]
    #[ignore = "needs CMR4_GAME_DIR"]
    fn the_real_build_patches_to_the_recorded_hash() {
        // CMR4_GAME_DIR=<game folder> cargo test -- --ignored
        patch::testing::assert_real_game("CMR4_GAME_DIR", &[&WIDESCREEN]);
    }

    #[test]
    fn an_unknown_build_runs_unchanged_with_its_digest() {
        let source = synthetic();
        for want in [true, false] {
            let prepared = prepare(&source, want);
            assert_eq!(prepared.edition, None);
            assert!(!prepared.widescreen);
            assert_eq!(prepared.bytes, source, "never patched by guesswork");
            assert_eq!(prepared.sha256, hex(&identify::sha256(&source)));
            assert!(prepared.describe().contains("unknown build"));
        }
        let message = unknown_message("abc");
        assert!(message.contains(ISSUE_URL) && message.contains("abc"));
        assert!(!message.contains("signature"));
    }

    #[test]
    fn the_table_names_both_known_digests() {
        assert_eq!(EDITIONS[0].sha256, V1_0);
        assert_eq!(EDITIONS[1].sha256, V1_0_WIDESCREEN);
        assert_eq!(WIDESCREEN.results, &[(V1_0, V1_0_WIDESCREEN)]);
        let rows = patch::describe(&WIDESCREEN, None).unwrap();
        assert_eq!(rows[0].offset, 0x07_7826);
        assert!(patch::to_markdown(&rows).contains("077826"));
    }
}
