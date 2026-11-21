//! The window: a tab per port, each with its console, its send line and its actions, all open at
//! once; a settings window. Every port is owned by a thread of its own (`link`): the window never
//! waits for a device.

mod bottom;
mod console_view;
mod files;
mod icons;
mod middle;
mod pill;
mod plot_view;
mod session;
mod settings_window;
mod terminal_view;
mod theme;
mod top;
mod widgets;
#[cfg(test)]
mod audit;

use std::path::PathBuf;
use std::time::Instant;

use eframe::egui::{self, CornerRadius, Key, Margin, Modifiers, Stroke};

use console_view::ConsoleView;
use crate::console::{Console, Shown, Time};
use crate::i18n;
use crate::journal::Journal;
use crate::link::{Event, Link, PortWatch, Ports};
use crate::plot::Plot;
use crate::ports::PortInfo;
use crate::send::History;
use crate::settings::{Language, LinkSettings, Loaded, Settings};
use crate::terminal::Terminal;
use middle::{PLOT_SHARE, shortcuts_height, shortcuts_text};
use session::RETRY_EVERY;
use terminal_view::TerminalView;
use theme::{CONTROL_HEIGHT, Palette, Tone};

/// The product's name, the window's title.
pub const TITLE: &str = "RSC";
/// Where eframe keeps the window's size, and where the settings go.
pub const APP_ID: &str = "rsc";
/// The window's least size, in points (WCAG 1.4.4): it holds whole, with its title bar and the
/// taskbar, on a 1366 × 768 screen at 150 % (909 × 433 points) and on a 1920 × 1080 at 200 % (959 ×
/// 462), measured on Windows 11 (19/11/2026).
pub const MIN_SIZE: egui::Vec2 = egui::vec2(900.0, 430.0);
/// The room a taskbar and a window's title bar take on a screen, in points: 48 and 30 on Windows 11
/// at 150 % (measured 19/11/2026), rounded up.
const SCREEN_BARS: f32 = 80.0;

/// Whether a window of `inner` points holds whole, with its title bar, on a screen of `screen`
/// points that keeps room for a taskbar.
fn fits_the_screen(inner: egui::Vec2, screen: egui::Vec2) -> bool {
    inner.x <= screen.x && inner.y + SCREEN_BARS <= screen.y
}
/// Another settings file than the usual one, for a second set or a test: its path.
pub const SETTINGS_VAR: &str = "RSC_SETTINGS";
/// The key under which eframe keeps egui's memory in app.ron (its `STORAGE_EGUI_MEMORY_KEY`).
const EGUI_MEMORY_KEY: &str = "egui";
/// The settings file: `RSC_SETTINGS`, or `settings.json` in the user's data folder.
pub fn settings_path() -> Option<PathBuf> {
    std::env::var_os(SETTINGS_VAR).map(PathBuf::from).or_else(|| eframe::storage_dir(APP_ID).map(|folder| folder.join("settings.json")))
}

/// The window's icon: RSC's logo.
pub fn icon() -> egui::IconData {
    theme::icon()
}

/// The local time, now.
pub fn now() -> Time {
    chrono::Local::now().naive_local()
}

/// What wakes the window from a thread: it draws again.
type Wake = Box<dyn Fn() + Send>;

/// What the tabs talk to: the machine's ports, or those a test plays. It starts each tab's thread,
/// and the one thread that lists the ports for them all.
pub struct Machine {
    link: Box<dyn Fn(Wake) -> Link>,
    watch: Box<dyn Fn(Wake) -> PortWatch>,
}

impl Machine {
    /// The real ports of the machine.
    pub fn system() -> Self {
        Self { link: Box::new(Link::system), watch: Box::new(PortWatch::system) }
    }

    /// Ports that `ports` plays, for the tabs and their list alike.
    pub fn of(ports: impl Ports + Clone) -> Self {
        let watched = ports.clone();
        Self { link: Box::new(move |wake| Link::spawn(ports.clone(), wake)), watch: Box::new(move |wake| PortWatch::spawn(watched.clone(), wake)) }
    }
}

/// Wakes the window of `ctx`.
fn waking(ctx: &egui::Context) -> Wake {
    let ctx = ctx.clone();
    Box::new(move || ctx.request_repaint())
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum State {
    Closed,
    /// Asked of the thread; `again` when the port was lost and is back.
    Opening { port: String, again: bool },
    /// Open on `port` with `link`: a change of either opens it again.
    Open { port: String, link: LinkSettings },
    /// Lost: opened again as soon as it is back.
    Waiting { port: String, tried: Option<Instant> },
}

/// A tab's port as it runs: what the settings do not keep.
struct Session {
    link: Link,
    console: Console,
    view: ConsoleView,
    state: State,
    input: String,
    history: History,
    journal: Option<Journal>,
    /// The last thing that went wrong, shown in the status bar until the next action.
    problem: Option<String>,
    /// The send line takes the focus at the next frame.
    focus_input: bool,
    /// The curves of the measures, whether they are paused, the time they show, their share of
    /// the height under the console.
    plot: Plot,
    plot_paused: bool,
    plot_view: plot_view::View,
    plot_share: f32,
    /// What the middle of the window shows when the curves are on, and whether there was room for
    /// the two at the last frame.
    display: Display,
    room_for_both: bool,
    /// The device's screen in the terminal mode, None out of it: it reads what comes only then.
    terminal: Option<Terminal>,
    terminal_view: TerminalView,
}

/// What the middle of the window shows when the curves are on: the console (or the terminal's
/// screen), the curves, or the two, one above the other. On a window too low for the two, one
/// alone (WCAG 1.4.4): the console, unless the curves are chosen.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Display {
    Console,
    Curves,
    #[default]
    Both,
}

impl Session {
    fn new(link: Link, scrollback: usize, plot_points: usize) -> Self {
        Self {
            terminal: None,
            terminal_view: TerminalView::default(),
            plot: Plot::new(plot_points),
            plot_paused: false,
            plot_view: plot_view::View::default(),
            plot_share: PLOT_SHARE,
            display: Display::default(),
            room_for_both: true,
            link,
            console: Console::new(scrollback),
            view: ConsoleView::default(),
            state: State::Closed,
            input: String::new(),
            history: History::default(),
            journal: None,
            problem: None,
            focus_input: false,
        }
    }

    fn tone(&self) -> Tone {
        match self.state {
            State::Closed if self.problem.is_some() => Tone::Bad,
            State::Closed => Tone::Neutral,
            State::Opening { .. } | State::Waiting { .. } => Tone::Busy,
            State::Open { .. } => Tone::Good,
        }
    }

    fn is_open(&self) -> bool {
        matches!(self.state, State::Open { .. })
    }
}

pub struct App {
    settings: Settings,
    path: Option<PathBuf>,
    /// The settings as last written, to write them again only when they change.
    saved: Settings,
    /// Why the settings could not be written, said once.
    save_problem: Option<String>,
    /// The ports of the machine, as the one thread that lists them last said.
    ports: Vec<PortInfo>,
    watch: PortWatch,
    /// One per tab of the settings, in the same order.
    sessions: Vec<Session>,
    machine: Machine,
    window: Option<settings_window::Window>,
    /// The pill's menu: the name typed for the device's profile, and why the last one was refused.
    pill_name: String,
    pill_problem: Option<String>,
    /// The keyboard moves the focus (Tab), so the focus ring shows; a press of the pointer ends it.
    keyboard: bool,
    /// The window's size was held against the screen's, once at the start.
    screen_checked: bool,
    clock: fn() -> Time,
}

impl App {
    /// The window, its settings read from `path`, on the ports of `machine`: a thread for each tab's
    /// port, one that lists them. Its time is given by `clock`.
    pub fn new(ctx: &egui::Context, path: Option<PathBuf>, machine: Machine, clock: fn() -> Time) -> Self {
        let (settings, loaded) = path.as_deref().map_or((Settings::default(), Loaded::Fresh), Settings::load);
        i18n::set_english(settings.language == Language::English);
        theme::install(ctx);
        ctx.set_theme(theme::preference(settings.theme));
        let sessions = (0..settings.tabs.len())
            .map(|tab| {
                let profile = settings.profile_of(tab);
                Session::new((machine.link)(waking(ctx)), profile.scrollback, profile.plot_points)
            })
            .collect();
        let watch = (machine.watch)(waking(ctx));
        // A file of an earlier version, or in another form than this version writes (the general
        // profile in full), is written again at once.
        let rewrite = matches!(loaded, Loaded::Converted(_)) || (loaded == Loaded::Read && path.as_deref().is_some_and(|path| settings.written_otherwise(path)));
        let saved = if rewrite { Settings { tabs: Vec::new(), ..Settings::default() } } else { settings.clone() };
        let mut app = Self {
            saved,
            settings,
            path,
            save_problem: None,
            ports: Vec::new(),
            watch,
            sessions,
            machine,
            window: None,
            pill_name: String::new(),
            pill_problem: None,
            keyboard: false,
            screen_checked: false,
            clock,
        };
        match loaded {
            Loaded::SetAside(kept) => {
                let text = tr!("Réglages illisibles : mis de côté dans {}, réglages par défaut.", "Settings unreadable: set aside in {}, defaults used.", kept.display());
                app.sessions[0].console.notice(&text, clock());
            }
            Loaded::Converted(kept) => {
                let text = tr!(
                    "Réglages d'une version précédente repris ; l'ancien fichier est gardé dans {}.",
                    "Settings of an earlier version taken in; the old file is kept in {}.",
                    kept.display()
                );
                let active = app.active();
                app.sessions[active].console.notice(&text, clock());
            }
            Loaded::Fresh | Loaded::Read => {}
        }
        app
    }

    /// The window on the machine's ports, its settings in the usual file.
    pub fn system(cc: &eframe::CreationContext) -> Self {
        Self::new(&cc.egui_ctx, settings_path(), Machine::system(), now)
    }

    fn active(&self) -> usize {
        self.settings.active
    }

    // ---------------------------------------------------------------- What happens between frames

    /// At the start, a window too high for the screen with its title bar and the taskbar is
    /// maximized: the system then gives it the room the taskbar leaves (a 1366 × 768 laptop at 150 %,
    /// 19/11/2026). Once, as soon as the system tells the screen's size.
    fn fit_the_screen(&mut self, ctx: &egui::Context) {
        if self.screen_checked {
            return;
        }
        let (screen, inner, maximized) = ctx.input(|input| (input.viewport().monitor_size, input.viewport().inner_rect, input.viewport().maximized));
        if let (Some(screen), Some(inner)) = (screen, inner) {
            self.screen_checked = true;
            if maximized != Some(true) && !fits_the_screen(inner.size(), screen) {
                ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(true));
            }
        }
    }

    fn tick(&mut self, ctx: &egui::Context) {
        self.fit_the_screen(ctx);
        i18n::set_english(self.settings.language == Language::English);
        // Asked again at each frame: whoever changed it, the settings have the last word.
        let wanted = theme::preference(self.settings.theme);
        if ctx.options(|options| options.theme_preference) != wanted {
            ctx.set_theme(wanted);
        }
        for (i, session) in self.sessions.iter_mut().enumerate() {
            let profile = self.settings.profile_of(i);
            session.console.set_limit(profile.scrollback);
            session.plot.set_limit(profile.plot_points);
            // The terminal mode turned on: a blank screen, which the view sizes and takes the keyboard.
            match (profile.terminal, session.terminal.is_some()) {
                (true, false) => {
                    session.terminal = Some(Terminal::new(24, 80, profile.scrollback));
                    session.terminal_view.focus = true;
                }
                (false, true) => session.terminal = None,
                _ => {}
            }
        }
        let now = (self.clock)();
        if let Some(list) = self.watch.latest() {
            self.ports = list;
        }
        for i in 0..self.sessions.len() {
            let events: Vec<Event> = self.sessions[i].link.events().collect();
            for event in events {
                self.on_event(i, event, now);
            }
            self.follow_settings(i, now);
            self.retry(i, now);
            self.hand_lines(i);
        }
        // Every port open, the list can wait two seconds: an open port's read tells when it is lost.
        self.watch.pace(self.sessions.iter().all(Session::is_open));
        if self.sessions.iter().any(|session| matches!(session.state, State::Waiting { .. })) {
            ctx.request_repaint_after(RETRY_EVERY);
        }
        self.save(now);
    }

    fn shown(&self, i: usize) -> Shown {
        let profile = self.settings.profile_of(i);
        Shown { timestamps: profile.timestamps, hex: profile.hex_view }
    }

    fn draw(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        let dark = ui.visuals().dark_mode;
        let palette = Palette::of(dark);
        let now = (self.clock)();
        self.keys_for_the_focus(&ctx);
        egui::CentralPanel::default().frame(egui::Frame::new().fill(palette.page).inner_margin(Margin::same(16))).show(ui, |ui| {
            egui::Frame::new()
                .fill(palette.card)
                .stroke(Stroke::new(1.0_f32, palette.border))
                .corner_radius(CornerRadius::same(16))
                .inner_margin(Margin::same(16))
                .show(ui, |ui| {
                    ui.set_min_size(ui.available_size());
                    let up = self.tabs_bar(ui, &palette, dark, now);
                    ui.add_space(4.0);
                    self.connection_bar(ui, now);
                    self.status_bar(ui, &palette, dark, up);
                    // The send line, or in the terminal mode its line of shortcuts, which can take two
                    // rows; then the actions.
                    let send = match &self.sessions[self.active()].terminal {
                        Some(terminal) => {
                            let (rows, cols) = terminal.size();
                            shortcuts_height(ui, &shortcuts_text(rows, cols), ui.available_width())
                        }
                        None => CONTROL_HEIGHT,
                    };
                    let below = send + CONTROL_HEIGHT + 3.0 * ui.spacing().item_spacing.y;
                    let height = (ui.available_height() - below).max(80.0);
                    ui.allocate_ui(egui::vec2(ui.available_width(), height), |ui| self.console_and_curves(ui, &palette, now));
                    self.send_bar(ui, now);
                    self.actions_bar(ui, &ctx, now);
                });
        });
        if let Some(window) = &mut self.window {
            let active = self.settings.active;
            if !settings_window::show(&ctx, window, &mut self.settings, active, &self.ports, self.path.as_deref()) {
                self.window = None;
            }
        }
        if self.keyboard {
            theme::focus_ring(&ctx);
        }
    }

    /// Tab shows the focus ring, a press of the pointer hides it. Ctrl+Tab and Ctrl+Shift+Tab move
    /// the focus on, out of the terminal's screen too, which keeps Tab for the device: a device
    /// cannot tell Ctrl+Tab from Tab. Before any widget, for the move to start from the focused one.
    fn keys_for_the_focus(&mut self, ctx: &egui::Context) {
        let (tab, pointer) = ctx.input(|input| (input.events.iter().any(|event| matches!(event, egui::Event::Key { key: Key::Tab, pressed: true, .. })), input.pointer.any_pressed()));
        if tab {
            self.keyboard = true;
        } else if pointer {
            self.keyboard = false;
        }
        let direction = ctx.input_mut(|input| {
            if input.consume_key(Modifiers::CTRL | Modifiers::SHIFT, Key::Tab) {
                Some(egui::FocusDirection::Previous)
            } else if input.consume_key(Modifiers::CTRL, Key::Tab) {
                Some(egui::FocusDirection::Next)
            } else {
                None
            }
        });
        if let Some(direction) = direction {
            ctx.memory_mut(|memory| memory.move_focus(direction));
        }
    }
}

impl eframe::App for App {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.tick(ctx);
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.draw(ui);
    }

    /// Only the window's size and place are kept (app.ron), not egui's memory: 18 KB of options and
    /// positions that the settings already hold or that nobody misses.
    fn persist_egui_memory(&self) -> bool {
        false
    }

    /// Called just before app.ron is written. eframe neither writes egui's memory any more nor takes
    /// away what an earlier version wrote, and reads it back at each start: it is taken away here.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        storage.remove_string(EGUI_MEMORY_KEY);
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        let now = (self.clock)();
        for i in 0..self.sessions.len() {
            self.hand_lines(i);
        }
        self.saved = Settings { tabs: Vec::new(), ..Settings::default() };
        self.save(now);
    }
}

/// Where a test's files go: a folder of its own, emptied first.
#[cfg(test)]
fn test_settings(test: &str) -> PathBuf {
    crate::settings::test_folder(test).join("settings.json")
}

#[cfg(test)]
mod tests;
