//! The tested CMR4 16:9 Low patch, applied only to a private executable copy.
//!
//! The one-match signature toggle is `rea_launcher::patch::signature_toggle`;
//! the signature and the messages are CMR4's.

use rea_launcher::identify::hex;
use rea_launcher::patch::{self, PatchError as SdkError};

pub use rea_launcher::patch::Toggle as Outcome;

pub const ORIGINAL: [u8; 6] = [0x04, 0x00, 0x00, 0x80, 0x3f, 0xe8];
pub const WIDESCREEN: [u8; 6] = [0x04, 0x00, 0x00, 0x40, 0x3f, 0xe8];

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

/// Lower-case hex SHA-256 of `bytes`.
pub fn sha256(bytes: &[u8]) -> String {
    hex(&rea_launcher::identify::sha256(bytes))
}

/// Applies or reverses the one tested signature. Both already-patched and
/// already-original inputs are successful, so repeated launches are idempotent.
pub fn widescreen(bytes: &mut [u8], enable: bool) -> Result<Outcome, PatchError> {
    patch::signature_toggle(bytes, &ORIGINAL, &WIDESCREEN, enable).map_err(|error| match error {
        SdkError::Signature {
            original,
            patched,
            sha256,
        } => PatchError {
            sha256: hex(&sha256),
            original_count: original,
            patched_count: patched,
        },
        other => unreachable!("signature_toggle only reports signatures: {other}"),
    })
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
        assert_eq!(
            error.to_string(),
            format!(
                "the CMR4 widescreen signature is missing; SHA-256: {}",
                sha256(&missing)
            )
        );

        let mut twice = [ORIGINAL, ORIGINAL].concat();
        let before = twice.clone();
        let error = widescreen(&mut twice, true).unwrap_err();
        assert_eq!(error.original_count, 2);
        assert_eq!(error.sha256, sha256(&before));
        assert!(
            error
                .to_string()
                .contains("ambiguous (2 original, 0 patched)")
        );
        assert_eq!(twice, before, "a failed patch changes nothing");

        let mut mixed = [ORIGINAL, WIDESCREEN].concat();
        assert_eq!(widescreen(&mut mixed, false).unwrap_err().patched_count, 1);
    }

    #[test]
    fn outcomes_log_with_the_names_older_launchers_wrote() {
        assert_eq!(format!("{:?}", Outcome::AlreadyPatched), "AlreadyPatched");
    }
}
