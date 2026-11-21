//! The settings' tests, a file a theme: a profile, the file, the files of earlier versions, the
//! library of profiles, tabs and named devices. What they share: a board, and settings with a named
//! device that has a profile of its own.

mod earlier_versions;
mod files;
mod library;
mod profiles;
mod tabs;

use std::fs;
use std::path::PathBuf;

use super::*;
use crate::plot::XAxis;
use crate::ports::UsbId;

fn board(serial: &str) -> UsbId {
    UsbId { vid: 0x10c4, pid: 0xea60, serial: Some(serial.into()) }
}

/// Settings with a tab on COM23, its device named "Carte" and given the profile "Carte", a copy of
/// the general one.
fn with_device() -> Settings {
    let mut settings = Settings::default();
    settings.tabs[0].port = "COM23".into();
    settings.name_device(0, "Carte", board("A1")).unwrap();
    settings.add_profile("Carte", None).unwrap();
    settings.choose_profile(0, Some("Carte"));
    settings
}
