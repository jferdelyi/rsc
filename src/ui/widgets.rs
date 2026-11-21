//! The controls of the window beyond egui's: a button that is an icon, a switch, a choice in
//! segments, a label with its explanation. Each says what it does in a tooltip, and to a screen
//! reader by its name.

use eframe::egui::accesskit::{self, Role};
use eframe::egui::{self, Color32, CornerRadius, Response, Sense, Stroke, StrokeKind, WidgetInfo, WidgetType};

use super::icons::{self, Icon};
use super::theme::{self, CONTROL_HEIGHT, Palette};

/// The size of an icon in a button.
const ICON_SIZE: f32 = 17.0;

/// A button showing an icon: what it does is `name`, for screen readers and tests, and `tooltip`
/// on hover, whether it is enabled or not.
pub fn icon_button(ui: &mut egui::Ui, icon: Icon, name: &str, tooltip: &str, enabled: bool) -> Response {
    let palette = Palette::of(ui.visuals().dark_mode);
    let button = egui::Button::new(icon.text(ICON_SIZE).color(palette.on_secondary)).corner_radius(CornerRadius::same(8)).min_size(egui::vec2(CONTROL_HEIGHT + 6.0, CONTROL_HEIGHT));
    named(theme::filled_button(ui, button, palette.secondary, enabled), WidgetType::Button, name).on_hover_text(tooltip).on_disabled_hover_text(tooltip)
}

/// A small icon without a frame, beside a text: a tab's cross, the "+" of the tabs.
pub fn bare_icon_button(ui: &mut egui::Ui, icon: Icon, name: &str, tooltip: &str) -> Response {
    let color = Palette::of(ui.visuals().dark_mode).muted;
    named(ui.add(egui::Button::new(icon.text(14.0).color(color)).frame(false)), WidgetType::Button, name).on_hover_text(tooltip)
}

/// A mark on the corner of a button: a journal that runs.
pub fn badge(ui: &egui::Ui, response: &Response, color: Color32) {
    let center = response.rect.right_top() + egui::vec2(-6.0, 6.0);
    ui.painter().circle_filled(center, 4.0, color);
}

/// A switch, then its label, the two as one control: a click on either turns it.
pub fn switch(ui: &mut egui::Ui, on: &mut bool, label: &str, tooltip: &str) -> Response {
    let palette = Palette::of(ui.visuals().dark_mode);
    let track = egui::vec2(30.0, 17.0);
    let gap = 8.0;
    let galley = ui.painter().layout_no_wrap(label.to_owned(), egui::TextStyle::Body.resolve(ui.style()), palette.text);
    let size = egui::vec2(track.x + gap + galley.size().x, track.y.max(galley.size().y).max(24.0));
    let (rect, mut response) = ui.allocate_exact_size(size, Sense::click());
    if response.clicked() {
        *on = !*on;
        response.mark_changed();
    }
    let value = *on;
    let enabled = ui.is_enabled();
    response.widget_info(|| WidgetInfo::selected(WidgetType::Checkbox, enabled, value, label));
    let how_far = ui.ctx().animate_bool_responsive(response.id, value);
    let track_rect = egui::Rect::from_min_size(egui::pos2(rect.left(), rect.center().y - track.y / 2.0), track);
    let fill = lerp_color(palette.switch_off, palette.accent, how_far);
    ui.painter().rect_filled(track_rect, CornerRadius::same(9), if enabled { fill } else { fill.gamma_multiply(0.5) });
    let radius = track.y / 2.0 - 2.5;
    let x = egui::lerp((track_rect.left() + radius + 2.5)..=(track_rect.right() - radius - 2.5), how_far);
    ui.painter().circle_filled(egui::pos2(x, track_rect.center().y), radius, Color32::WHITE);
    if response.has_focus() {
        ui.painter().rect_stroke(track_rect.expand(2.0), CornerRadius::same(10), Stroke::new(1.0_f32, palette.accent), StrokeKind::Outside);
    }
    let text_color = if enabled { palette.text } else { palette.muted };
    ui.painter().galley(egui::pos2(track_rect.right() + gap, rect.center().y - galley.size().y / 2.0), galley, text_color);
    response.on_hover_text(tooltip).on_disabled_hover_text(tooltip)
}

/// A choice among a few, side by side in one frame, the one chosen filled.
pub fn segmented<T: PartialEq + Copy>(ui: &mut egui::Ui, value: &mut T, options: &[(T, &str)], tooltip: &str) -> Response {
    segmented_where(ui, value, options, tooltip, |_| true, "")
}

/// A choice in segments, some of whose options may be closed: `open` says which, `closed` why, in
/// the closed option's tooltip.
pub fn segmented_where<T: PartialEq + Copy>(ui: &mut egui::Ui, value: &mut T, options: &[(T, &str)], tooltip: &str, open: impl Fn(T) -> bool, closed: &str) -> Response {
    let palette = Palette::of(ui.visuals().dark_mode);
    egui::Frame::new()
        .fill(palette.field)
        .stroke(Stroke::new(1.0_f32, palette.control))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(egui::Margin::same(2))
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            ui.horizontal(|ui| {
                for (option, label) in options {
                    let chosen = *value == *option;
                    let text = egui::RichText::new(*label).color(if chosen { palette.on_primary } else { palette.text });
                    let button = egui::Button::new(text)
                        .fill(if chosen { palette.primary } else { Color32::TRANSPARENT })
                        .stroke(Stroke::NONE)
                        .corner_radius(CornerRadius::same(6))
                        .min_size(egui::vec2(44.0, CONTROL_HEIGHT - 6.0));
                    let enabled = open(*option);
                    let response = ui.add_enabled(enabled, button);
                    let label = *label;
                    response.widget_info(|| WidgetInfo::selected(WidgetType::RadioButton, enabled, chosen, label));
                    let response = if enabled { response } else { response.on_disabled_hover_text(closed) };
                    if response.clicked() {
                        *value = *option;
                    }
                }
            });
        })
        .response
        .on_hover_text(tooltip)
}

/// A form's label, its explanation in the tooltip of an info icon beside it and of the label: the
/// label, which names the control of its row for screen readers. The icon is hidden from them:
/// they would read its glyph.
pub fn label_with_info(ui: &mut egui::Ui, text: &str, info: &str) -> Response {
    let muted = Palette::of(ui.visuals().dark_mode).muted;
    ui.horizontal(|ui| {
        let label = ui.label(text).on_hover_text(info);
        hidden(&ui.label(icons::INFO.text(13.0).color(muted)).on_hover_text(info));
        label
    })
    .inner
}

/// Gives a response a name, for screen readers and for the tests that look a control up by it.
fn named(response: Response, kind: WidgetType, name: &str) -> Response {
    let enabled = response.enabled();
    response.widget_info(|| WidgetInfo::labeled(kind, enabled, name));
    response
}

/// Names an egui control for screen readers, its value and role kept: for a control that no text
/// beside it labels (WCAG 4.1.2).
pub fn name(response: &Response, name: &str) {
    response.ctx.accesskit_node_builder(response.id, |node| node.set_label(name));
}

/// What a control says of itself beyond its name, for screen readers: a tab's state, say, which
/// its mark shows on screen (WCAG 1.4.1).
pub fn describe(response: &Response, description: &str) {
    response.ctx.accesskit_node_builder(response.id, |node| node.set_description(description));
}

/// Hides a piece that only decorates from screen readers: an icon made of a glyph.
pub fn hidden(response: &Response) {
    response.ctx.accesskit_node_builder(response.id, |node| node.set_hidden());
}

/// Whether a screen reader asks for the window's tree (AccessKit on): what is made only for one is
/// made then, and costs nothing otherwise.
pub fn reading(response: &Response) -> bool {
    response.ctx.accesskit_node_builder(response.id, |_| ()).is_some()
}

/// A painted area of text for screen readers (WCAG 1.3.1, 4.1.2): its role, where it shows (the part
/// in view of what scrolls, where a screen reader frames it), and what it says when it shows no
/// text, its hint. False when no screen reader asks for the tree.
pub fn text_area(response: &Response, shown: egui::Rect, role: Role, hint: Option<&str>) -> bool {
    response
        .ctx
        .accesskit_node_builder(response.id, |node| {
            node.set_role(role);
            node.set_bounds(bounds(shown));
            if let Some(hint) = hint {
                node.set_description(hint);
            }
        })
        .is_some()
}

/// Where an element shows, for screen readers.
pub fn bounds(rect: egui::Rect) -> accesskit::Rect {
    accesskit::Rect { x0: rect.min.x.into(), y0: rect.min.y.into(), x1: rect.max.x.into(), y1: rect.max.y.into() }
}

/// The lines a painted area of text shows, for screen readers alone: each one a text element of the
/// area, with its text, what it is when that says more (« envoyé »), and where it shows. Lines are
/// `(id, rect, text, what)`; the screen reader goes through them in that order.
pub fn text_lines(ui: &mut egui::Ui, area: &Response, lines: impl IntoIterator<Item = (egui::Id, egui::Rect, String, Option<&'static str>)>) {
    // Under the area's element: the lines a Ui of their own (a container screen readers pass by).
    let reader = ui.new_child(egui::UiBuilder::new().id(area.id.with("lines")).max_rect(area.rect).accessibility_parent(area.id));
    for (id, rect, text, what) in lines {
        let line = reader.interact(rect, id, Sense::hover());
        line.widget_info(|| WidgetInfo::labeled(WidgetType::Label, true, &text));
        // An element that takes no focus gets no place from egui: it is given here.
        ui.ctx().accesskit_node_builder(id, |node| {
            node.set_bounds(bounds(rect));
            if let Some(what) = what {
                node.set_description(what);
            }
        });
    }
}

/// Where a row of the settings keeps its label's id while its controls are added.
const ROW_LABEL: &str = "rsc-row-label";

/// The label of the row being added: its controls take their name from it.
pub fn set_row_label(ui: &egui::Ui, label: Option<egui::Id>) {
    let key = egui::Id::new(ROW_LABEL);
    ui.data_mut(|data| {
        if let Some(label) = label {
            data.insert_temp(key, label);
        } else {
            data.remove::<egui::Id>(key);
        }
    });
}

/// Ties an egui control of a row of the settings to the row's label, for screen readers.
pub fn labelled(response: Response) -> Response {
    match response.ctx.data(|data| data.get_temp::<egui::Id>(egui::Id::new(ROW_LABEL))) {
        Some(label) => response.labelled_by(label),
        None => response,
    }
}

/// The width `id` took at the last frame.
pub fn measured(ui: &egui::Ui, id: egui::Id) -> Option<f32> {
    ui.data(|data| data.get_temp::<f32>(id))
}

/// Keeps the width `id` takes, for the next frame to lay out by it; a change draws once more.
pub fn measure(ui: &egui::Ui, id: egui::Id, width: f32) {
    if measured(ui, id).is_none_or(|known| (known - width).abs() > 0.5) {
        ui.data_mut(|data| data.insert_temp(id, width));
        ui.ctx().request_repaint();
    }
}

/// Controls added left to right, the group set against the right edge: Tab goes in the order they
/// are read (WCAG 2.4.3), where a right-to-left layout reverses it. Laid out by the width the
/// group took at the last frame.
pub fn at_right<R>(ui: &mut egui::Ui, id: egui::Id, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    let width = measured(ui, id).unwrap_or(0.0);
    ui.add_space((ui.available_width() - width).max(0.0));
    let group = ui.horizontal(add);
    measure(ui, id, group.response.rect.width());
    group.inner
}

fn lerp_color(from: Color32, to: Color32, how_far: f32) -> Color32 {
    let mix = |a: u8, b: u8| (f32::from(a) + (f32::from(b) - f32::from(a)) * how_far).round() as u8;
    Color32::from_rgb(mix(from.r(), to.r()), mix(from.g(), to.g()), mix(from.b(), to.b()))
}
