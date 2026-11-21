//! The bar of the curves: their title, the horizontal axis, the part shown, the pause, the places
//! of the curves, the export and the clearing.

use eframe::egui::{self, CornerRadius, RichText, Sense};

use super::{Asked, Options, Span, View, icons, theme, widgets};
use super::charts::{period, within};
use crate::plot::{Plot, XAxis};

/// The bar of the curves: their title and axis on the left, the part shown and the actions on the
/// right.
pub(super) fn bar(ui: &mut egui::Ui, plot: &Plot, view: &mut View, options: Options<'_>, id: egui::Id, asked: &mut Asked) {
    ui.horizontal(|ui| {
        // The title, painted once the row has its height: a label would be centred on the row as it
        // is when the label comes, before the taller controls after it (3 px too high, 18/11/2026).
        let title = egui::WidgetText::from(RichText::new(t!("Courbes", "Curves")).strong().color(theme::CONSOLE_TEXT)).into_galley(
            ui,
            Some(egui::TextWrapMode::Extend),
            f32::INFINITY,
            egui::TextStyle::Body,
        );
        let (title_rect, _) = ui.allocate_exact_size(title.size(), Sense::hover());
        let mut x = options.x;
        widgets::segmented(
            ui,
            &mut x,
            &[(XAxis::Time, t!("Temps", "Time")), (XAxis::Number, t!("N°", "No."))],
            t!(
                "L'axe horizontal : le temps, ou le numéro de la mesure (1 pour la première ligne de mesures, 2 pour la suivante…)",
                "The horizontal axis: the time, or the number of the measure (1 for the first line of measures, 2 for the next…)"
            ),
        );
        if x != options.x {
            asked.x = Some(x);
        }
        if options.paused {
            ui.label(RichText::new(t!("en pause", "paused")).color(theme::CONSOLE_TIME));
            // Without a drag nor the wheel (WCAG 2.5.7, and 2.1.1 at the keyboard): the view moves
            // and zooms by buttons, within the part chosen.
            let whole = period(plot, view.span, options.x);
            let shown = match view.shown {
                Some((axis, from, to)) if axis == options.x => within((from, to), whole),
                _ => whole,
            };
            let width = shown.1 - shown.0;
            let moves = [
                (icons::CHEVRON_LEFT, t!("Plus tôt", "Earlier"), t!("Plus tôt : la vue recule d'une demi-largeur", "Earlier: the view goes back by half its width"), (shown.0 - width / 2.0, shown.1 - width / 2.0)),
                (icons::CHEVRON_RIGHT, t!("Plus tard", "Later"), t!("Plus tard : la vue avance d'une demi-largeur", "Later: the view goes on by half its width"), (shown.0 + width / 2.0, shown.1 + width / 2.0)),
                (icons::ZOOM_IN, t!("Rapprocher", "Closer"), t!("Rapprocher : la vue montre moitié moins, au même milieu", "Closer: the view shows half as much, around the same middle"), (shown.0 + width / 4.0, shown.1 - width / 4.0)),
                (icons::ZOOM_OUT, t!("Éloigner", "Farther"), t!("Éloigner : la vue montre deux fois plus, dans la part choisie", "Farther: the view shows twice as much, within the part chosen"), (shown.0 - width / 2.0, shown.1 + width / 2.0)),
            ];
            for (icon, label, hint, next) in moves {
                if widgets::icon_button(ui, icon, label, hint, true).clicked() {
                    let (from, to) = within(next, whole);
                    view.shown = Some((options.x, from, to));
                }
            }
        }
        // Set against the right edge, added in the order they are read (WCAG 2.4.3).
        widgets::at_right(ui, id.with("actions"), |ui| {
            let list = egui::ComboBox::from_id_salt(id.with("span"))
                .width(120.0)
                .selected_text(view.span.label(options.x))
                .show_ui(ui, |ui| {
                    for option in Span::ALL {
                        ui.selectable_value(&mut view.span, option, option.label(options.x));
                    }
                })
                .response;
            widgets::name(&list, t!("Part des mesures montrée", "Part of the measures shown"));
            list.on_hover_text(t!("La part des mesures montrée, jusqu'à la dernière", "The part of the measures shown, up to the last one"));
            let places = widgets::icon_button(
                ui,
                icons::ROWS,
                t!("Haut ou bas", "Above or below"),
                t!("Chaque courbe dans le graphique du haut ou du bas, chacun avec son échelle", "Each curve in the chart above or below, each with its own scale"),
                !plot.is_empty(),
            );
            if let Some(menu) = egui::Popup::menu(&places).close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside).show(|ui| places_menu(ui, plot, options, asked)) {
                widgets::name(&menu.response, t!("Menu Haut ou bas", "Menu Above or below"));
            }
            let (icon, label, hint) = if options.paused {
                (icons::PLAY, t!("Reprendre", "Resume"), t!("Reprendre : les nouvelles mesures s'ajoutent de nouveau", "Resume: new measures are added again"))
            } else {
                (
                    icons::PAUSE,
                    t!("Pause", "Pause"),
                    t!("Pause : plus de mesure ajoutée ; les courbes se zooment et se déplacent le long de l'axe horizontal", "Pause: no measure added; the curves can be zoomed and moved along the horizontal axis"),
                )
            };
            if widgets::icon_button(ui, icon, label, hint, true).clicked() {
                asked.pause = true;
            }
            let clear = t!("Effacer les courbes ; la console n'est pas touchée", "Clear the curves; the console is not touched");
            if widgets::icon_button(ui, icons::ERASER, t!("Effacer les courbes", "Clear the curves"), clear, !plot.is_empty()).clicked() {
                asked.clear = true;
            }
            let export = t!("Exporter les mesures gardées en CSV, pour un tableur", "Export the measures kept as CSV, for a spreadsheet");
            if widgets::icon_button(ui, icons::UPLOAD, t!("Exporter en CSV", "Export as CSV"), export, !plot.is_empty()).clicked() {
                asked.export = true;
            }
        });
        let top = ui.min_rect().center().y - title.size().y / 2.0;
        ui.painter().galley(egui::pos2(title_rect.left(), top), title, theme::CONSOLE_TEXT);
    });
}

/// Each curve, its colour, its name, and the chart it is drawn in.
fn places_menu(ui: &mut egui::Ui, plot: &Plot, options: Options<'_>, asked: &mut Asked) {
    for (index, curve) in plot.curves().iter().enumerate() {
        ui.horizontal(|ui| {
            // On a chip of the console's background, as in the charts: the curves' colours stand at
            // 3:1 at least against it, not against the light theme's menu (WCAG 1.4.11).
            let (chip, _) = ui.allocate_exact_size(egui::vec2(18.0, 18.0), Sense::hover());
            ui.painter().rect_filled(chip, CornerRadius::same(5), theme::CONSOLE);
            ui.painter().rect_filled(egui::Rect::from_center_size(chip.center(), egui::vec2(10.0, 10.0)), CornerRadius::same(3), theme::curve(index));
            let was_below = options.below.contains(&curve.name);
            let mut below = was_below;
            widgets::segmented(
                ui,
                &mut below,
                &[(false, t!("Haut", "Above")), (true, t!("Bas", "Below"))],
                &tr!("Le graphique de « {name} »", "The chart of \"{name}\"", name = curve.name),
            );
            ui.label(&curve.name);
            if below != was_below {
                asked.below = Some((curve.name.clone(), below));
            }
        });
    }
}
