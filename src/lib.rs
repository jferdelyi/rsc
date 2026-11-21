//! RSC (Rust Serial Console): a desktop serial terminal. Pick a port, set the link, read what the
//! device sends as text or as bytes, send it lines or files, copy, save or journal the console, name
//! each device with what it changes of the general profile, and hold several ports at once.
//!
//! The window is in `ui`; everything else knows nothing of it, and is tested without it.

// No unsafe code, ever: what the device sends and what the files hold go through safe Rust only.
#![forbid(unsafe_code)]

// French or English: `t!` and `tr!`, for the modules below.
#[macro_use]
pub mod i18n;

pub mod console;
pub mod journal;
pub mod licences;
pub mod link;
pub mod plot;
pub mod ports;
pub mod send;
pub mod settings;
pub mod terminal;
pub mod ui;
