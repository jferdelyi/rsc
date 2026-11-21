//! The window's tests, a file a theme, on ports a test plays. What they share: the window on
//! a settings file of their own, its clock, the ports, and waiting for the thread of a port.

mod by_hand;
mod console_and_curves;
mod files;
mod link;
mod profiles;
mod terminal;
mod window;

use std::fs;
use std::io;
use std::time::Duration;

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;

use super::*;
use super::files::{Saved, file_stem, save_filters};
use super::pill::short_name;
use super::settings_window::Section;
use super::top::tab_name;
use crate::link::fake::FakePorts;
use crate::plot::XAxis;
use crate::ports::UsbId;
use crate::settings::{self, LineEnding, Profile};

/// No test waits longer than this for the thread of a port.
const DEADLINE: Duration = Duration::from_secs(5);

fn clock() -> Time {
    crate::console::at(0, 0)
}

fn harness(fake: &FakePorts, settings: Settings, test: &str) -> Harness<'static, App> {
    let path = test_settings(test);
    settings.save(&path).expect("settings written");
    harness_at(fake, path)
}

/// The window on the settings file `path`, as it is.
fn harness_at(fake: &FakePorts, path: PathBuf) -> Harness<'static, App> {
    let fake = fake.clone();
    Harness::builder().with_size(egui::vec2(1100.0, 720.0)).build_eframe(move |cc| {
        let fake = fake.clone();
        App::new(&cc.egui_ctx, Some(path.clone()), Machine::of(fake.clone()), clock)
    })
}

/// Frames until `done`, or the deadline.
fn until(harness: &mut Harness<'static, App>, what: &str, done: impl Fn(&App) -> bool) {
    let start = Instant::now();
    while !done(harness.state()) {
        assert!(start.elapsed() < DEADLINE, "{what}: not within {DEADLINE:?}");
        harness.step();
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn console_text(app: &App, i: usize) -> String {
    app.sessions[i].console.text(Shown::default())
}

fn on(port: &str) -> Settings {
    let mut settings = Settings::default();
    settings.tabs[0].port = port.into();
    settings
}

fn board() -> UsbId {
    UsbId { vid: 0x10c4, pid: 0xea60, serial: Some("A1".into()) }
}

fn usb_port(name: &str) -> PortInfo {
    PortInfo { name: name.into(), description: "CP2102N".into(), usb: Some(board()) }
}

fn is_disabled(harness: &Harness<'static, App>, label: &str) -> bool {
    use egui_kittest::kittest::NodeT;
    harness.get_by_label(label).accesskit_node().is_disabled()
}

fn screen(app: &App) -> String {
    app.sessions[0].terminal.as_ref().map(|terminal| terminal.screen().contents()).unwrap_or_default()
}
