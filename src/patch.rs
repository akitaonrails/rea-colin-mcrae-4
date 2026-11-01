//! The tested CMR4 16:9 Low patch, applied only to a private executable copy.

use sha2::{Digest, Sha256};

pub const ORIGINAL: [u8; 6] = [0x04, 0x00, 0x00, 0x80, 0x3f, 0xe8];
pub const WIDESCREEN: [u8; 6] = [0x04, 0x00, 0x00, 0x40, 0x3f, 0xe8];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Patched,
    AlreadyPatched,
    Restored,
    AlreadyOriginal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatchError {
    pub sha256: String,
    pub original_count: usize,
    pub patched_count: usize,
}

impl std::fmt::Display for PatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.original_count + self.patched_count == 0 {
            write!(
                f,
                "the CMR4 widescreen signature is missing; SHA-256: {}",
                self.sha256
            )
        } else {
            write!(
                f,
                "the CMR4 widescreen signature is ambiguous ({} original, {} patched); SHA-256: {}",
                self.original_count, self.patched_count, self.sha256
            )
        }
    }
}

pub fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn occurrences(haystack: &[u8], needle: &[u8]) -> Vec<usize> {
    haystack
        .windows(needle.len())
        .enumerate()
        .filter_map(|(at, window)| (window == needle).then_some(at))
        .collect()
}

/// Applies or reverses the one tested signature. Both already-patched and
/// already-original inputs are successful, so repeated launches are idempotent.
pub fn widescreen(bytes: &mut [u8], enable: bool) -> Result<Outcome, PatchError> {
    let original = occurrences(bytes, &ORIGINAL);
    let patched = occurrences(bytes, &WIDESCREEN);
    let error = || PatchError {
        sha256: sha256(bytes),
        original_count: original.len(),
        patched_count: patched.len(),
    };
    match (enable, original.as_slice(), patched.as_slice()) {
        (true, &[at], []) => {
            bytes[at..at + ORIGINAL.len()].copy_from_slice(&WIDESCREEN);
            Ok(Outcome::Patched)
        }
        (true, [], &[_]) => Ok(Outcome::AlreadyPatched),
        (false, [], &[at]) => {
            bytes[at..at + WIDESCREEN.len()].copy_from_slice(&ORIGINAL);
            Ok(Outcome::Restored)
        }
        (false, &[_], []) => Ok(Outcome::AlreadyOriginal),
        _ => Err(error()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn patches_and_restores_the_exact_signature() {
        let mut bytes = [0x90, 0x90, 0x04, 0, 0, 0x80, 0x3f, 0xe8, 0x90];
        assert_eq!(widescreen(&mut bytes, true), Ok(Outcome::Patched));
        assert_eq!(&bytes[2..8], &WIDESCREEN);
        assert_eq!(widescreen(&mut bytes, true), Ok(Outcome::AlreadyPatched));
        assert_eq!(widescreen(&mut bytes, false), Ok(Outcome::Restored));
        assert_eq!(&bytes[2..8], &ORIGINAL);
        assert_eq!(widescreen(&mut bytes, false), Ok(Outcome::AlreadyOriginal));
    }

    #[test]
    fn patched_source_can_be_restored() {
        let mut bytes = WIDESCREEN;
        assert_eq!(widescreen(&mut bytes, false), Ok(Outcome::Restored));
        assert_eq!(bytes, ORIGINAL);
    }

    #[test]
    fn missing_and_ambiguous_signatures_fail_with_a_digest() {
        let mut missing = [0; 16];
        let error = widescreen(&mut missing, true).unwrap_err();
        assert_eq!(error.sha256, sha256(&missing));
        assert!(error.to_string().contains("missing"));

        let mut twice = [ORIGINAL, ORIGINAL].concat();
        let before = twice.clone();
        let error = widescreen(&mut twice, true).unwrap_err();
        assert_eq!(error.original_count, 2);
        assert_eq!(error.sha256, sha256(&before));
        assert_eq!(twice, before, "a failed patch changes nothing");

        let mut mixed = [ORIGINAL, WIDESCREEN].concat();
        assert_eq!(widescreen(&mut mixed, false).unwrap_err().patched_count, 1);
    }
}
