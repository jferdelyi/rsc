//! A tab's port as it runs: its state, the events of its thread, opening, closing and
//! opening again, the lines sent, the lines handed to the curves and the journal; tabs added and
//! closed, and the settings written.

use std::time::{Duration, Instant};

use eframe::egui;

use crate::console::{Shown, Time};
use crate::link::{Command, Event};
use crate::ports;
use crate::send::{self, EscapeError};
use crate::settings::FlowControl;
use super::{App, Session, State, waking};

/// How often a lost port that is back is tried again.
pub(super) const RETRY_EVERY: Duration = Duration::from_secs(1);
pub(super) fn flow_label(flow: FlowControl) -> &'static str {
    match flow {
        FlowControl::None => t!("sans contrôle de flux", "no flow control"),
        FlowControl::Software => t!("contrôle de flux XON/XOFF", "XON/XOFF flow control"),
        FlowControl::Hardware => t!("contrôle de flux RTS/CTS", "RTS/CTS flow control"),
    }
}

fn escape_problem(error: EscapeError) -> String {
    match error {
        EscapeError::Unknown(c) => tr!(
            "« \\{c} » n'est pas une séquence connue : \\n, \\r, \\t, \\0, \\e, \\\\ ou \\xNN.",
            "\"\\{c}\" is not a known sequence: \\n, \\r, \\t, \\0, \\e, \\\\ or \\xNN."
        ),
        EscapeError::Hex => t!("\\x attend deux chiffres hexadécimaux, comme \\x03.", "\\x takes two hexadecimal digits, as in \\x03.").to_owned(),
        EscapeError::Dangling => {
            t!("La ligne finit par une barre oblique inverse : \\\\ pour en envoyer une.", "The line ends with a backslash: \\\\ sends one.").to_owned()
        }
    }
}

impl App {
    pub(super) fn on_event(&mut self, i: usize, event: Event, now: Time) {
        let session = &mut self.sessions[i];
        match event {
            Event::Data(bytes) => {
                session.console.receive(&bytes, now);
                // The device's questions to its terminal (where the cursor is, how big the screen is)
                // answered at once.
                if let Some(terminal) = &mut session.terminal {
                    let answers = terminal.receive(&bytes);
                    if !answers.is_empty() {
                        session.link.send(Command::Send(answers));
                    }
                }
            }
            Event::Opened { port, link } => {
                session.console.notice(&tr!("Connecté à {port} · {} · {}", "Connected to {port} · {} · {}", link.summary(), flow_label(link.flow_control)), now);
                session.state = State::Open { port: port.clone(), link };
                // Connected in the terminal mode: the keys go to the device at once.
                session.terminal_view.focus = session.terminal.is_some();
                session.problem = None;
                if let Some(name) = self.settings.tabs[i].device.clone() {
                    self.settings.seen(&name, &port);
                }
                let profile = self.settings.profile_of(i);
                if profile.journal_on_connect && profile.journal_folder().is_some() && self.sessions[i].journal.is_none() {
                    self.start_journal(i, now);
                }
            }
            Event::Refused { port, error } => {
                if matches!(session.state, State::Opening { again: true, .. }) {
                    // Back in the list but not ready yet: tried again a moment later.
                    session.state = State::Waiting { port, tried: Some(Instant::now()) };
                } else {
                    let text = tr!("{port} ne s'ouvre pas : {error}", "{port} does not open: {error}");
                    session.console.notice(&text, now);
                    session.problem = Some(text);
                    session.state = State::Closed;
                }
            }
            // Closed as asked: when it is to open it again, the state is already the next one.
            Event::Closed(None) => {
                if !matches!(session.state, State::Opening { .. }) {
                    session.console.notice(t!("Déconnecté", "Disconnected"), now);
                    session.state = State::Closed;
                }
            }
            Event::Closed(Some(why)) => {
                let port = match &session.state {
                    State::Open { port, .. } | State::Opening { port, .. } => port.clone(),
                    _ => self.settings.tabs[i].port.clone(),
                };
                session.console.notice(&tr!("{port} perdu : {why}", "{port} lost: {why}"), now);
                if self.settings.profile_of(i).reconnect {
                    session.console.notice(t!("Reconnexion dès son retour.", "Reconnecting as soon as it is back."), now);
                    session.state = State::Waiting { port, tried: None };
                } else {
                    session.problem = Some(tr!("{port} perdu", "{port} lost"));
                    session.state = State::Closed;
                }
            }
        }
    }

    /// An open port whose port or link changed (the baud rate, the profile in the settings window)
    /// is opened again with them.
    pub(super) fn follow_settings(&mut self, i: usize, now: Time) {
        let tab = &self.settings.tabs[i];
        let wanted = self.settings.profile_of(i).link();
        if let State::Open { port, link } = &self.sessions[i].state
            && (port != &tab.port || link != &wanted)
            && !tab.port.is_empty()
        {
            self.open(i, tab.port.clone(), false, now);
        }
    }

    /// A lost port that is back is opened again, once a second at most.
    pub(super) fn retry(&mut self, i: usize, now: Time) {
        let State::Waiting { port, tried } = &self.sessions[i].state else {
            return;
        };
        if tried.is_some_and(|tried| tried.elapsed() < RETRY_EVERY) {
            return;
        }
        if let Some(found) = ports::resolve(port, self.usb_of(i), &self.ports) {
            self.open(i, found, true, now);
        }
    }

    /// The USB device a tab follows: its named device's.
    fn usb_of(&self, i: usize) -> Option<&crate::ports::UsbId> {
        self.settings.tabs[i].device.as_deref().and_then(|name| self.settings.find_device(name)).map(|device| &device.usb)
    }

    fn open(&mut self, i: usize, port: String, again: bool, now: Time) {
        let link = self.settings.profile_of(i).link();
        let tab = &mut self.settings.tabs[i];
        let session = &mut self.sessions[i];
        if port != tab.port {
            if !tab.port.is_empty() && tab.device.is_some() {
                session.console.notice(&tr!("{} est devenu {port} : même appareil USB.", "{} became {port}: the same USB device.", tab.port), now);
            }
            tab.port.clone_from(&port);
        }
        session.link.send(Command::Open { port: port.clone(), link });
        session.state = State::Opening { port, again };
        session.problem = None;
    }

    /// The connection button: the port of the tab, or the one its USB device is now on.
    pub(super) fn connect(&mut self, i: usize, now: Time) {
        let tab = &self.settings.tabs[i];
        // A port the list does not show (a path typed by hand) is still tried: the system answers.
        let port = ports::resolve(&tab.port, self.usb_of(i), &self.ports).unwrap_or_else(|| tab.port.clone());
        self.open(i, port, false, now);
    }

    pub(super) fn disconnect(&mut self, i: usize, now: Time) {
        let session = &mut self.sessions[i];
        match session.state {
            State::Waiting { .. } => session.console.notice(t!("Reconnexion arrêtée", "Reconnection stopped"), now),
            _ => session.link.send(Command::Close),
        }
        session.state = State::Closed;
    }

    pub(super) fn send_line(&mut self, i: usize, now: Time) {
        let profile = self.settings.profile_of(i);
        let session = &mut self.sessions[i];
        let text = session.input.clone();
        let bytes = if profile.hex_input {
            send::parse_hex(&text).map_err(|piece| {
                tr!("« {piece} » n'est pas de l'hexadécimal : des octets comme 48 65 6C ou 0x48.", "\"{piece}\" is not hexadecimal: bytes such as 48 65 6C or 0x48.")
            })
        } else {
            send::payload(&text, profile.line_ending, profile.escapes).map_err(escape_problem)
        };
        match bytes {
            Ok(bytes) => {
                session.link.send(Command::Send(bytes.clone()));
                if profile.local_echo {
                    session.console.sent(&text, &bytes, now);
                }
                session.history.push(&text);
                session.input.clear();
                session.problem = None;
            }
            Err(problem) => session.problem = Some(problem),
        }
    }

    /// The lines that ended, handed to the journal and to the curves, those of them that are on.
    pub(super) fn hand_lines(&mut self, i: usize) {
        let profile = self.settings.profile_of(i);
        let session = &mut self.sessions[i];
        let following = session.journal.is_some() || profile.plot;
        session.console.follow(following);
        if !following {
            return;
        }
        let lines = session.console.take_ended();
        if lines.is_empty() {
            return;
        }
        if profile.plot && !session.plot_paused {
            for line in &lines {
                session.plot.read(line, &profile.plot_prefix);
            }
        }
        let shown = Shown { timestamps: profile.journal_timestamps, hex: profile.hex_view };
        let failed = session.journal.as_mut().and_then(|journal| {
            let path = journal.path().display().to_string();
            journal.write(&lines, shown).err().map(|error| tr!("Journal arrêté, {path} ne s'écrit plus : {error}", "Journal stopped, {path} can no longer be written: {error}"))
        });
        if let Some(problem) = failed {
            session.journal = None;
            session.problem = Some(problem);
        }
    }

    pub(super) fn add_tab(&mut self, ctx: &egui::Context) {
        let tab = self.settings.add_tab();
        let profile = self.settings.profile_of(tab);
        self.sessions.push(Session::new((self.machine.link)(waking(ctx)), profile.scrollback, profile.plot_points));
    }

    pub(super) fn close_tab(&mut self, i: usize, now: Time) {
        if self.sessions.len() > 1 {
            self.stop_journal(i, now);
            // The thread ends with its end of the link, and closes the port.
            self.sessions[i].link.send(Command::Close);
            self.sessions.remove(i);
            self.settings.close_tab(i);
        }
    }

    pub(super) fn save(&mut self, now: Time) {
        let Some(path) = &self.path else {
            return;
        };
        if self.settings == self.saved {
            return;
        }
        match self.settings.save(path) {
            Ok(()) => {
                self.saved = self.settings.clone();
                self.save_problem = None;
            }
            Err(error) => {
                let problem = tr!("Réglages non enregistrés dans {} : {error}", "Settings not saved in {}: {error}", path.display());
                if self.save_problem.as_ref() != Some(&problem) {
                    let active = self.active();
                    self.sessions[active].console.notice(&problem, now);
                    self.save_problem = Some(problem);
                }
                // Not tried again at every frame: at the next change.
                self.saved = self.settings.clone();
            }
        }
    }

    // ---------------------------------------------------------------- The window
}
