//! The middle of the window: the console or the terminal's screen, the curves under it, the
//! separation between them, and the terminal's line of shortcuts.

use eframe::egui::{self, Align, CornerRadius, CursorIcon, Key, Layout, Modifiers, RichText, Sense, TextStyle};

use crate::console::Time;
use crate::link::Command;
use super::console_view::Look;
use super::plot_view;
use super::theme::{CONTROL_HEIGHT, Palette};
use super::widgets;
use super::{App, Display};

/// The separation between the console and the curves, dragged to share the height.
const SPLIT: f32 = 10.0;
/// The share of the height the curves take at first, and at least and at most.
pub(super) const PLOT_SHARE: f32 = 0.5;
pub(super) const PLOT_SHARES: std::ops::RangeInclusive<f32> = 0.15..=0.85;
/// What an arrow moves the separation by, at the keyboard.
const PLOT_SHARE_STEP: f32 = 0.05;/// The console's frame, above and below its rows.
const CONSOLE_FRAME: f32 = 24.0;
/// What the send line becomes in the terminal mode: where the keyboard goes, the screen's size, and
/// what Ctrl+C and Ctrl+V do, said once and for all.
pub(super) fn shortcuts_text(rows: u16, cols: u16) -> String {
    tr!(
        "Le clavier va à l'appareil · {cols}×{rows} · Ctrl+Tab : sortir · Ctrl+C : copie la sélection, sinon interrompt · Ctrl+V : colle · Ctrl+Maj+C / Ctrl+Maj+V : toujours copier / coller",
        "The keyboard goes to the device · {cols}×{rows} · Ctrl+Tab: leave · Ctrl+C: copies the selection, else interrupts · Ctrl+V: pastes · Ctrl+Shift+C / Ctrl+Shift+V: always copy / paste"
    )
}

/// The height the line of shortcuts takes at `width`, on as many rows as it needs: a control's
/// height at least, as the send line it stands for.
pub(super) fn shortcuts_height(ui: &egui::Ui, text: &str, width: f32) -> f32 {
    let body = TextStyle::Body.resolve(ui.style());
    ui.ctx().fonts_mut(|fonts| fonts.layout(text.to_owned(), body, egui::Color32::PLACEHOLDER, width).size().y).max(CONTROL_HEIGHT)
}

/// The line of shortcuts, never cut short (WCAG 1.4.4): it goes on to a second row on a narrow
/// window.
pub(super) fn shortcuts_line(ui: &mut egui::Ui, rows: u16, cols: u16) {
    let muted = Palette::of(ui.visuals().dark_mode).muted;
    let text = shortcuts_text(rows, cols);
    let height = shortcuts_height(ui, &text, ui.available_width());
    ui.allocate_ui_with_layout(egui::vec2(ui.available_width(), height), Layout::left_to_right(Align::Center), |ui| {
        ui.add(egui::Label::new(RichText::new(&text).color(muted)).wrap());
    });
}

impl App {
    /// The console, and the curves under it when they are on, the height shared by a separation
    /// that is dragged, or moved by the arrows once it has the focus; or one of the two alone, as
    /// the display asks, or as a window too low for the two leaves no other choice.
    pub(super) fn console_and_curves(&mut self, ui: &mut egui::Ui, palette: &Palette, now: Time) {
        let i = self.active();
        let profile = self.settings.profile_of(i);
        if !profile.plot {
            self.console(ui);
            return;
        }
        let total = ui.available_height();
        let width = ui.available_width();
        // Room for the two: the console with its frame and 3 rows, the separation, and the curves
        // with their highest chart whole. Without it, one alone, the console unless the curves are
        // chosen: nothing is cut nor goes out of the window, the text at 200 % for instance (WCAG
        // 1.4.4).
        let row = ui.ctx().fonts_mut(|fonts| fonts.row_height(&egui::FontId::monospace(profile.font_size)));
        let console_least = CONSOLE_FRAME + 3.0 * row;
        let curves_least = plot_view::least_view(&self.sessions[i].plot, &profile.plot_below);
        let room_for_both = total >= console_least + SPLIT + curves_least;
        self.sessions[i].room_for_both = room_for_both;
        let shown = match self.sessions[i].display {
            Display::Both if !room_for_both => Display::Console,
            display => display,
        };
        if shown == Display::Console {
            self.console(ui);
            return;
        }
        if shown == Display::Both {
            // The share asked for, within the least of each.
            let console_most = (total - SPLIT - curves_least).max(console_least);
            let console_height = (total * (1.0 - self.sessions[i].plot_share) - SPLIT / 2.0).clamp(console_least, console_most);
            ui.allocate_ui(egui::vec2(width, console_height), |ui| self.console(ui));
            let (rect, response) = ui.allocate_exact_size(egui::vec2(width, SPLIT), Sense::click_and_drag());
            if response.hovered() || response.dragged() {
                ui.ctx().set_cursor_icon(CursorIcon::ResizeVertical);
            }
            let session = &mut self.sessions[i];
            if response.dragged() && total > 0.0 {
                session.plot_share = (session.plot_share - response.drag_delta().y / total).clamp(*PLOT_SHARES.start(), *PLOT_SHARES.end());
            }
            // At the keyboard (WCAG 2.1.1): the arrows move it by 5 % of the height, and do not move
            // the focus away.
            if response.has_focus() {
                ui.memory_mut(|memory| memory.set_focus_lock_filter(response.id, egui::EventFilter { vertical_arrows: true, ..Default::default() }));
                let (up, down) = ui.input_mut(|input| (input.count_and_consume_key(Modifiers::NONE, Key::ArrowUp), input.count_and_consume_key(Modifiers::NONE, Key::ArrowDown)));
                let steps = up as f32 - down as f32;
                session.plot_share = (session.plot_share + PLOT_SHARE_STEP * steps).clamp(*PLOT_SHARES.start(), *PLOT_SHARES.end());
            }
            // Without a drag (WCAG 2.5.7): a click, or Enter at the keyboard, offers three heights.
            // Under the grip, at the middle: where the pointer clicks, and where the keyboard's focus is.
            let heights = egui::Popup::menu(&response).anchor(egui::PopupAnchor::Position(rect.center_bottom())).show(|ui| {
                ui.label(RichText::new(t!("Hauteur des courbes", "Height of the curves")).color(palette.muted));
                for (share, label) in [(0.25, t!("Un quart", "A quarter")), (0.5, t!("La moitié", "Half")), (0.75, t!("Trois quarts", "Three quarters"))] {
                    if ui.button(label).clicked() {
                        session.plot_share = share;
                    }
                }
            });
            if let Some(heights) = heights {
                widgets::name(&heights.response, t!("Menu Hauteur des courbes", "Menu Height of the curves"));
            }
            let share = f64::from((session.plot_share * 100.0).round());
            response.widget_info(|| egui::WidgetInfo::slider(true, share, t!("Séparation console et courbes", "Separation of the console and the curves")));
            let grip = egui::Rect::from_center_size(rect.center(), egui::vec2(48.0, 3.0));
            // The muted text at rest, 5.65:1 and 5.82:1 on the card (WCAG 1.4.11); the text under the pointer.
            ui.painter().rect_filled(grip, CornerRadius::same(2), if response.hovered() || response.dragged() { palette.text } else { palette.muted });
        }
        let session = &mut self.sessions[i];
        let id = egui::Id::new(("curves", i));
        let options = plot_view::Options { paused: session.plot_paused, x: profile.plot_x, below: &profile.plot_below };
        let asked = ui.allocate_ui(ui.available_size(), |ui| plot_view::show(ui, &session.plot, &mut session.plot_view, options, id)).inner;
        if asked.pause {
            session.plot_paused = !session.plot_paused;
        }
        if asked.clear {
            session.plot.clear();
        }
        // The axis and the places of the curves are the tab's profile's, as the "Curves" switch.
        if let Some(x) = asked.x {
            self.settings.edit_tab_profile(i, |edited| edited.plot_x = x);
        }
        if let Some((name, below)) = asked.below {
            self.settings.edit_tab_profile(i, |edited| {
                edited.plot_below.retain(|known| *known != name);
                if below {
                    edited.plot_below.push(name);
                }
            });
        }
        if asked.export {
            self.export_curves(i, now);
        }
    }

    fn console(&mut self, ui: &mut egui::Ui) {
        let i = self.active();
        let profile = self.settings.profile_of(i);
        let look = Look { font_size: profile.font_size, shown: self.shown(i), ansi_colors: profile.ansi_colors, minimap: profile.minimap };
        let session = &mut self.sessions[i];
        let open = session.is_open();
        if let Some(terminal) = &mut session.terminal {
            let hint = if open {
                t!("Connecté : tapez, le clavier va à l'appareil.", "Connected: type, the keyboard goes to the device.")
            } else {
                t!("Choisissez un port, puis « Connecter ».", "Choose a port, then \"Connect\".")
            };
            let asked = session.terminal_view.show(ui, terminal, profile.font_size, hint);
            if !asked.send.is_empty() && open {
                session.link.send(Command::Send(asked.send));
            }
            if let Some(text) = asked.copy {
                ui.ctx().copy_text(text);
            }
            if asked.clear {
                terminal.clear();
                session.terminal_view.clear_selection();
            }
            return;
        }
        let hint = if open {
            t!("Connecté : ce que l'appareil envoie s'affiche ici.", "Connected: what the device sends shows here.")
        } else {
            t!("Choisissez un port, puis « Connecter ».", "Choose a port, then \"Connect\".")
        };
        let asked = session.view.show(ui, &session.console, &look, hint);
        if let Some(text) = asked.copy {
            ui.ctx().copy_text(text);
        }
        if asked.clear {
            session.console.clear();
            session.view.clear_selection();
        }
        if !asked.typed.is_empty() {
            session.input.push_str(&asked.typed);
            session.focus_input = true;
        }
    }
}
