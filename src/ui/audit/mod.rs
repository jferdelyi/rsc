//! Accessibility measures of the window against WCAG 2.2 AA (18/11/2026). Ignored tests, run by
//! hand on a machine with a GPU, that write their tables and captures into `build/audit/`, or into
//! the folder `RSC_AUDIT` names:
//!
//! ```text
//! cargo test --lib audit -- --ignored --nocapture --test-threads 1
//! ```
//!
//! They measure and do not judge: each figure is written beside its threshold, the report reads
//! them.

mod colour_and_drag;
mod contrast;
mod keyboard;
mod screen_readers;
mod tree;
mod zoom;

use std::fmt::Write as _;
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use eframe::egui::{self, Color32, Key, Modifiers, Pos2, Rect, Vec2};
use egui::accesskit::{Action, Live, NodeId, Role};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use egui_plot::PlotMemory;

use super::settings_window::{Section, Window};
use super::theme::{self, Palette, Tone};
use super::{App, Machine, State};
use crate::console::{Color, Kind, Time};
use crate::link::fake::FakePorts;
use crate::ports::{PortInfo, UsbId};
use crate::settings::{Settings, Theme};

/// The window's size at its start (main.rs).
const WINDOW: Vec2 = egui::vec2(1100.0, 720.0);

/// The window's least size: at 200 %, twice as many pixels.
const LEAST: Vec2 = super::MIN_SIZE;

/// The most presses of Tab in a walk.
const MOST_TABS: usize = 120;

/// Where the tables and the captures go.
fn folder() -> PathBuf {
    let folder = std::env::var_os("RSC_AUDIT").map_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("build").join("audit"), PathBuf::from);
    fs::create_dir_all(&folder).expect("the audit's folder");
    folder
}

/// A text cell of a table, quoted.
fn cell(text: &str) -> String {
    format!("\"{}\"", text.replace('"', "\"\""))
}

/// The relative luminance of a colour, as WCAG 2.2 defines it.
fn luminance(color: Color32) -> f64 {
    let linear = |channel: u8| {
        let c = f64::from(channel) / 255.0;
        if c <= 0.040_45 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
    };
    0.2126 * linear(color.r()) + 0.7152 * linear(color.g()) + 0.0722 * linear(color.b())
}

/// The contrast ratio of two colours, from 1 to 21.
fn ratio(a: Color32, b: Color32) -> f64 {
    let (a, b) = (luminance(a), luminance(b));
    (a.max(b) + 0.05) / (a.min(b) + 0.05)
}

/// The screens the audit looks at.
#[derive(Clone, Copy)]
enum Screen {
    /// The line console: two tabs, the first connected, lines in colour received.
    Line,
    /// The terminal mode, on what a menu program wrote.
    Terminal,
    /// The curves under the console.
    Curves,
    /// The curves, their menu "Above or below" open.
    CurvesMenu,
    /// The menu of the pill of a named device.
    Pill,
    /// The settings, at a section, on the general profile.
    Settings(Section),
}

impl Screen {
    fn all() -> Vec<Screen> {
        let mut all = vec![Screen::Line, Screen::Terminal, Screen::Curves, Screen::CurvesMenu, Screen::Pill];
        all.extend(Section::ALL.map(Screen::Settings));
        all
    }

    fn name(self) -> String {
        match self {
            Screen::Line => "fenetre-ligne".into(),
            Screen::Terminal => "fenetre-terminal".into(),
            Screen::Curves => "fenetre-courbes".into(),
            Screen::CurvesMenu => "courbes-menu-haut-ou-bas".into(),
            Screen::Pill => "pastille-profil".into(),
            Screen::Settings(section) => format!("reglages-{}", Section::ALL.iter().position(|known| *known == section).map_or(0, |n| n + 1)),
        }
    }
}

fn theme_name(theme: Theme) -> &'static str {
    match theme {
        Theme::Light => "clair",
        Theme::Dark => "sombre",
        Theme::System => "systeme",
    }
}

fn still_clock() -> Time {
    crate::console::at(0, 0)
}

/// A clock that runs, 25 ms at each look: the curves spread over time.
fn running_clock() -> Time {
    static MILLIS: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(0);
    crate::console::at(0, 0) + chrono::TimeDelta::milliseconds(MILLIS.fetch_add(25, std::sync::atomic::Ordering::Relaxed))
}

fn board() -> UsbId {
    UsbId { vid: 0x10c4, pid: 0xea60, serial: Some("A1".into()) }
}

fn ftdi() -> UsbId {
    UsbId { vid: 0x0403, pid: 0x6001, serial: Some("FT77".into()) }
}

/// Frames until `done`, or 5 s.
fn until(harness: &mut Harness<'static, App>, what: &str, done: impl Fn(&App) -> bool) {
    let start = Instant::now();
    while !done(harness.state()) {
        assert!(start.elapsed() < Duration::from_secs(5), "{what}: not within 5 s");
        harness.step();
        std::thread::sleep(Duration::from_millis(10));
    }
}

/// The window on `screen`, in `theme`, `size` points at `scale` pixels a point, drawn on the GPU
/// when `drawn`; the pointer out of it. The fake ports come with it.
fn open(screen: Screen, theme: Theme, size: Vec2, scale: f32, drawn: bool) -> (Harness<'static, App>, FakePorts) {
    let fake = FakePorts::default();
    fake.0.lock().unwrap().listed = vec![
        PortInfo { name: "COM23".into(), description: "CP2102N USB to UART Bridge Controller".into(), usb: Some(board()) },
        PortInfo { name: "COM4".into(), description: "USB Serial Device".into(), usb: Some(ftdi()) },
    ];
    let mut settings = Settings { theme, ..Settings::default() };
    settings.tabs[0].port = "COM23".into();
    settings.general.timestamps = true;
    settings.name_device(0, "Carte Nano n°3", board()).unwrap();
    settings.add_profile("Carte Nano n°3", None).unwrap();
    settings.choose_profile(0, Some("Carte Nano n°3"));
    let second = settings.add_tab();
    settings.choose_port(second, "COM4", Some(&ftdi()));
    settings.active = 0;
    match screen {
        Screen::Terminal => settings.edit_tab_profile(0, |profile| profile.terminal = true),
        Screen::Curves | Screen::CurvesMenu => settings.edit_tab_profile(0, |profile| profile.plot = true),
        _ => {}
    }
    let clock = if matches!(screen, Screen::Curves | Screen::CurvesMenu) { running_clock } else { still_clock };
    // Relative to the package, where the tests run: the settings' window shows this path, and a
    // capture must not show a folder of the machine (the temporary folder names its user).
    // One folder a test: tests run side by side.
    let test = std::thread::current().name().and_then(|name| name.rsplit("::").next()).unwrap_or("audit").to_owned();
    let folder = PathBuf::from("build").join("audit").join(test).join(format!("{}-{}-{}", screen.name(), theme_name(theme), (scale * 100.0) as u32));
    let _ = fs::remove_dir_all(&folder);
    fs::create_dir_all(&folder).expect("a folder for the settings");
    let path = folder.join("settings.json");
    settings.save(&path).expect("settings written");
    let link = fake.clone();
    let builder = Harness::builder().with_size(size).with_pixels_per_point(scale);
    let builder = if drawn { builder.wgpu() } else { builder };
    let mut harness = builder.build_eframe(move |cc| App::new(&cc.egui_ctx, Some(path.clone()), Machine::of(link.clone()), clock));
    harness.get_by_label("Connecter").click();
    until(&mut harness, "connected", |app| app.sessions[0].is_open());
    match screen {
        Screen::Terminal => {
            let recorded = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests").join("terminal").join("menu.out");
            fake.device_sends(&fs::read(recorded).unwrap());
            until(&mut harness, "drawn", |app| app.sessions[0].terminal.as_ref().is_some_and(|terminal| !terminal.screen().contents().trim().is_empty()));
        }
        Screen::Curves | Screen::CurvesMenu => {
            for n in 0..60_usize {
                let x = n as f64 / 8.0;
                let line = format!("temp={:.2} hum={:.2} wind={:.2}\r\n", 21.0 + x.sin(), 40.0 + 3.0 * (x / 2.0).cos(), 12.0 + 4.0 * (x / 5.0).sin());
                fake.device_sends(line.as_bytes());
                until(&mut harness, "measure", |app| app.sessions[0].plot.curves().first().is_some_and(|curve| curve.points().len() == n + 1));
            }
        }
        _ => {
            fake.device_sends(b"\x1b[0;32mI (312) boot: ESP-IDF v5.3 2nd stage bootloader\x1b[0m\r\n");
            fake.device_sends(b"\x1b[0;33mW (847) wifi: aucun point d'acces enregistre\x1b[0m\r\n");
            fake.device_sends("prêt, tapez help\r\n".as_bytes());
            fake.device_sends(b"\x1b[0;31mE (22410) i2c: delai depasse sur 0x44\x1b[0m\r\n");
            until(&mut harness, "lines", |app| app.sessions[0].console.text(super::Shown::default()).contains("0x44"));
        }
    }
    harness.run();
    // The curves' screens on a window too low for the console and the curves: the curves alone, as
    // a person would choose them.
    if matches!(screen, Screen::Curves | Screen::CurvesMenu) && !harness.state().sessions[0].room_for_both {
        harness.state_mut().sessions[0].display = super::Display::Curves;
        harness.run();
    }
    match screen {
        Screen::Settings(section) => harness.state_mut().window = Some(Window::at(section, None)),
        Screen::Pill => harness.get_by(|node| node.label().is_some_and(|label| label.starts_with("Profil Carte"))).click(),
        Screen::CurvesMenu => harness.get_by_label("Haut ou bas").click(),
        _ => {}
    }
    harness.event(egui::Event::PointerGone);
    harness.run();
    (harness, fake)
}

/// What the accessibility tree says of an element of the window.
struct Element {
    id: NodeId,
    role: Role,
    name: String,
    value: String,
    placeholder: String,
    /// In points.
    rect: Rect,
    click: bool,
    focus: bool,
    focused: bool,
    disabled: bool,
}

impl Element {
    /// Something to click or to reach with the keyboard.
    fn target(&self) -> bool {
        self.click || self.focus
    }

    fn small(&self) -> bool {
        self.rect.width() < 24.0 || self.rect.height() < 24.0
    }

    /// How a report calls it: its name, else its value, else its role and place.
    fn called(&self) -> String {
        if !self.name.is_empty() {
            self.name.clone()
        } else if !self.value.is_empty() {
            self.value.clone()
        } else {
            format!("{:?} sans nom en {:.0}, {:.0}", self.role, self.rect.min.x, self.rect.min.y)
        }
    }
}

/// The elements of the window as AccessKit gives them to a screen reader.
fn elements(harness: &Harness<'_, App>) -> Vec<Element> {
    let scale = harness.ctx.pixels_per_point();
    harness
        .root()
        .children_recursive()
        .filter_map(|node| {
            let node = node.accesskit_node();
            // What is hidden from screen readers is not read: a decorative icon, for instance.
            if node.is_hidden() {
                return None;
            }
            let bounds = node.bounding_box()?;
            let rect = Rect::from_min_max(Pos2::new(bounds.x0 as f32 / scale, bounds.y0 as f32 / scale), Pos2::new(bounds.x1 as f32 / scale, bounds.y1 as f32 / scale));
            Some(Element {
                id: node.locate().0,
                role: node.role(),
                name: node.label().unwrap_or_default(),
                value: node.value().unwrap_or_default(),
                placeholder: node.placeholder().unwrap_or_default().to_owned(),
                rect,
                click: node.data().supports_action(Action::Click),
                focus: node.data().supports_action(Action::Focus),
                focused: node.is_focused(),
                disabled: node.is_disabled(),
            })
        })
        .collect()
}

/// The element that has the keyboard's focus.
fn focused(harness: &Harness<'_, App>) -> Option<Element> {
    elements(harness).into_iter().find(|element| element.focused)
}

/// Nothing focused.
fn unfocus(harness: &mut Harness<'static, App>) {
    if let Some(id) = harness.ctx.memory(|memory| memory.focused()) {
        harness.ctx.memory_mut(|memory| memory.surrender_focus(id));
    }
    harness.run();
}

/// The label whose text begins with `start`.
fn label<'a>(all: &'a [Element], start: &str) -> &'a Element {
    all.iter().find(|element| element.role == Role::Label && element.value.starts_with(start)).unwrap_or_else(|| panic!("a label « {start}… »"))
}

/// A label shows its whole text, in the window's body font: on one row as wide as the text, or
/// on as many rows as it needs, not cut short by « … » (WCAG 1.4.4).
fn whole(harness: &Harness<'_, App>, element: &Element) -> bool {
    let font = egui::FontId::proportional(14.0);
    let (width, row) = harness.ctx.fonts_mut(|fonts| (fonts.layout_no_wrap(element.value.clone(), font.clone(), Color32::WHITE).size().x, fonts.row_height(&font)));
    let rows = (width / element.rect.width().max(1.0)).ceil();
    element.rect.width() + 1.0 >= width || element.rect.height() + 1.0 >= rows * row
}

/// The state's label stands on the row of the tabs.
fn state_on_tabs_row(all: &[Element]) -> bool {
    let state = label(all, "Connecté à");
    let tab = all.iter().find(|element| element.role == Role::Button && element.name == "COM4").expect("the tab « COM4 »");
    (state.rect.center().y - tab.rect.center().y).abs() < 6.0
}

/// A choice of the display, « Console », « Courbes » or « Les deux », clicked.
fn display(harness: &mut Harness<'static, App>, choice: &str) {
    harness.get_by(|node| node.role() == Role::RadioButton && node.label().as_deref() == Some(choice)).click();
    harness.run();
}

/// The texts painted in the last frame: where each stands, what it says, its colour.
fn texts(harness: &Harness<'_, App>) -> Vec<(Rect, String, Color32)> {
    fn look(shape: &egui::Shape, found: &mut Vec<(Rect, String, Color32)>) {
        match shape {
            egui::Shape::Vec(shapes) => shapes.iter().for_each(|shape| look(shape, found)),
            egui::Shape::Text(text) => found.push((text.galley.rect.translate(text.pos.to_vec2()), text.galley.text().to_owned(), text.fallback_color)),
            _ => {}
        }
    }
    let mut found = Vec::new();
    for clipped in &harness.output().shapes {
        look(&clipped.shape, &mut found);
    }
    found
}

/// The texts painted in the last frame: where each stands, the rectangle it is cut to, what it says.
fn clipped_texts(harness: &Harness<'_, App>) -> Vec<(Rect, Rect, String)> {
    fn look(shape: &egui::Shape, clip: Rect, found: &mut Vec<(Rect, Rect, String)>) {
        match shape {
            egui::Shape::Vec(shapes) => shapes.iter().for_each(|shape| look(shape, clip, found)),
            egui::Shape::Text(text) => found.push((text.galley.rect.translate(text.pos.to_vec2()), clip, text.galley.text().to_owned())),
            _ => {}
        }
    }
    let mut found = Vec::new();
    for clipped in &harness.output().shapes {
        look(&clipped.shape, clipped.clip_rect, &mut found);
    }
    found
}
