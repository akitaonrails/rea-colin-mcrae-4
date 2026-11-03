//! Portable launcher logic: CMR4's game knowledge on top of the shared
//! `rea_launcher` crate. Win32 calls go through the binary's `win32` module
//! to `rea_launcher_win32`.

pub mod audience;
pub mod components;
pub mod config;
pub mod display;
pub mod edition;
pub mod form;
pub mod instance;
pub mod plan;
pub mod profile;
pub mod registry;
pub mod runtime;
pub mod window;

pub const GAME_EXE: &str = "cmr4.exe";

#[cfg(test)]
mod e2e;
