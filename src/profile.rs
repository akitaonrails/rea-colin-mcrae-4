//! The game's `OPTIONS` file: player profiles and controller setup, as CMR04
//! writes it (987,056 bytes, no checksum).
//!
//! A new profile drives with the keyboard: the pad only works once the player
//! picks it in Options → Controllers, sets Accel/brake to Analogue and binds
//! the triggers, and picking the controller there can freeze the game under
//! Wine. With the Xidi gamepad on, the launcher makes that choice for a
//! profile whose pad setup is still the factory one, the way the CMR3
//! launcher does for its `CMR3` profile. Anything set up by hand is left
//! alone.
//!
//! Layout, from diffs of files the game wrote (factory profile, then the same
//! profile after choosing the controller and analogue pedals by hand):
//! - Player blocks start every 0x29C40 bytes; player 1's controller block is
//!   at 0x4AA4, players 2 to 4 repeat it.
//! - 0x4AA4 byte 0: steering mode (1 analogue); byte 1: accel/brake mode
//!   (2 digital, 0 analogue).
//! - 0x4ADC: the pad's button per action (accelerate, brake, handbrake, ...),
//!   1 to 9 in a factory profile. Analogue pedals clear accelerate (-1) and
//!   keep brake's slot at 1.
//! - 0x4B00, 0x4B04: the pedal axis, 16 for the axis Xidi feeds from RT/LT
//!   (shown as "Z axis" in the game's menu).
//! - 0x4B18, 0x4B1C: the pad's USB product and vendor IDs. The game picks
//!   the controller whose IDs match when it starts; no match means Keyboard.
//! - 0x4BE0: two axis-tuning records (centre, deadzone, range, sensitivity),
//!   all zero until a pad axis is configured.
//! - 0xA71AC: player 1's input device type, 0 keyboard and 2 controller.
//!   Without it the game stays on Keyboard even when the IDs match.
//!
//! Verified in the game on 2026-11-02: a factory profile written by this
//! module started a stage on the pad (stick steering, RT/LT pedals) without
//! visiting Options.

pub const SIZE: usize = 987_056;
const STRIDE: usize = 0x29C40;
const PLAYERS: usize = 4;
const MODES: usize = 0x4AA4;
const BUTTONS: usize = 0x4ADC;
const PEDAL_AXIS: usize = 0x4B00;
const DEVICE: usize = 0x4B18;
const TUNING: usize = 0x4BE0;
const TUNING_LEN: usize = 0x40;
const INPUT_DEVICE: usize = 0xA71AC;
const KEYBOARD: u8 = 0;
const CONTROLLER: u8 = 2;

const DIGITAL_PEDALS: u8 = 2;
const ANALOGUE_PEDALS: u8 = 0;
const ANALOGUE_STEERING: u8 = 1;
const XIDI_PEDAL_AXIS: i32 = 16;
/// Centre, deadzone, range and sensitivity, as the game stores them after
/// the player sets up a pad, for the steering and pedal axes.
const TUNING_RECORD: [i32; 8] = [32767, -8000, 8000, -32000, 32000, 0, 100, 0];

fn word(profile: &[u8], at: usize) -> i32 {
    i32::from_le_bytes(profile[at..at + 4].try_into().unwrap())
}

fn set_word(profile: &mut [u8], at: usize, value: i32) {
    profile[at..at + 4].copy_from_slice(&value.to_le_bytes());
}

/// True when player 1's pad setup is still the factory one: on the
/// keyboard, digital pedals, buttons 1 to 9 and no axis tuning.
pub fn is_untouched(profile: &[u8]) -> bool {
    profile.len() == SIZE
        && profile[INPUT_DEVICE] == KEYBOARD
        && profile[MODES + 1] == DIGITAL_PEDALS
        && (0..9).all(|i| word(profile, BUTTONS + 4 * i) == i as i32 + 1)
        && profile[TUNING..TUNING + TUNING_LEN].iter().all(|&b| b == 0)
}

/// Gives an untouched profile the pad with these USB IDs, analogue steering
/// and analogue pedals on Xidi's trigger axis. Returns whether it changed.
pub fn choose_gamepad(profile: &mut [u8], vendor: u16, product: u16) -> bool {
    if !is_untouched(profile) {
        return false;
    }
    profile[INPUT_DEVICE] = CONTROLLER;
    profile[MODES] = ANALOGUE_STEERING;
    profile[MODES + 1] = ANALOGUE_PEDALS;
    set_word(profile, BUTTONS, -1);
    set_word(profile, BUTTONS + 4, 1);
    set_word(profile, PEDAL_AXIS, XIDI_PEDAL_AXIS);
    set_word(profile, PEDAL_AXIS + 4, XIDI_PEDAL_AXIS);
    for (i, value) in TUNING_RECORD.iter().chain(&TUNING_RECORD).enumerate() {
        set_word(profile, TUNING + 4 * i, *value);
    }
    for player in 0..PLAYERS {
        let block = player * STRIDE;
        profile[block + MODES] = ANALOGUE_STEERING;
        set_word(profile, block + DEVICE, i32::from(product));
        set_word(profile, block + DEVICE + 4, i32::from(vendor));
    }
    true
}

/// Applies `choose_gamepad` to the file at `path` when it exists.
pub fn choose_gamepad_in(
    path: &std::path::Path,
    vendor: u16,
    product: u16,
) -> std::io::Result<bool> {
    let mut profile = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error),
    };
    if !choose_gamepad(&mut profile, vendor, product) {
        return Ok(false);
    }
    // Whole or not at all: a torn OPTIONS would lose every player's profile.
    crate::runtime::write_atomic(path, &profile)?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A profile with the factory pad setup the game writes for a new player
    /// (only the fields this module reads or writes are filled in).
    fn factory() -> Vec<u8> {
        let mut profile = vec![0u8; SIZE];
        for player in 0..PLAYERS {
            let block = player * STRIDE;
            profile[block + MODES + 1] = DIGITAL_PEDALS;
            for i in 0..9 {
                set_word(&mut profile, block + BUTTONS + 4 * i, i as i32 + 1);
            }
            set_word(&mut profile, block + PEDAL_AXIS, 12);
            set_word(&mut profile, block + PEDAL_AXIS + 4, 12);
            set_word(&mut profile, block + DEVICE, 654);
            set_word(&mut profile, block + DEVICE + 4, -1);
        }
        profile
    }

    #[test]
    fn a_factory_profile_gets_the_pad_and_analogue_pedals() {
        let mut profile = factory();
        assert!(is_untouched(&profile));
        assert!(choose_gamepad(&mut profile, 0x2DC8, 0x310B));
        // The values the game itself saved after the same choice by hand.
        assert_eq!(profile[INPUT_DEVICE], CONTROLLER);
        assert_eq!(profile[MODES], 1);
        assert_eq!(profile[MODES + 1], 0);
        assert_eq!(word(&profile, BUTTONS), -1);
        assert_eq!(word(&profile, BUTTONS + 4), 1);
        assert_eq!(word(&profile, BUTTONS + 8), 3, "handbrake stays on X");
        assert_eq!(word(&profile, PEDAL_AXIS), 16);
        assert_eq!(word(&profile, PEDAL_AXIS + 4), 16);
        assert_eq!(word(&profile, TUNING), 32767);
        assert_eq!(word(&profile, TUNING + 0x20), 32767);
        assert_eq!(word(&profile, TUNING + 0x38), 100);
        for player in 0..PLAYERS {
            let block = player * STRIDE;
            assert_eq!(word(&profile, block + DEVICE), 12555);
            assert_eq!(word(&profile, block + DEVICE + 4), 11720);
            assert_eq!(profile[block + MODES], 1);
        }
        // Players 2 to 4 keep their own pedal setup.
        assert_eq!(profile[STRIDE + MODES + 1], DIGITAL_PEDALS);
    }

    #[test]
    fn a_configured_profile_is_left_alone() {
        let mut profile = factory();
        assert!(choose_gamepad(&mut profile, 0x2DC8, 0x310B));
        let once = profile.clone();
        assert!(!is_untouched(&profile));
        assert!(!choose_gamepad(&mut profile, 0x045E, 0x028E));
        assert_eq!(profile, once);

        // A chosen controller, hand-set analogue pedals, rebound buttons or
        // tuned axes all count as set up by hand.
        for change in [
            (INPUT_DEVICE, CONTROLLER),
            (MODES + 1, ANALOGUE_PEDALS),
            (BUTTONS + 8, 7),
            (TUNING + 2, 1),
        ] {
            let mut profile = factory();
            profile[change.0] = change.1;
            let before = profile.clone();
            assert!(!choose_gamepad(&mut profile, 0x2DC8, 0x310B), "{change:?}");
            assert_eq!(profile, before);
        }
    }

    #[test]
    fn other_files_are_never_changed() {
        for size in [0, SIZE - 1, SIZE + 1] {
            let mut profile = factory();
            profile.resize(size, 0);
            let before = profile.clone();
            assert!(!choose_gamepad(&mut profile, 0x2DC8, 0x310B));
            assert_eq!(profile, before);
        }
    }

    #[test]
    fn the_file_is_written_only_when_it_changes() {
        let dir = std::env::temp_dir().join(format!("cmr4-profile-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("OPTIONS");
        assert!(
            !choose_gamepad_in(&path, 0x2DC8, 0x310B).unwrap(),
            "missing file"
        );
        std::fs::write(&path, factory()).unwrap();
        assert!(choose_gamepad_in(&path, 0x2DC8, 0x310B).unwrap());
        assert!(!choose_gamepad_in(&path, 0x2DC8, 0x310B).unwrap());
        assert_eq!(word(&std::fs::read(&path).unwrap(), DEVICE), 12555);
        std::fs::remove_dir_all(dir).unwrap();
    }
}
