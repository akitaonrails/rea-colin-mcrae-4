//! Portable launcher logic: CMR4's game knowledge on top of the shared
//! `rea_launcher` crate. Win32 calls go through the binary's `win32` module
//! to `rea_launcher_win32`.

pub mod components;
pub mod config;
pub mod display;
pub mod form;
pub mod patch;
pub mod profile;
pub mod registry;
pub mod runtime;

pub const GAME_EXE: &str = "cmr4.exe";
