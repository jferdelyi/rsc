//! The top of the window: the tabs, the connection bar (the port or named device, the profile's
//! pill, the baud rate, « Connecter »), and the state line with its switches.

use eframe::egui::{self, CornerRadius, RichText, TextStyle};

use crate::console::Time;
use crate::ports::{self, PortInfo};
use crate::settings::{self, BAUD_RATES, Device};
use super::bottom::ACTION_WIDTH;
use super::icons;
use super::pill::{PILL_GUESS, pill_width_id, profile_label};
use super::session::flow_label;
use super::settings_window;
use super::theme::{self, Palette, Tone};
use super::widgets;
use super::{App, State};

/// The widths of the controls of the bars: their edges line up from one bar to the other.
const BAUD_WIDTH: f32 = 112.0;
/// A tab's name: its named device's, else its port.
pub(super) fn tab_name(tab: &settings::Tab) -> String {
    match (&tab.device, tab.port.is_empty()) {
        (Some(device), _) => device.clone(),
        (None, false) => tab.port.clone(),
        (None, true) => t!("Nouvel onglet", "New tab").to_owned(),
    }
}

/// The port a named device is plugged in now, if it is.
pub(super) fn port_of(device: &Device, ports: &[PortInfo]) -> Option<String> {
    ports::resolve(&device.last_port, Some(&device.usb), ports)
}

/// A piece of the state of a tab: its text, its colour, what its tooltip says.
type Piece = (String, egui::Color32, Option<String>);

/// Where the switches of the state's row keep the width they took.
const STATUS_SWITCHES: &str = "status-switches";
/// The state line's identity, wherever it stands.
const STATE_LINE: &str = "state-line";

/// The width the state takes on one row, its point included.
fn state_width(ui: &egui::Ui, pieces: &[Piece]) -> f32 {
    let body = TextStyle::Body.resolve(ui.style());
    let spacing = ui.spacing().item_spacing.x;
    pieces.iter().fold(10.0, |width, (piece, _, _)| width + spacing + ui.ctx().fonts_mut(|fonts| fonts.layout_no_wrap(piece.clone(), body.clone(), egui::Color32::PLACEHOLDER).size().x))
}

/// The state of a tab: its mark, then its pieces. To screen readers, one status, announced politely
/// when its words change (WCAG 4.1.3); the same element on the row of the tabs or under the
/// connection bar, for a move from one to the other not to be read again.
fn state_line(ui: &mut egui::Ui, (tone, color): (Tone, egui::Color32), pieces: &[Piece]) {
    let id = egui::Id::new(STATE_LINE);
    let shown = ui.scope_builder(egui::UiBuilder::new().id(id), |ui| {
        ui.ctx().accesskit_node_builder(ui.unique_id(), |node| {
            node.set_role(egui::accesskit::Role::Status);
            node.set_live(egui::accesskit::Live::Polite);
            node.set_label(words_of(pieces));
        });
        theme::mark(ui, tone, color);
        for (piece, color, hover) in pieces {
            let label = ui.label(RichText::new(piece).color(*color));
            if let Some(hover) = hover {
                label.on_hover_text(hover);
            }
        }
    });
    // Its place, once laid out: where a screen reader frames it.
    ui.ctx().accesskit_node_builder(id, |node| node.set_bounds(widgets::bounds(shown.response.rect)));
}

/// The words of the state line, its pieces joined.
fn words_of(pieces: &[Piece]) -> String {
    pieces.iter().map(|(piece, _, _)| piece.as_str()).collect::<Vec<_>>().join(" ")
}

impl App {
    /// The tabs, and the state of the one in front when it goes up here: then true.
    pub(super) fn tabs_bar(&mut self, ui: &mut egui::Ui, palette: &Palette, dark: bool, now: Time) -> bool {
        let ctx = ui.ctx().clone();
        let (mut select, mut close, mut add, mut up) = (None, None, false, false);
        let (tone, pieces) = self.state_pieces(self.active(), palette, dark);
        ui.horizontal(|ui| {
            let many = self.sessions.len() > 1;
            for (i, session) in self.sessions.iter().enumerate() {
                theme::mark(ui, session.tone(), palette.tone(session.tone(), dark));
                let tab = &self.settings.tabs[i];
                let name = tab_name(tab);
                let state = self.state_words(i, palette, dark);
                let which = profile_label(&self.settings, i);
                let hint = match &tab.device {
                    Some(_) => tr!("{state}\nAppareil nommé, sur {}, avec le profil {which}", "{state}\nNamed device, on {}, with the profile {which}", tab.port),
                    None => tr!("{state}\nUn port, avec le profil {which}", "{state}\nA port, with the profile {which}"),
                };
                let chosen = i == self.settings.active;
                let response = ui.add(egui::Button::selectable(chosen, name));
                widgets::describe(&response, &state);
                let response = response.on_hover_text(hint);
                if chosen {
                    // Marked by more than its fill, too close to the card's (WCAG 1.4.11): a line of the
                    // accent under it.
                    let line = egui::Rect::from_min_max(egui::pos2(response.rect.left() + 6.0, response.rect.bottom() - 2.0), egui::pos2(response.rect.right() - 6.0, response.rect.bottom()));
                    ui.painter().rect_filled(line, CornerRadius::same(1), palette.accent);
                }
                if response.clicked() {
                    select = Some(i);
                }
                if many && widgets::bare_icon_button(ui, icons::X, t!("Fermer l'onglet", "Close the tab"), t!("Fermer l'onglet et son port", "Close the tab and its port")).clicked() {
                    close = Some(i);
                }
                ui.add_space(8.0);
            }
            if widgets::bare_icon_button(ui, icons::PLUS, t!("Nouvel onglet", "New tab"), t!("Nouvel onglet : un autre port en même temps", "New tab: another port at the same time")).clicked() {
                add = true;
            }
            // The state of the tab in front, up at the right of the tabs, when it does not hold
            // with the switches under the connection bar and holds here: a row more for the
            // console.
            let full = ui.max_rect().width();
            let switches = widgets::measured(ui, egui::Id::new(STATUS_SWITCHES)).unwrap_or(0.0);
            let natural = state_width(ui, &pieces);
            if natural + 16.0 + switches > full && natural + 16.0 <= ui.available_width() {
                widgets::at_right(ui, egui::Id::new("tabs-state"), |ui| state_line(ui, tone, &pieces));
                up = true;
            }
        });
        if let Some(i) = select {
            self.settings.active = i;
            let session = &mut self.sessions[i];
            if session.terminal.is_some() {
                session.terminal_view.focus = true;
            } else {
                session.focus_input = true;
            }
        }
        if let Some(i) = close {
            self.close_tab(i, now);
        }
        if add {
            self.add_tab(&ctx);
        }
        up
    }

    pub(super) fn connection_bar(&mut self, ui: &mut egui::Ui, now: Time) {
        let i = self.active();
        ui.horizontal(|ui| {
            let spacing = ui.spacing().item_spacing.x;
            // The pill grows with a profile's name: the device's selector gives it the room.
            let pill = self.pill_words(i);
            let pill_width = widgets::measured(ui, pill_width_id(i)).unwrap_or(PILL_GUESS);
            let device_width = (ui.available_width() - pill_width - BAUD_WIDTH - ACTION_WIDTH - 3.0 * spacing).max(120.0);
            self.device_choice(ui, i, device_width);
            self.profile_pill(ui, i, pill);
            self.baud_choice(ui, i);
            let (label, enabled, hint) = match self.sessions[i].state {
                State::Closed => (t!("Connecter", "Connect"), !self.settings.tabs[i].port.is_empty(), t!("Ouvrir le port avec cette liaison", "Open the port with this link")),
                State::Opening { .. } => (t!("Annuler", "Cancel"), true, t!("Ne plus essayer d'ouvrir le port", "Stop trying to open the port")),
                State::Open { .. } => (t!("Déconnecter", "Disconnect"), true, t!("Fermer le port", "Close the port")),
                State::Waiting { .. } => (t!("Arrêter", "Stop"), true, t!("Ne plus attendre le retour du port", "Stop waiting for the port to come back")),
            };
            if theme::primary_button(ui, label, ACTION_WIDTH, enabled).on_hover_text(hint).on_disabled_hover_text(t!("Choisissez d'abord un port", "Choose a port first")).clicked() {
                match self.sessions[i].state {
                    State::Closed => self.connect(i, now),
                    _ => self.disconnect(i, now),
                }
            }
        });
    }

    /// The named devices, plugged in or not, then the ports that carry none.
    fn device_choice(&mut self, ui: &mut egui::Ui, i: usize, width: f32) {
        let tab = &self.settings.tabs[i];
        let label_of = |device: &Device| match port_of(device, &self.ports) {
            Some(port) => format!("{} · {port}", device.name),
            None => tr!("{} (absent)", "{} (missing)", device.name),
        };
        let selected = match tab.device.as_deref().and_then(|name| self.settings.find_device(name)) {
            Some(device) => label_of(device),
            None => match self.ports.iter().find(|port| port.name == tab.port) {
                Some(port) => port.label(),
                None if tab.port.is_empty() => t!("Choisir un appareil ou un port", "Choose a device or a port").to_owned(),
                None => tr!("{} (absent)", "{} (missing)", tab.port),
            },
        };
        let (mut device, mut port, mut name_it) = (None, None, false);
        // A USB device on the tab's port that has no name yet: it can get one from here.
        let unnamed = tab.device.is_none() && self.ports.iter().any(|known| known.name == tab.port && known.usb.as_ref().is_some_and(|usb| self.settings.device_of(usb).is_none()));
        let list = egui::ComboBox::from_id_salt(("device", i))
            .width(width)
            .selected_text(selected.clone())
            .truncate()
            .show_ui(ui, |ui| {
                for known in &self.settings.devices {
                    let chosen = tab.device.as_deref() == Some(known.name.as_str());
                    if ui.selectable_label(chosen, label_of(known)).on_hover_text(known.usb.label()).clicked() {
                        device = Some(known.name.clone());
                    }
                }
                let bare: Vec<&PortInfo> = self.ports.iter().filter(|port| port.usb.as_ref().is_none_or(|usb| self.settings.device_of(usb).is_none())).collect();
                if !self.settings.devices.is_empty() && !bare.is_empty() {
                    ui.separator();
                }
                for known in bare {
                    if ui.selectable_label(tab.device.is_none() && known.name == tab.port, known.label()).clicked() {
                        port = Some(known.clone());
                    }
                }
                if self.ports.is_empty() && self.settings.devices.is_empty() {
                    ui.label(t!("Aucun port détecté", "No port found"));
                }
                if unnamed {
                    ui.separator();
                    if ui.button((icons::TAG.text(14.0), "  ", t!("Nommer cet appareil…", "Name this device…"))).clicked() {
                        name_it = true;
                    }
                }
            })
            .response;
        widgets::name(&list, t!("Appareil", "Device"));
        list.on_hover_text(tr!(
            "{selected}\nUn appareil nommé est retrouvé sur n'importe quel port, avec son profil (réglages, Appareils).",
            "{selected}\nA named device is found on whatever port, with its profile (settings, Devices)."
        ));
        if let Some(name) = device {
            self.settings.choose_device(i, &name);
            if let Some(found) = self.settings.find_device(&name).and_then(|known| port_of(known, &self.ports)) {
                self.settings.tabs[i].port = found;
            }
        }
        if let Some(port) = port {
            self.settings.choose_port(i, &port.name, port.usb.as_ref());
        }
        if name_it {
            self.window = Some(settings_window::Window::for_device(self.product_on(i)));
        }
    }

    /// The baud rate of the profile the tab uses.
    fn baud_choice(&mut self, ui: &mut egui::Ui, i: usize) {
        let before = self.settings.profile_of(i).baud_rate;
        let mut rate = before;
        let which = profile_label(&self.settings, i);
        let list = egui::ComboBox::from_id_salt(("baud", i))
            .width(BAUD_WIDTH)
            .selected_text(rate.to_string())
            .show_ui(ui, |ui| {
                if !BAUD_RATES.contains(&rate) {
                    ui.selectable_value(&mut rate, before, before.to_string());
                }
                for value in BAUD_RATES {
                    ui.selectable_value(&mut rate, value, value.to_string());
                }
            })
            .response;
        widgets::name(&list, t!("Débit", "Baud rate"));
        list.on_hover_text(tr!("Le débit du profil {which} ; toute autre valeur dans les réglages", "The baud rate of the profile {which}; any other value in the settings"));
        if rate != before {
            self.settings.edit_tab_profile(i, |profile| profile.baud_rate = rate);
        }
    }

    /// The state of tab `i` in words, with its tone and colour: the link, then the last problem and
    /// the journal running, each with its own colour, the journal's path in its tooltip.
    fn state_pieces(&self, i: usize, palette: &Palette, dark: bool) -> ((Tone, egui::Color32), Vec<Piece>) {
        let session = &self.sessions[i];
        let text = match &session.state {
            State::Closed => t!("Déconnecté", "Disconnected").to_owned(),
            State::Opening { port, .. } => tr!("Connexion à {port}…", "Connecting to {port}…"),
            State::Open { port, link } => tr!("Connecté à {port} · {} · {}", "Connected to {port} · {} · {}", link.summary(), flow_label(link.flow_control)),
            State::Waiting { port, .. } => tr!("{port} perdu : reconnexion dès son retour…", "{port} lost: reconnecting as soon as it is back…"),
        };
        let color = palette.tone(session.tone(), dark);
        let mut pieces = vec![(text, color, None)];
        if let Some(problem) = &session.problem {
            pieces.push((format!("· {problem}"), palette.tone(Tone::Bad, dark), None));
        }
        if let Some(journal) = &session.journal {
            let name = journal.path().file_name().map(|name| name.to_string_lossy().into_owned()).unwrap_or_default();
            pieces.push((tr!("· journal {name}", "· journal {name}"), palette.muted, Some(journal.path().display().to_string())));
        }
        ((session.tone(), color), pieces)
    }

    /// The state of tab `i` in the words of the state line, for its tooltip and its description to
    /// screen readers (WCAG 1.4.1): its mark alone shows it on screen.
    fn state_words(&self, i: usize, palette: &Palette, dark: bool) -> String {
        words_of(&self.state_pieces(i, palette, dark).1)
    }

    /// The switches of the tab in front, under the connection bar, with its state when they hold
    /// on one row and the state is not up on the row of the tabs (`up`); else on a row of their own
    /// under the state, which stays whole (WCAG 1.4.4).
    pub(super) fn status_bar(&mut self, ui: &mut egui::Ui, palette: &Palette, dark: bool, up: bool) {
        let i = self.active();
        let mut profile = self.settings.profile_of(i);
        let which = profile_label(&self.settings, i);
        let mut follow = self.sessions[i].view.follows();
        let (tone, pieces) = self.state_pieces(i, palette, dark);
        let natural = state_width(ui, &pieces);
        // The switches set against the right edge, added in the order they are read (WCAG 2.4.3).
        let switches = egui::Id::new(STATUS_SWITCHES);
        let one_row = natural + 16.0 + widgets::measured(ui, switches).unwrap_or(0.0) <= ui.available_width();
        let mut add_switches = |ui: &mut egui::Ui| {
            widgets::at_right(ui, switches, |ui| {
                ui.spacing_mut().item_spacing.x = 16.0;
                // The line console's own: greyed in the terminal mode, which shows the device's screen.
                let terminal = profile.terminal;
                let line_only = t!("Sans effet en mode terminal : c'est un réglage de la console en mode ligne", "No effect in the terminal mode: a setting of the line console");
                let hint = |text: String| if terminal { line_only.to_owned() } else { text };
                ui.add_enabled_ui(!terminal, |ui| {
                    widgets::switch(ui, &mut profile.timestamps, t!("Horodatage", "Timestamps"), &hint(tr!("L'heure de chaque ligne, à la milliseconde · profil {which}", "The time of each line, to the millisecond · profile {which}")));
                    widgets::switch(
                        ui,
                        &mut follow,
                        t!("Défilement auto", "Auto scroll"),
                        &hint(t!("La console suit ses dernières lignes ; remonter la met en pause, revenir tout en bas la relance", "The console follows its last lines; going up pauses it, coming back to the bottom starts it again").to_owned()),
                    );
                    widgets::switch(ui, &mut profile.hex_view, t!("Affichage hex", "Hex view"), &hint(tr!("Les octets reçus et envoyés, en hexadécimal · profil {which}", "The bytes received and sent, in hexadecimal · profile {which}")));
                });
                widgets::switch(ui, &mut profile.plot, t!("Courbes", "Curves"), &tr!("Les lignes faites de valeurs, en courbes sous la console · profil {which}", "The lines made of values, as curves under the console · profile {which}"));
                widgets::switch(
                    ui,
                    &mut profile.terminal,
                    t!("Terminal", "Terminal"),
                    &tr!(
                        "Les touches vont à l'appareil une à une, et son écran se dessine comme dans un terminal (vi, menus). Ctrl+C copie la sélection, sinon interrompt ; Ctrl+V colle · profil {which}",
                        "The keys go to the device one by one, and its screen is drawn as in a terminal (vi, menus). Ctrl+C copies the selection, else interrupts; Ctrl+V pastes · profile {which}"
                    ),
                );
            });
        };
        if up {
            ui.horizontal(|ui| {
                ui.set_min_height(24.0);
                add_switches(ui);
            });
        } else if one_row {
            ui.horizontal(|ui| {
                ui.set_min_height(24.0);
                state_line(ui, tone, &pieces);
                add_switches(ui);
            });
        } else {
            ui.horizontal_wrapped(|ui| {
                ui.set_min_height(24.0);
                state_line(ui, tone, &pieces);
            });
            ui.horizontal(|ui| {
                ui.set_min_height(24.0);
                add_switches(ui);
            });
        }
        if follow != self.sessions[i].view.follows() {
            self.sessions[i].view.set_follow(follow);
        }
        self.settings.edit_tab_profile(i, |edited| *edited = profile);
    }
}
