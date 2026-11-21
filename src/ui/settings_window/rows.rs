//! The rows of a section of the settings: a label with its explanation, and the controls; a card,
//! a line that says a problem.

use eframe::egui::{self, Align, CornerRadius, Layout, Margin, RichText, Stroke};

use super::{CONTROL_HEIGHT, Editor, LABEL_WIDTH, Palette, Tone, widgets};
use crate::settings::Profile;

/// The room before each row's label.
const INDENT: f32 = 10.0;

/// A row of the form: its label with its explanation, and its controls, on the profile being changed.
pub(super) fn row(ui: &mut egui::Ui, editor: &mut Editor, label: &str, info: &str, add: impl FnOnce(&mut egui::Ui, &mut Profile)) {
    ui.horizontal(|ui| {
        ui.add_space(INDENT);
        let label = label_column(ui, label, info);
        widgets::set_row_label(ui, Some(label));
        add(ui, &mut editor.profile);
        widgets::set_row_label(ui, None);
    });
}

/// The label of a row, in its column: its id, which names the row's controls.
fn label_column(ui: &mut egui::Ui, label: &str, info: &str) -> egui::Id {
    ui.allocate_ui_with_layout(egui::vec2(LABEL_WIDTH, CONTROL_HEIGHT), Layout::left_to_right(Align::Center), |ui| {
        ui.set_min_size(egui::vec2(LABEL_WIDTH, CONTROL_HEIGHT));
        widgets::label_with_info(ui, label, info).id
    })
    .inner
}

/// A row of the application's sections, which have no profile.
pub(super) fn plain_row(ui: &mut egui::Ui, label: &str, info: &str, add: impl FnOnce(&mut egui::Ui)) {
    ui.horizontal(|ui| {
        ui.add_space(INDENT);
        let label = label_column(ui, label, info);
        widgets::set_row_label(ui, Some(label));
        add(ui);
        widgets::set_row_label(ui, None);
    });
}

/// A card: a frame of its own in the section, for a profile or a device.
pub(super) fn card<R>(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    let palette = Palette::of(ui.visuals().dark_mode);
    egui::Frame::new()
        .fill(palette.card)
        .stroke(Stroke::new(1.0_f32, palette.border))
        .corner_radius(CornerRadius::same(10))
        .inner_margin(Margin::symmetric(14, 10))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            add(ui)
        })
        .inner
}

pub(super) fn problem_line(ui: &mut egui::Ui, problem: Option<&str>) {
    if let Some(problem) = problem {
        let dark = ui.visuals().dark_mode;
        ui.label(RichText::new(problem).color(Palette::of(dark).tone(Tone::Bad, dark)));
    }
}
