//! The bottom of the window: the send line, or the terminal's shortcuts in its place, and the
//! actions.

use eframe::egui::{self, Align, Key, Layout, Modifiers, TextStyle};

use crate::console::Time;
use crate::settings::LineEnding;
use super::icons;
use super::middle::shortcuts_line;
use super::settings_window;
use super::theme::{self, CONTROL_HEIGHT, Palette, Tone};
use super::widgets;
use super::{App, Display};

pub(super) const ACTION_WIDTH: f32 = 140.0;
const ENDING_WIDTH: f32 = 96.0;
const HEX_INPUT_WIDTH: f32 = 128.0;
fn ending_label(ending: LineEnding) -> &'static str {
    match ending {
        LineEnding::None => t!("Rien", "None"),
        LineEnding::Lf => "LF",
        LineEnding::Cr => "CR",
        LineEnding::CrLf => "CR LF",
    }
}

impl App {
    pub(super) fn send_bar(&mut self, ui: &mut egui::Ui, now: Time) {
        let i = self.active();
        if let Some(terminal) = &self.sessions[i].terminal {
            let (rows, cols) = terminal.size();
            shortcuts_line(ui, rows, cols);
            return;
        }
        let id = egui::Id::new(("input", i));
        // The arrows walk the lines sent before the field sees them.
        if ui.ctx().memory(|memory| memory.has_focus(id)) {
            let (up, down) = ui.ctx().input_mut(|input| (input.consume_key(Modifiers::NONE, Key::ArrowUp), input.consume_key(Modifiers::NONE, Key::ArrowDown)));
            let session = &mut self.sessions[i];
            let line = if up { session.history.older(&session.input).map(str::to_owned) } else if down { session.history.newer().map(str::to_owned) } else { None };
            if let Some(line) = line {
                session.input = line;
                if let Some(mut state) = egui::TextEdit::load_state(ui.ctx(), id) {
                    let end = egui::text::CCursor::new(session.input.chars().count());
                    state.cursor.set_char_range(Some(egui::text::CCursorRange::one(end)));
                    state.store(ui.ctx(), id);
                }
            }
        }
        let mut send = false;
        let mut profile = self.settings.profile_of(i);
        ui.horizontal(|ui| {
            let spacing = ui.spacing().item_spacing.x;
            // A label that stays when a line is typed (WCAG 3.3.2), the first word of the field's name
            // (2.5.3).
            ui.label(if profile.hex_input { t!("Octets", "Bytes") } else { t!("Ligne", "Line") });
            let field_width = (ui.available_width() - HEX_INPUT_WIDTH - ENDING_WIDTH - ACTION_WIDTH - 3.0 * spacing).max(120.0);
            let session = &mut self.sessions[i];
            let hint = if profile.hex_input { t!("Octets, par exemple 48 65 6C", "Bytes, for instance 48 65 6C") } else { t!("Ligne à envoyer, puis Entrée", "Line to send, then Enter") };
            let response = ui.add(
                egui::TextEdit::singleline(&mut session.input)
                    .id(id)
                    .hint_text(hint)
                    .font(TextStyle::Monospace)
                    .desired_width(field_width)
                    .min_size(egui::vec2(field_width, CONTROL_HEIGHT))
                    .vertical_align(Align::Center),
            );
            widgets::name(&response, if profile.hex_input { t!("Octets à envoyer", "Bytes to send") } else { t!("Ligne à envoyer", "Line to send") });
            if std::mem::take(&mut session.focus_input) {
                response.request_focus();
            }
            if response.changed() {
                session.problem = None;
            }
            if response.lost_focus() && ui.input(|input| input.key_pressed(Key::Enter)) {
                send = true;
                response.request_focus();
            }
            ui.allocate_ui_with_layout(egui::vec2(HEX_INPUT_WIDTH, CONTROL_HEIGHT), Layout::left_to_right(Align::Center), |ui| {
                // Its whole width, so that the send button ends where the connect button does.
                ui.set_min_width(HEX_INPUT_WIDTH);
                widgets::switch(ui, &mut profile.hex_input, t!("Saisie hex", "Hex input"), t!("La ligne est lue comme des octets : 48 65 6C, 0x48…", "The line is read as bytes: 48 65 6C, 0x48…"));
            });
            ui.add_enabled_ui(!profile.hex_input, |ui| {
                let list = egui::ComboBox::from_id_salt(("ending", i)).width(ENDING_WIDTH).selected_text(ending_label(profile.line_ending)).show_ui(ui, |ui| {
                    for ending in [LineEnding::None, LineEnding::Lf, LineEnding::Cr, LineEnding::CrLf] {
                        ui.selectable_value(&mut profile.line_ending, ending, ending_label(ending));
                    }
                });
                widgets::name(&list.response, t!("Fin de ligne", "Line ending"));
            })
            .response
            .on_hover_text(t!("Ce qui suit chaque ligne envoyée", "What follows each line sent"));
            let something = !(profile.hex_input && session.input.trim().is_empty());
            if theme::primary_button(ui, t!("Envoyer", "Send"), ACTION_WIDTH, session.is_open() && something)
                .on_hover_text(t!("Envoyer la ligne (Entrée) ; les flèches reprennent les précédentes", "Send the line (Enter); the arrows bring back the previous ones"))
                .on_disabled_hover_text(t!("Il faut un port ouvert", "A port must be open"))
                .clicked()
            {
                send = true;
            }
        });
        self.settings.edit_tab_profile(i, |edited| *edited = profile);
        if send && self.sessions[i].is_open() {
            self.send_line(i, now);
            self.sessions[i].focus_input = true;
        }
    }

    pub(super) fn actions_bar(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, now: Time) {
        let i = self.active();
        let console_empty = self.sessions[i].console.is_empty();
        // In the terminal mode, copying and clearing are the screen's.
        let terminal = self.sessions[i].terminal.as_ref();
        let empty = terminal.map_or(console_empty, |terminal| terminal.screen().contents().trim().is_empty() && terminal.history() == 0);
        let in_terminal = terminal.is_some();
        let open = self.sessions[i].is_open();
        let plot_on = self.settings.profile_of(i).plot;
        let dark = ui.visuals().dark_mode;
        ui.horizontal(|ui| {
            if widgets::icon_button(ui, icons::SETTINGS, t!("Réglages", "Settings"), t!("Réglages : profils, appareils, liaison, console…", "Settings: profiles, devices, link, console…"), true).clicked() {
                self.window = Some(settings_window::Window::for_tab(&self.settings, i));
            }
            let copy_hint = if in_terminal {
                t!("Copier la sélection, ou tout l'écran s'il n'y en a pas", "Copy the selection, or the whole screen when there is none")
            } else {
                t!("Copier la sélection, ou toute la console s'il n'y en a pas", "Copy the selection, or the whole console when there is none")
            };
            if widgets::icon_button(ui, icons::COPY, t!("Copier", "Copy"), copy_hint, !empty).clicked() {
                self.copy(ctx, i);
            }
            if widgets::icon_button(ui, icons::UPLOAD, t!("Exporter la console", "Export the console"), t!("Exporter la console dans un fichier", "Export the console to a file"), !console_empty).clicked() {
                self.save_console(i, now);
            }
            let journal = self.sessions[i].journal.is_some();
            let (name, hint) = if journal {
                (t!("Arrêter le journal", "Stop the journal"), t!("Arrêter le journal en cours", "Stop the journal running"))
            } else {
                (t!("Démarrer le journal", "Start the journal"), t!("Écrire tout ce qui arrive dans un fichier, au fil de l'eau", "Write everything that comes to a file, as it goes"))
            };
            let response = widgets::icon_button(ui, icons::FILE_TEXT, name, hint, true);
            if journal {
                widgets::badge(ui, &response, Palette::of(dark).tone(Tone::Bad, dark));
            }
            if response.clicked() {
                if journal {
                    self.stop_journal(i, now);
                } else {
                    self.start_journal(i, now);
                }
            }
            if widgets::icon_button(ui, icons::FILE_INPUT, t!("Envoyer un fichier", "Send a file"), t!("Envoyer un fichier à l'appareil, tel quel (16 Mo au plus)", "Send a file to the device, as it is (16 MB at most)"), open).clicked() {
                self.send_file(i, now);
            }
            let clear_hint = if in_terminal {
                t!("Effacer l'écran du terminal ; la console en mode ligne et le journal ne sont pas touchés", "Clear the terminal's screen; the line console and the journal are not touched")
            } else {
                t!("Effacer la console ; le journal n'est pas touché", "Clear the console; the journal is not touched")
            };
            if widgets::icon_button(ui, icons::ERASER, t!("Effacer", "Clear"), clear_hint, !empty).clicked() {
                let session = &mut self.sessions[i];
                if let Some(terminal) = &mut session.terminal {
                    terminal.clear();
                    session.terminal_view.clear_selection();
                } else {
                    session.console.clear();
                    session.view.clear_selection();
                }
            }
            // The curves on: what the middle of the window shows. On a window too low for the two,
            // « Les deux » is closed, and the one shown is chosen (WCAG 1.4.4).
            if plot_on {
                let session = &mut self.sessions[i];
                let room = session.room_for_both;
                let mut shown = match session.display {
                    Display::Both if !room => Display::Console,
                    display => display,
                };
                let before = shown;
                let console = if in_terminal { t!("Terminal", "Terminal") } else { t!("Console", "Console") };
                widgets::at_right(ui, egui::Id::new("display-choice"), |ui| {
                    ui.label(t!("Affichage", "Display"));
                    widgets::segmented_where(
                        ui,
                        &mut shown,
                        &[(Display::Console, console), (Display::Curves, t!("Courbes", "Curves")), (Display::Both, t!("Les deux", "Both"))],
                        t!("Ce que montre le milieu de la fenêtre : la console, les courbes, ou les deux l'une au-dessus de l'autre", "What the middle of the window shows: the console, the curves, or the two one above the other"),
                        |option| option != Display::Both || room,
                        t!("Trop bas pour les deux : agrandir la fenêtre", "Too low for the two: enlarge the window"),
                    );
                });
                if shown != before {
                    session.display = shown;
                }
            }
        });
    }
}
