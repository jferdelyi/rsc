//! The console under the mouse and the keyboard: the selection, the clicks, the wheel, the
//! scroll bar, Page Up and Page Down, Home and End.

use eframe::egui;

use super::{Asked, ConsoleView};
use super::scene::Scene;
use crate::console::Position;

impl ConsoleView {
    pub(super) fn interact(&mut self, ui: &egui::Ui, response: &egui::Response, scene: &Scene, page: f32, asked: &mut Asked) {
        let (console, shown) = (scene.console, scene.look.shown);
        if response.drag_started() {
            response.request_focus();
            let origin = ui.ctx().input(|input| input.pointer.press_origin()).or(response.interact_pointer_pos());
            self.selection = origin.map(|origin| {
                let at = scene.place(origin, true);
                (at, at)
            });
        }
        if response.dragged()
            && let (Some(pointer), Some((anchor, _))) = (response.interact_pointer_pos(), self.selection)
        {
            self.selection = Some((anchor, scene.place(pointer, true)));
            // Past an edge, the console follows the pointer.
            let clip = ui.clip_rect();
            let past = if pointer.y < clip.top() { clip.top() - pointer.y } else { -(pointer.y - clip.bottom()).max(0.0) };
            if past != 0.0 {
                ui.scroll_with_delta(egui::vec2(0.0, past.clamp(-40.0, 40.0)));
                ui.ctx().request_repaint();
            }
        }
        if response.clicked() {
            response.request_focus();
            self.selection = None;
        }
        if let Some(pointer) = response.interact_pointer_pos() {
            let at = scene.place(pointer, false);
            if response.triple_clicked() {
                let width = console.line(at.line).map_or(0, |line| line.width(shown));
                self.selection = Some((Position { line: at.line, col: 0 }, Position { line: at.line, col: width }));
            } else if response.double_clicked() {
                self.selection = console.word_at(at, shown);
            }
        }
        if response.has_focus() {
            let (copy, all, typed) = ui.ctx().input(|input| {
                let copy = input.events.iter().any(|event| matches!(event, egui::Event::Copy));
                let all = input.modifiers.command && input.key_pressed(egui::Key::A);
                let typed: String = input.events.iter().filter_map(|event| if let egui::Event::Text(text) = event { Some(text.as_str()) } else { None }).collect();
                (copy, all, typed)
            });
            if all {
                self.select_all(console, shown);
            }
            if copy {
                asked.copy = self.selected_text(console, shown);
            }
            asked.typed = typed;
            self.keys(ui, page);
        }
        response.context_menu(|ui| {
            let selected = self.selected_text(console, shown);
            if ui.add_enabled(selected.is_some(), egui::Button::new(t!("Copier", "Copy"))).clicked() {
                asked.copy = selected;
                ui.close();
            }
            if ui.button(t!("Tout sélectionner", "Select all")).clicked() {
                self.select_all(console, shown);
                ui.close();
            }
            if ui.button(t!("Effacer la console", "Clear the console")).clicked() {
                asked.clear = true;
                ui.close();
            }
        });
    }

    /// The console moved by the keyboard (WCAG 2.1.1), at the next frame: Page Up and Page Down by
    /// `page`, Home and End to its ends. Going up stops the following and coming back to the bottom
    /// starts it again, as with the wheel (`follow_next`).
    fn keys(&mut self, ui: &egui::Ui, page: f32) {
        let pressed = |key| ui.ctx().input(|input| input.key_pressed(key));
        let to = if pressed(egui::Key::PageUp) {
            Some((self.last_offset - page).max(0.0))
        } else if pressed(egui::Key::PageDown) {
            // Past the end, the scroll area brings it back to its last row.
            Some(self.last_offset + page)
        } else if pressed(egui::Key::Home) {
            Some(0.0)
        } else {
            None
        };
        if let Some(to) = to {
            self.scroll_to = Some(to);
            ui.ctx().request_repaint();
        }
        if pressed(egui::Key::End) {
            self.to_bottom = true;
            ui.ctx().request_repaint();
        }
    }
}
