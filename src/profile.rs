//! The game's `OPTIONS` file: player profiles and controller setup, as CMR04
//! writes it (987,056 bytes, no checksum).
//!
//! A new profile drives with the keyboard: the pad only works once the player
//! picks it in Options → Controllers, sets Accel/brake to Analogue and binds
//! the triggers, and picking the controller there can freeze the game under
//! Wine. The launcher makes that choice for a profile whose controls were
//! never set up: the pad (Xidi) with analogue pedals, or, when the in-game
//! wheel layer shows the game its virtual wheel, that wheel with separate
//! analogue pedals and force feedback. Anything set up by hand is left alone.
//!
//! Layout (diffs of files the game wrote, and cmr4.exe's loader and menus,
//! `docs/RESEARCH.md` "Wheels"):
//! - Player blocks start every 0x29C40 bytes. Each holds one 0x2520-byte
//!   record per controller slot from 0x64: slot 0 keyboard, 1 mouse, then
//!   the controllers in the order DirectInput lists them. Player 1's first
//!   controller (slot 2) is at 0x4AA4; the wheel layer lists its wheel
//!   first, which moves the pad to slot 3 (0x6FC4).
//! - In a record: byte 0 flags (attached, force feedback on, its strength),
//!   byte 1 pedal modes (2 digital, 0 analogue); 28 control codes from +4
//!   (the pad's buttons per action from +0x38, the pedal axes at +0x5C and
//!   +0x60); the device's USB product and vendor IDs at +0x74 and +0x78;
//!   axis tunings of 0x20 bytes per action from +0x7C; autocentre off at
//!   +0x251C.
//! - A record whose IDs don't match the device in its slot is reset to that
//!   device's defaults when the game loads the profile; so are the player's
//!   own tweaks of it. The launcher keeps a copy of each device's record in
//!   a [`Stash`] beside `OPTIONS` and puts it back when that device returns
//!   to a slot (a run without the wheel puts the pad back in slot 2).
//! - 0xA71AC: the slot player 1 drives with (dword), 0 the keyboard.
//!
//! Verified in the game on 2026-11-02: a factory profile written by this
//! module started a stage on the pad (stick steering, RT/LT pedals) without
//! visiting Options.

use std::collections::BTreeMap;
use std::fmt;
use std::io;
use std::path::Path;

use rea_launcher::fsx;
use rea_launcher::wheel::dinput::VIRTUAL;

pub const SIZE: usize = 987_056;
/// The launcher's copies of the players' controller records, beside
/// `OPTIONS` in the runtime.
pub const STASH_FILE: &str = ".cmr4-controller-records";

const STRIDE: usize = 0x29C40;
const PLAYERS: usize = 4;
const RECORDS: usize = 0x64;
const RECORD: usize = 0x2520;
const PLAYER_SLOT: usize = 0xA71AC;
const KEYBOARD: i32 = 0;
/// The first controller's slot: the pad's, or the virtual wheel's when the
/// layer shows it.
const FIRST_CONTROLLER: usize = 2;
/// The pad's slot behind the virtual wheel.
const PAD_BEHIND_WHEEL: usize = 3;

// Inside a record.
const MODES: usize = 0;
const CONTROLS: usize = 0x04;
const BUTTONS: usize = 0x38;
const PEDAL_AXIS: usize = 0x5C;
const DEVICE: usize = 0x74;
const TUNINGS: usize = 0x7C;
const TUNING_LEN: usize = 0x20;
/// The pad's steering and pedal tunings (actions 6 and 7).
const PAD_TUNING: usize = TUNINGS + 6 * TUNING_LEN;
const AUTOCENTRE_OFF: usize = 0x251C;

const DIGITAL_PEDALS: u8 = 2;
const ANALOGUE_PEDALS: u8 = 0;
const ANALOGUE_STEERING: u8 = 1;
const XIDI_PEDAL_AXIS: i32 = 16;
/// Centre, deadzone, range and sensitivity, as the game stores them after
/// the player sets up a pad, for the steering and pedal axes.
const TUNING_RECORD: [i32; 8] = [32767, -8000, 8000, -32000, 32000, 0, 100, 0];

/// The virtual wheel's control codes in the game: its 32 buttons, two for
/// its hat, then the axes in the order the layer lists them.
const STEER: i32 = 34;
const COMBINED_PEDALS: i32 = 35;
const THROTTLE: i32 = 36;
const BRAKE: i32 = 37;
/// The wheel's controls: the game's own defaults for a driving device,
/// with accelerate and brake on the separate pedal axes.
const WHEEL_CONTROLS: [i32; 28] = [
    COMBINED_PEDALS,
    COMBINED_PEDALS,
    -1,
    -1,
    STEER,
    STEER,
    COMBINED_PEDALS,
    COMBINED_PEDALS,
    STEER,
    STEER,
    COMBINED_PEDALS,
    COMBINED_PEDALS,
    0,
    1,
    2,
    3,
    4,
    5,
    6,
    7,
    8,
    9,
    THROTTLE,
    BRAKE,
    -1,
    -1,
    -1,
    -1,
];
/// Attached, force feedback on, strength step 4 of 0..=4 (full: the layer
/// applies the player's strength).
const WHEEL_FLAGS: u8 = 0x13;
/// Tunings per action: analogue steering (7), accelerate (9) and brake
/// (10), with no deadzone (a pedal reads 0 at rest, 32767 pressed).
const WHEEL_TUNINGS: [(usize, [i32; 8]); 3] = [
    (7, [32767, 0, 0, -32767, 32768, 0, 100, 0]),
    (9, [0, 0, 0, 0, 65535, 0, 100, 0]),
    (10, [0, 0, 0, 0, 65535, 0, 100, 0]),
];

fn word(profile: &[u8], at: usize) -> i32 {
    i32::from_le_bytes(profile[at..at + 4].try_into().unwrap())
}

fn set_word(profile: &mut [u8], at: usize, value: i32) {
    profile[at..at + 4].copy_from_slice(&value.to_le_bytes());
}

/// Where `player`'s record for controller `slot` starts.
fn record(player: usize, slot: usize) -> usize {
    player * STRIDE + RECORDS + slot * RECORD
}

/// A controller's USB vendor and product IDs, as the game keys its records.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Ids {
    pub vendor: u16,
    pub product: u16,
}

impl fmt::Display for Ids {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04x}:{:04x}", self.vendor, self.product)
    }
}

/// The wheel layer's virtual wheel.
pub const WHEEL: Ids = Ids {
    vendor: VIRTUAL.vid,
    product: VIRTUAL.pid,
};

/// The device a record is for; `None` for an empty or factory record.
fn ids_at(profile: &[u8], at: usize) -> Option<Ids> {
    let product = u16::try_from(word(profile, at + DEVICE)).ok()?;
    let vendor = u16::try_from(word(profile, at + DEVICE + 4)).ok()?;
    (vendor, product)
        .ne(&(0, 0))
        .then_some(Ids { vendor, product })
}

fn player_slot(profile: &[u8]) -> i32 {
    word(profile, PLAYER_SLOT)
}

/// True when the record at `at` is a pad's factory setup: digital pedals,
/// buttons 1 to 9 and no axis tuning.
fn is_factory_pad(profile: &[u8], at: usize) -> bool {
    profile[at + MODES + 1] == DIGITAL_PEDALS
        && (0..9).all(|i| word(profile, at + BUTTONS + 4 * i) == i as i32 + 1)
        && profile[at + PAD_TUNING..at + PAD_TUNING + 2 * TUNING_LEN]
            .iter()
            .all(|&b| b == 0)
}

/// True when player 1's controls were never set up: on the keyboard, with
/// the first controller's factory setup.
pub fn is_untouched(profile: &[u8]) -> bool {
    profile.len() == SIZE
        && player_slot(profile) == KEYBOARD
        && is_factory_pad(profile, record(0, FIRST_CONTROLLER))
}

/// Player 1's record for the virtual wheel, as the launcher seeds it.
fn wheel_record() -> Vec<u8> {
    let mut record = vec![0; RECORD];
    record[MODES] = WHEEL_FLAGS;
    record[MODES + 1] = ANALOGUE_PEDALS;
    for (i, code) in WHEEL_CONTROLS.iter().enumerate() {
        set_word(&mut record, CONTROLS + 4 * i, *code);
    }
    set_word(&mut record, DEVICE, i32::from(WHEEL.product));
    set_word(&mut record, DEVICE + 4, i32::from(WHEEL.vendor));
    for (action, tuning) in WHEEL_TUNINGS {
        for (i, value) in tuning.iter().enumerate() {
            set_word(&mut record, TUNINGS + action * TUNING_LEN + 4 * i, *value);
        }
    }
    set_word(&mut record, AUTOCENTRE_OFF, 1);
    record
}

/// The pad's setup in `slot`: analogue steering and analogue pedals on
/// Xidi's trigger axis for player 1, and the pad's IDs for every player.
fn seed_pad(profile: &mut [u8], slot: usize, pad: Ids) {
    let at = record(0, slot);
    profile[at + MODES + 1] = ANALOGUE_PEDALS;
    set_word(profile, at + BUTTONS, -1);
    set_word(profile, at + BUTTONS + 4, 1);
    set_word(profile, at + PEDAL_AXIS, XIDI_PEDAL_AXIS);
    set_word(profile, at + PEDAL_AXIS + 4, XIDI_PEDAL_AXIS);
    for (i, value) in TUNING_RECORD.iter().chain(&TUNING_RECORD).enumerate() {
        set_word(profile, at + PAD_TUNING + 4 * i, *value);
    }
    for player in 0..PLAYERS {
        let at = record(player, slot);
        profile[at + MODES] = ANALOGUE_STEERING;
        set_word(profile, at + DEVICE, i32::from(pad.product));
        set_word(profile, at + DEVICE + 4, i32::from(pad.vendor));
    }
}

/// The controllers the game will list at its start, in their slots.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Layout {
    /// The wheel layer shows the virtual wheel (first, in slot 2).
    pub wheel: bool,
    /// The Xidi pad and its IDs, behind the wheel when there is one.
    pub pad: Option<Ids>,
}

impl Layout {
    fn pad_slot(&self) -> usize {
        if self.wheel {
            PAD_BEHIND_WHEEL
        } else {
            FIRST_CONTROLLER
        }
    }
}

/// The launcher's copy of player 1's record for each device it lays out,
/// so a device that comes back to a slot gets the player's own setup.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Stash(BTreeMap<Ids, Vec<u8>>);

const STASH_MAGIC: &[u8; 8] = b"CMR4REC1";
const STASH_ENTRY: usize = 4 + RECORD;

impl Stash {
    /// The stash file's records; `None` when `bytes` is not one.
    pub fn parse(bytes: &[u8]) -> Option<Stash> {
        let entries = bytes.strip_prefix(STASH_MAGIC)?;
        if entries.len() % STASH_ENTRY != 0 {
            return None;
        }
        let records = entries.chunks(STASH_ENTRY).map(|entry| {
            let half = |at: usize| u16::from_le_bytes([entry[at], entry[at + 1]]);
            let ids = Ids {
                vendor: half(0),
                product: half(2),
            };
            (ids, entry[4..].to_vec())
        });
        Some(Stash(records.collect()))
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = STASH_MAGIC.to_vec();
        for (ids, record) in &self.0 {
            out.extend(ids.vendor.to_le_bytes());
            out.extend(ids.product.to_le_bytes());
            out.extend(record);
        }
        out
    }

    fn get(&self, ids: Ids) -> Option<&[u8]> {
        self.0.get(&ids).map(Vec::as_slice)
    }

    /// Takes in player 1's records of the slots the launcher lays out
    /// (the first controller and the pad behind the wheel) that belong to
    /// a device. Returns the devices whose copy changed.
    pub fn keep(&mut self, profile: &[u8]) -> Vec<Ids> {
        if profile.len() != SIZE {
            return Vec::new();
        }
        let mut kept = Vec::new();
        for slot in [FIRST_CONTROLLER, PAD_BEHIND_WHEEL] {
            let at = record(0, slot);
            let Some(ids) = ids_at(profile, at) else {
                continue;
            };
            let bytes = &profile[at..at + RECORD];
            if self.get(ids) != Some(bytes) {
                self.0.insert(ids, bytes.to_vec());
                kept.push(ids);
            }
        }
        kept
    }
}

/// What [`prepare`] changed.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Changes {
    /// Devices whose record came back from the stash.
    pub restored: Vec<Ids>,
    /// Devices given the launcher's setup.
    pub seeded: Vec<Ids>,
    /// The slot player 1 now drives with, when the launcher picked it.
    pub player: Option<usize>,
}

impl Changes {
    pub fn is_empty(&self) -> bool {
        self.restored.is_empty() && self.seeded.is_empty() && self.player.is_none()
    }
}

/// Puts `ids`' stashed record into player 1's `slot` unless the record
/// there is already that device's. Returns whether it did.
fn restore(profile: &mut [u8], slot: usize, ids: Ids, stash: &Stash) -> bool {
    let at = record(0, slot);
    if ids_at(profile, at) == Some(ids) {
        return false;
    }
    let Some(saved) = stash.get(ids) else {
        return false;
    };
    profile[at..at + RECORD].copy_from_slice(saved);
    true
}

/// Gets player 1's controller records ready for `layout`:
/// - the virtual wheel's record in slot 2: kept when it is there, else the
///   stashed one, else the launcher's seed (separate analogue pedals, force
///   feedback at full, autocentre off);
/// - the pad's in its slot: kept, or the stashed one; then, while it is
///   the pad's factory setup, analogue pedals, as for a profile never set
///   up (a pad behind the wheel gets them unless player 1 drives with it);
/// - player 1 drives with slot 2 when the profile was never set up, and
///   follows the pad back to slot 2 from behind the wheel.
///
/// A record replaced here would have been reset by the game anyway: its
/// IDs aren't the device's in that slot.
pub fn prepare(profile: &mut [u8], layout: Layout, stash: &Stash) -> Changes {
    let mut changes = Changes::default();
    if profile.len() != SIZE {
        return changes;
    }
    let untouched = is_untouched(profile);
    if layout.wheel {
        let at = record(0, FIRST_CONTROLLER);
        if restore(profile, FIRST_CONTROLLER, WHEEL, stash) {
            changes.restored.push(WHEEL);
        } else if ids_at(profile, at) != Some(WHEEL) {
            profile[at..at + RECORD].copy_from_slice(&wheel_record());
            changes.seeded.push(WHEEL);
        }
    }
    if let Some(pad) = layout.pad {
        let slot = layout.pad_slot();
        if restore(profile, slot, pad, stash) {
            changes.restored.push(pad);
        }
        let driven = player_slot(profile) == slot as i32;
        if is_factory_pad(profile, record(0, slot)) && (untouched || layout.wheel && !driven) {
            seed_pad(profile, slot, pad);
            changes.seeded.push(pad);
        }
    }
    let controller = layout.wheel || layout.pad.is_some();
    let pad_returns =
        !layout.wheel && layout.pad.is_some() && player_slot(profile) == PAD_BEHIND_WHEEL as i32;
    if untouched && controller || pad_returns {
        set_word(profile, PLAYER_SLOT, FIRST_CONTROLLER as i32);
        changes.player = Some(FIRST_CONTROLLER);
    }
    changes
}

/// The `launcher.log` lines for what [`prepare_in`] did.
pub fn describe(kept: &[Ids], changes: &Changes) -> Vec<String> {
    let list = |ids: &[Ids]| {
        let names: Vec<String> = ids.iter().map(Ids::to_string).collect();
        names.join(", ")
    };
    let mut lines = Vec::new();
    if !kept.is_empty() {
        lines.push(format!("kept the records of {}", list(kept)));
    }
    if changes.is_empty() {
        lines.push("profile already set up or not created yet".to_owned());
    }
    if !changes.restored.is_empty() {
        lines.push(format!("restored {}", list(&changes.restored)));
    }
    if !changes.seeded.is_empty() {
        lines.push(format!(
            "set up {} with analogue pedals",
            list(&changes.seeded)
        ));
    }
    if let Some(slot) = changes.player {
        lines.push(format!("player 1 drives with slot {slot}"));
    }
    lines
}

/// [`Stash::keep`] then [`prepare`] on the `OPTIONS` file at `options`,
/// with the stash file at `stash`; nothing when `OPTIONS` doesn't exist
/// yet (the game writes it during its first run). Each file is written
/// whole (a torn OPTIONS would lose every player's profile), and only when
/// it changes. Returns the stash's newly kept devices and the changes.
pub fn prepare_in(options: &Path, stash: &Path, layout: Layout) -> io::Result<(Vec<Ids>, Changes)> {
    let mut profile = match std::fs::read(options) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return Ok((Vec::new(), Changes::default()));
        }
        Err(error) => return Err(error),
    };
    // A missing or foreign stash file starts a new stash.
    let mut copies = std::fs::read(stash)
        .ok()
        .and_then(|bytes| Stash::parse(&bytes))
        .unwrap_or_default();
    let kept = copies.keep(&profile);
    if !kept.is_empty() {
        fsx::write_atomic(stash, copies.to_bytes())?;
    }
    let changes = prepare(&mut profile, layout, &copies);
    if !changes.is_empty() {
        fsx::write_atomic(options, &profile)?;
    }
    Ok((kept, changes))
}

#[cfg(test)]
mod tests {
    use super::*;

    const PAD: Ids = Ids {
        vendor: 0x2DC8,
        product: 0x310B,
    };
    const OTHER_PAD: Ids = Ids {
        vendor: 0x045E,
        product: 0x028E,
    };
    const PAD_ONLY: Layout = Layout {
        wheel: false,
        pad: Some(PAD),
    };
    const WHEEL_AND_PAD: Layout = Layout {
        wheel: true,
        pad: Some(PAD),
    };
    const WHEEL_ONLY: Layout = Layout {
        wheel: true,
        pad: None,
    };

    const SLOT_2: usize = 0x4AA4;
    const SLOT_3: usize = 0x6FC4;

    /// A profile as the game writes it for a new player: the first
    /// controller's factory setup, the other controller slots empty (all
    /// controls -1, no device), as in an OPTIONS the game wrote.
    fn factory() -> Vec<u8> {
        let mut profile = vec![0u8; SIZE];
        for player in 0..PLAYERS {
            for slot in FIRST_CONTROLLER..6 {
                let at = record(player, slot);
                profile[at + MODES + 1] = DIGITAL_PEDALS;
                for i in 0..30 {
                    set_word(&mut profile, at + CONTROLS + 4 * i, -1);
                }
            }
            let at = record(player, FIRST_CONTROLLER);
            for i in 0..9 {
                set_word(&mut profile, at + BUTTONS + 4 * i, i as i32 + 1);
            }
            set_word(&mut profile, at + PEDAL_AXIS, 12);
            set_word(&mut profile, at + PEDAL_AXIS + 4, 12);
            set_word(&mut profile, at + DEVICE, 654);
            set_word(&mut profile, at + DEVICE + 4, -1);
        }
        profile
    }

    fn slot_ids(profile: &[u8], slot: usize) -> Option<Ids> {
        ids_at(profile, record(0, slot))
    }

    fn slot_record(profile: &[u8], slot: usize) -> &[u8] {
        &profile[record(0, slot)..record(0, slot) + RECORD]
    }

    /// One launch: the stash takes the records in, then the profile is
    /// prepared from it.
    fn launch(profile: &mut [u8], stash: &mut Stash, layout: Layout) -> Changes {
        stash.keep(profile);
        prepare(profile, layout, stash)
    }

    #[test]
    fn the_layout_matches_the_seed_offsets() {
        assert_eq!(record(0, 2), SLOT_2);
        assert_eq!(record(0, 3), SLOT_3);
        assert_eq!(record(1, 2), SLOT_2 + STRIDE);
        assert_eq!(SLOT_2 + PAD_TUNING, 0x4BE0, "the pad's tunings");
        assert_eq!(SLOT_2 + CONTROLS + 22 * 4, 0x4B00, "accelerate");
        assert_eq!(SLOT_2 + DEVICE, 0x4B18);
        assert_eq!(RECORD, AUTOCENTRE_OFF + 4);
        // Buttons, two codes for the hat, then X, Y, Z and Rz.
        assert_eq!(STEER, VIRTUAL.buttons as i32 + 2);
        assert_eq!([STEER, COMBINED_PEDALS, THROTTLE, BRAKE], [34, 35, 36, 37]);
    }

    #[test]
    fn a_factory_profile_gets_the_pad_and_analogue_pedals() {
        let mut profile = factory();
        assert!(is_untouched(&profile));
        let changes = prepare(&mut profile, PAD_ONLY, &Stash::default());
        assert_eq!(
            changes,
            Changes {
                restored: vec![],
                seeded: vec![PAD],
                player: Some(2)
            }
        );
        // The values the game itself saved after the same choice by hand.
        assert_eq!(word(&profile, PLAYER_SLOT), 2);
        assert_eq!(profile[SLOT_2], 1);
        assert_eq!(profile[SLOT_2 + 1], 0);
        assert_eq!(word(&profile, SLOT_2 + BUTTONS), -1);
        assert_eq!(word(&profile, SLOT_2 + BUTTONS + 4), 1);
        assert_eq!(
            word(&profile, SLOT_2 + BUTTONS + 8),
            3,
            "handbrake stays on X"
        );
        assert_eq!(word(&profile, 0x4B00), 16);
        assert_eq!(word(&profile, 0x4B04), 16);
        assert_eq!(word(&profile, 0x4BE0), 32767);
        assert_eq!(word(&profile, 0x4BE0 + 0x20), 32767);
        assert_eq!(word(&profile, 0x4BE0 + 0x38), 100);
        for player in 0..PLAYERS {
            let at = record(player, 2);
            assert_eq!(word(&profile, at + DEVICE), 12555);
            assert_eq!(word(&profile, at + DEVICE + 4), 11720);
            assert_eq!(profile[at], 1);
        }
        // Players 2 to 4 keep their own pedal setup.
        assert_eq!(profile[record(1, 2) + 1], DIGITAL_PEDALS);
    }

    #[test]
    fn the_pad_setup_is_todays_byte_for_byte() {
        // What launcher 0.5.2 wrote for a factory profile, field by field:
        // nothing else in the file changes.
        let mut expected = factory();
        set_word(&mut expected, PLAYER_SLOT, 2);
        expected[SLOT_2 + 1] = 0;
        set_word(&mut expected, 0x4ADC, -1);
        set_word(&mut expected, 0x4AE0, 1);
        set_word(&mut expected, 0x4B00, 16);
        set_word(&mut expected, 0x4B04, 16);
        for (i, value) in TUNING_RECORD.iter().chain(&TUNING_RECORD).enumerate() {
            set_word(&mut expected, 0x4BE0 + 4 * i, *value);
        }
        for player in 0..PLAYERS {
            let at = 0x4AA4 + player * STRIDE;
            expected[at] = 1;
            set_word(&mut expected, at + 0x74, 0x310B);
            set_word(&mut expected, at + 0x78, 0x2DC8);
        }
        let mut profile = factory();
        prepare(&mut profile, PAD_ONLY, &Stash::default());
        assert!(profile == expected);
    }

    #[test]
    fn a_configured_profile_is_left_alone_without_the_wheel() {
        let mut profile = factory();
        prepare(&mut profile, PAD_ONLY, &Stash::default());
        let once = profile.clone();
        assert!(!is_untouched(&profile));
        assert!(prepare(&mut profile, PAD_ONLY, &Stash::default()).is_empty());
        assert!(
            prepare(
                &mut profile,
                Layout {
                    wheel: false,
                    pad: Some(OTHER_PAD)
                },
                &Stash::default()
            )
            .is_empty()
        );
        assert!(profile == once);

        // A chosen controller, hand-set analogue pedals, rebound buttons or
        // tuned axes all count as set up by hand.
        for change in [
            (PLAYER_SLOT, 2),
            (SLOT_2 + 1, ANALOGUE_PEDALS),
            (SLOT_2 + BUTTONS + 8, 7),
            (0x4BE0 + 2, 1),
        ] {
            let mut profile = factory();
            profile[change.0] = change.1;
            let before = profile.clone();
            assert!(
                prepare(&mut profile, PAD_ONLY, &Stash::default()).is_empty(),
                "{change:?}"
            );
            assert!(profile == before, "{change:?}");
        }
    }

    #[test]
    fn nothing_to_lay_out_changes_nothing() {
        let mut profile = factory();
        assert!(prepare(&mut profile, Layout::default(), &Stash::default()).is_empty());
        assert!(profile == factory());
    }

    #[test]
    fn other_files_are_never_changed() {
        for size in [0, SIZE - 1, SIZE + 1] {
            let mut profile = factory();
            profile.resize(size, 0);
            let before = profile.clone();
            assert!(prepare(&mut profile, WHEEL_AND_PAD, &Stash::default()).is_empty());
            assert!(Stash::default().keep(&profile).is_empty());
            assert!(profile == before);
        }
    }

    #[test]
    fn a_factory_profile_gets_the_wheel_from_the_seed() {
        let mut profile = factory();
        let changes = prepare(&mut profile, WHEEL_ONLY, &Stash::default());
        assert_eq!(changes.seeded, [WHEEL]);
        assert_eq!(changes.player, Some(2));
        assert_eq!(word(&profile, PLAYER_SLOT), 2);
        // The seed file's values, at their file offsets.
        assert_eq!(profile[0x4AA4], 0x13, "attached, force feedback at full");
        assert_eq!(profile[0x4AA5], 0, "analogue steering and pedals");
        assert_eq!(word(&profile, 0x4AA4 + 0x24), 34, "steering on X");
        assert_eq!(word(&profile, 0x4B00), 36, "accelerate on Z");
        assert_eq!(word(&profile, 0x4B04), 37, "brake on Rz");
        assert_eq!(word(&profile, 0x4B18), 0x5245);
        assert_eq!(word(&profile, 0x4B1C), 0x1209);
        assert_eq!(
            word(&profile, 0x4C00),
            32767,
            "steering centred, no deadzone"
        );
        assert_eq!(word(&profile, 0x4C00 + 4), 0);
        assert_eq!(word(&profile, 0x4C40 + 16), 65535, "accelerate's range");
        assert_eq!(word(&profile, 0x4C60 + 16), 65535, "brake's range");
        assert_eq!(word(&profile, 0x6FC0), 1, "autocentre off");
        assert!(
            profile[0x4BE0..0x4C00].iter().all(|&b| b == 0),
            "the pad's tunings are gone"
        );
        let controls: Vec<i32> = (0..28).map(|i| word(&profile, 0x4AA8 + 4 * i)).collect();
        assert_eq!(controls, WHEEL_CONTROLS);
        // Only player 1's wheel record and slot change.
        let mut rest = profile.clone();
        rest[SLOT_2..SLOT_2 + RECORD].copy_from_slice(slot_record(&factory(), 2));
        set_word(&mut rest, PLAYER_SLOT, 0);
        assert!(rest == factory());
    }

    #[test]
    fn the_pad_behind_the_wheel_gets_analogue_pedals_once_the_game_set_it_up() {
        let mut profile = factory();
        let mut stash = Stash::default();
        let changes = launch(&mut profile, &mut stash, WHEEL_AND_PAD);
        assert_eq!(
            changes.seeded,
            [WHEEL],
            "slot 3 is empty: the game sets it up"
        );
        // The game's own defaults for the pad in slot 3 (as in slot 2 of a
        // new profile), written when it saves.
        let pad_defaults = slot_record(&factory(), 2).to_vec();
        let at = record(0, 3);
        profile[at..at + RECORD].copy_from_slice(&pad_defaults);
        set_word(&mut profile, at + DEVICE, 0x310B);
        set_word(&mut profile, at + DEVICE + 4, 0x2DC8);
        let changes = launch(&mut profile, &mut stash, WHEEL_AND_PAD);
        assert_eq!(changes.seeded, [PAD]);
        assert_eq!(changes.player, None);
        assert_eq!(word(&profile, PLAYER_SLOT), 2, "still the wheel");
        assert_eq!(word(&profile, SLOT_3 + PEDAL_AXIS), 16);
        assert_eq!(profile[SLOT_3 + 1], ANALOGUE_PEDALS);
        assert_eq!(slot_ids(&profile, 2), Some(WHEEL));
    }

    #[test]
    fn the_players_wheel_and_pad_tweaks_survive_switching() {
        let mut profile = factory();
        let mut stash = Stash::default();
        // A pad run, then the player tunes the pad in the game.
        launch(&mut profile, &mut stash, PAD_ONLY);
        set_word(&mut profile, SLOT_2 + PAD_TUNING + 4, -3000);
        let tuned_pad = slot_record(&profile, 2).to_vec();
        // A wheel run: the wheel takes slot 2, the pad's record moves to 3.
        let changes = launch(&mut profile, &mut stash, WHEEL_AND_PAD);
        assert_eq!(changes.seeded, [WHEEL]);
        assert_eq!(changes.restored, [PAD]);
        assert_eq!(slot_record(&profile, 3), tuned_pad.as_slice());
        assert_eq!(word(&profile, PLAYER_SLOT), 2, "the wheel");
        // The player tunes the wheel: deadzone and swapped pedal codes.
        set_word(&mut profile, SLOT_2 + TUNINGS + 7 * TUNING_LEN + 4, -500);
        let tuned_wheel = slot_record(&profile, 2).to_vec();
        // A run without the wheel: the pad's record comes back to slot 2.
        let changes = launch(&mut profile, &mut stash, PAD_ONLY);
        assert_eq!(changes.restored, [PAD]);
        assert_eq!(slot_record(&profile, 2), tuned_pad.as_slice());
        // And the wheel's comes back on the next wheel run.
        let changes = launch(&mut profile, &mut stash, WHEEL_AND_PAD);
        assert_eq!(changes.restored, [WHEEL]);
        assert!(changes.seeded.is_empty());
        assert_eq!(slot_record(&profile, 2), tuned_wheel.as_slice());
        // Nothing to do when nothing moved.
        assert!(launch(&mut profile, &mut stash, WHEEL_AND_PAD).is_empty());
    }

    #[test]
    fn the_player_follows_the_pad_back_from_behind_the_wheel() {
        let mut profile = factory();
        let mut stash = Stash::default();
        launch(&mut profile, &mut stash, PAD_ONLY);
        launch(&mut profile, &mut stash, WHEEL_AND_PAD);
        // The player picks the pad in slot 3 in the game.
        set_word(&mut profile, PLAYER_SLOT, 3);
        assert_eq!(
            launch(&mut profile, &mut stash, WHEEL_AND_PAD).player,
            None,
            "kept while the wheel is there"
        );
        assert_eq!(launch(&mut profile, &mut stash, PAD_ONLY).player, Some(2));
        assert_eq!(word(&profile, PLAYER_SLOT), 2);
        // The keyboard, picked by hand, stays.
        set_word(&mut profile, PLAYER_SLOT, 0);
        assert_eq!(launch(&mut profile, &mut stash, WHEEL_AND_PAD).player, None);
        assert_eq!(launch(&mut profile, &mut stash, PAD_ONLY).player, None);
        assert_eq!(word(&profile, PLAYER_SLOT), 0);
    }

    #[test]
    fn a_pad_set_up_by_hand_behind_the_wheel_stays_digital() {
        let mut profile = factory();
        let mut stash = Stash::default();
        launch(&mut profile, &mut stash, WHEEL_AND_PAD);
        // The game's pad defaults in slot 3, and the player drives with it.
        let at = record(0, 3);
        profile[at..at + RECORD].copy_from_slice(slot_record(&factory(), 2));
        set_word(&mut profile, at + DEVICE, 0x310B);
        set_word(&mut profile, at + DEVICE + 4, 0x2DC8);
        set_word(&mut profile, PLAYER_SLOT, 3);
        let changes = launch(&mut profile, &mut stash, WHEEL_AND_PAD);
        assert!(changes.seeded.is_empty(), "{changes:?}");
        assert_eq!(profile[SLOT_3 + 1], DIGITAL_PEDALS);
    }

    #[test]
    fn the_stash_keeps_devices_only_and_survives_its_file() {
        let mut profile = factory();
        let mut stash = Stash::default();
        assert!(stash.keep(&profile).is_empty(), "factory IDs are no device");
        prepare(&mut profile, WHEEL_AND_PAD, &stash);
        assert_eq!(stash.keep(&profile), [WHEEL]);
        assert!(stash.keep(&profile).is_empty(), "unchanged");
        let bytes = stash.to_bytes();
        assert_eq!(bytes.len(), 8 + STASH_ENTRY);
        assert_eq!(Stash::parse(&bytes), Some(stash));
        assert_eq!(Stash::parse(b"CMR4REC1"), Some(Stash::default()));
        for foreign in [&b""[..], b"CMR4REC2", &bytes[..bytes.len() - 1]] {
            assert_eq!(Stash::parse(foreign), None);
        }
    }

    #[test]
    fn the_log_says_what_changed() {
        let changes = Changes {
            restored: vec![PAD],
            seeded: vec![WHEEL],
            player: Some(2),
        };
        assert_eq!(
            describe(&[PAD, OTHER_PAD], &changes),
            [
                "kept the records of 2dc8:310b, 045e:028e",
                "restored 2dc8:310b",
                "set up 1209:5245 with analogue pedals",
                "player 1 drives with slot 2"
            ]
        );
        assert_eq!(
            describe(&[], &Changes::default()),
            ["profile already set up or not created yet"]
        );
    }

    #[test]
    fn the_files_are_written_only_when_they_change() {
        let dir = std::env::temp_dir().join(format!("cmr4-profile-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let options = dir.join("OPTIONS");
        let stash = dir.join(STASH_FILE);
        let (kept, changes) = prepare_in(&options, &stash, WHEEL_AND_PAD).unwrap();
        assert!(kept.is_empty() && changes.is_empty(), "missing file");
        assert!(!stash.exists());

        std::fs::write(&options, factory()).unwrap();
        let (kept, changes) = prepare_in(&options, &stash, PAD_ONLY).unwrap();
        assert!(kept.is_empty());
        assert_eq!(changes.seeded, [PAD]);
        assert_eq!(word(&std::fs::read(&options).unwrap(), 0x4B18), 12555);

        let (kept, changes) = prepare_in(&options, &stash, WHEEL_AND_PAD).unwrap();
        assert_eq!(kept, [PAD]);
        assert_eq!((changes.seeded, changes.restored), (vec![WHEEL], vec![PAD]));
        let written = std::fs::read(&options).unwrap();
        assert_eq!(word(&written, 0x4B18), 0x5245);
        assert_eq!(word(&written, 0x6FC4 + 0x74), 12555);

        // A broken stash file starts again.
        std::fs::write(&stash, b"junk").unwrap();
        let (kept, changes) = prepare_in(&options, &stash, WHEEL_AND_PAD).unwrap();
        assert_eq!(kept, [WHEEL, PAD]);
        assert!(changes.is_empty());
        assert!(Stash::parse(&std::fs::read(&stash).unwrap()).is_some());
        std::fs::remove_dir_all(dir).unwrap();
    }
}
