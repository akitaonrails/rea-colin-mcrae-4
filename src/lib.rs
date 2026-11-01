//! Portable launcher logic. Win32 calls are isolated in the binary's `win32` module.

pub mod components;
pub mod config;
pub mod form;
pub mod patch;
pub mod pe;
pub mod registry;
pub mod runtime;

pub const GAME_EXE: &str = "cmr4.exe";
