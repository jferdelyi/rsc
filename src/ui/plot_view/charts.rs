//! The chart or the two charts: the curves of each against the axis chosen, their scale, the name
//! at the end of each curve, and what a chart says to screen readers.

use std::cell::Cell;
use std::rc::Rc;

use eframe::egui::{self, CornerRadius};
use egui_plot::{AxisHints, Legend, Line, Plot as Chart, PlotMemory, PlotPoint, PlotPoints};

use super::{GAP, Group, Options, Span, View, X_AXIS, least_height, theme, widgets};
use super::marks::{GRID_LEAST, MARK_GAP, MARK_HIDDEN, hover, mark_of, number, vertical_label, vertical_marks};
use crate::plot::{Curve, Plot, XAxis};

/// The chart or the two charts: the curves of each, against the axis chosen, sharing `room`.
pub(super) fn charts(ui: &mut egui::Ui, plot: &Plot, view: &mut View, options: Options<'_>, id: egui::Id, room: f32) {
    let x = options.x;
    let period = period(plot, view.span, x);
    // What the charts show: the period while running; paused, what they showed last, within it.
    let shown = match view.shown {
        Some((axis, from, to)) if options.paused && axis == x => within((from, to), period),
        _ => period,
    };
    let (below, above): (Group<'_>, Group<'_>) = plot.curves().iter().enumerate().partition(|(_, curve)| options.below.contains(&curve.name));
    let groups: Vec<Group<'_>> = [above, below].into_iter().filter(|group| !group.is_empty()).collect();
    let count = groups.len();
    let share = (room - GAP * (count - 1) as f32) / count as f32;
    // Reduced to the chart's width: five curves of 100 000 points took 42 ms a frame (17/11/2026).
    let columns = ui.available_width().max(1.0) as usize;
    let mut moved = None;
    let mut reset = false;
    for (slot, group) in groups.iter().enumerate() {
        let chart_id = id.with(("chart", slot));
        // The curves hidden by a click on the legend count no more in the scale.
        let hidden = PlotMemory::load(ui.ctx(), chart_id).map(|memory| memory.hidden_items).unwrap_or_default();
        let range = group
            .iter()
            .filter(|(_, curve)| !hidden.contains(&curve_id(&curve.name)))
            .filter_map(|(_, curve)| curve.range(x, period.0, period.1))
            .reduce(|(low, high), (least, most)| (low.min(least), high.max(most)));
        let scale = range.map(padded).or(view.shown_y[slot]).unwrap_or((0.0, 1.0));
        // Only the lowest chart bears the marks of the horizontal axis, under its area.
        let lowest = slot + 1 == count;
        let height = share.max(least_height(group.len()) + if lowest { X_AXIS } else { 0.0 });
        // The step of the vertical marks: the grid finds it, the labels follow it (in that order,
        // within the frame).
        let step = Rc::new(Cell::new(1.0));
        let found = Rc::clone(&step);
        // Each curve with the points it is drawn by, reduced to the chart's width.
        let lines: Vec<(usize, &Curve, Vec<[f64; 2]>)> = group.iter().map(|(index, curve)| (*index, *curve, curve.reduced(x, shown.0, shown.1, columns))).collect();
        // The legend on the side the curves' ends leave free, where their names stand: at the top
        // left when an end is in the right half, at the top right when they all are in the left one
        // (a few measures in; seen over the legend in the real window, 19/11/2026).
        let width = (shown.1 - shown.0).abs().max(f64::MIN_POSITIVE);
        let ends_right = lines.iter().filter_map(|(_, _, points)| points.last()).any(|end| (end[0] - shown.0) / width > 0.5);
        let corner = if ends_right { egui_plot::Corner::LeftTop } else { egui_plot::Corner::RightTop };
        let chart = Chart::new(chart_id)
            .id(chart_id)
            .height(height)
            .legend(Legend::default().position(corner))
            .show_axes([lowest, true])
            // The unit on the marks: an axis title would take the height of a row.
            .x_axis_formatter(move |mark, _| mark_of(x, mark.value))
            // The vertical marks by steps of 1, 2 or 5, each in full colour (WCAG 1.4.3).
            .grid_spacing(GRID_LEAST..=300.0)
            .custom_y_axes(vec![AxisHints::new_y().label_spacing(MARK_HIDDEN..=MARK_GAP)])
            .y_grid_spacer(move |input| {
                let (marks, step) = vertical_marks(&input);
                found.set(step);
                marks
            })
            .y_axis_formatter(move |mark, _| vertical_label(mark.value, step.get()))
            .y_axis_min_width(36.0)
            .label_formatter(move |position| hover(x, position))
            .link_cursor(id.with("cursor"), [true, false])
            .allow_drag([options.paused, false])
            .allow_zoom([options.paused, false])
            .allow_scroll([options.paused, false])
            .allow_boxed_zoom(false)
            .allow_double_click_reset(false);
        let response = chart.show(ui, |chart| {
            for (index, curve, points) in &lines {
                chart.line(Line::new(curve.name.clone(), PlotPoints::new(points.clone())).id(curve_id(&curve.name)).color(theme::curve(*index)).width(1.5));
            }
            chart.set_plot_bounds_x(shown.0..=shown.1);
            chart.set_plot_bounds_y(scale.0..=scale.1);
        });
        // Each curve told by its name at the end of its line, not only by its colour (WCAG 1.4.1).
        let ends: Vec<(&str, egui::Pos2)> = lines
            .iter()
            .filter(|(_, curve, _)| !hidden.contains(&curve_id(&curve.name)))
            .filter_map(|(_, curve, points)| points.last().map(|end| (curve.name.as_str(), response.transform.position_from_point(&PlotPoint::new(end[0], end[1])))))
            .collect();
        curve_names(ui, response.response.rect, ends);
        let names: Vec<&str> = group.iter().map(|(_, curve)| curve.name.as_str()).collect();
        widgets::name(&response.response, &tr!("Courbes : {}", "Curves: {}", names.join(", ")));
        // The curves in words, their text alternative for screen readers (WCAG 1.1.1).
        if widgets::reading(&response.response) {
            let curves = group.iter().map(|(_, curve)| (*curve, hidden.contains(&curve_id(&curve.name))));
            widgets::describe(&response.response, &chart_words(curves, x, shown, options.paused));
        }
        let bounds = response.transform.bounds();
        let drawn = (bounds.min()[0], bounds.max()[0]);
        let width = (shown.1 - shown.0).abs().max(f64::MIN_POSITIVE);
        if (drawn.0 - shown.0).abs() > width * 1e-9 || (drawn.1 - shown.1).abs() > width * 1e-9 {
            moved = Some(drawn);
        }
        reset |= response.response.double_clicked();
        view.shown_y[slot] = Some((bounds.min()[1], bounds.max()[1]));
        if !lowest {
            ui.add_space(GAP);
        }
    }
    view.charts = count;
    // A double click: back to the whole period.
    let next = moved.map_or(shown, |moved| within(moved, period));
    view.shown = (!reset).then_some((x, next.0, next.1));
    if moved.is_some() || reset {
        // The other chart follows at the next frame.
        ui.ctx().request_repaint();
    }
}

/// What a chart says of its curves (with whether each is hidden by its legend) to screen readers: of
/// each, its last value and its lowest and highest in the part `shown`, or that it is hidden; then
/// whether the curves are paused. « temp : 22, de 20.75 à 22 ; hum : masquée ».
fn chart_words<'a>(curves: impl Iterator<Item = (&'a Curve, bool)>, x: XAxis, shown: (f64, f64), paused: bool) -> String {
    let mut words: Vec<String> = curves
        .map(|(curve, hidden)| {
            let name = &curve.name;
            if hidden {
                return tr!("{name} : masquée", "{name}: hidden");
            }
            match (curve.last_within(x, shown.0, shown.1), curve.range(x, shown.0, shown.1)) {
                (Some(last), Some((low, high))) => tr!("{name} : {}, de {} à {}", "{name}: {}, from {} to {}", number(last), number(low), number(high)),
                _ => tr!("{name} : aucune mesure dans la part montrée", "{name}: no measure in the part shown"),
            }
        })
        .collect();
    if paused {
        words.push(t!("en pause", "paused").to_owned());
    }
    words.join(" ; ")
}

/// Each curve's name at the end of its line, in the console's text on a chip of its background: the
/// name tells the curve, not only its colour (WCAG 1.4.1). Right of the end when there is room (the
/// curves paused, or shorter than the part shown), else above it, at its left; names that would
/// overlap go down, one under the other.
fn curve_names(ui: &egui::Ui, area: egui::Rect, mut ends: Vec<(&str, egui::Pos2)>) {
    let painter = ui.painter_at(area);
    let font = egui::FontId::proportional(12.0);
    ends.sort_by(|a, b| a.1.y.total_cmp(&b.1.y));
    let mut free = area.top() + 2.0;
    for (name, end) in ends {
        let galley = painter.layout_no_wrap(name.to_owned(), font.clone(), theme::CONSOLE_TEXT);
        let size = galley.size() + egui::vec2(8.0, 4.0);
        let (left, top) = if end.x + 6.0 + size.x <= area.right() - 2.0 { (end.x + 6.0, end.y - size.y / 2.0) } else { ((end.x - size.x - 4.0).max(area.left() + 2.0), end.y - size.y - 4.0) };
        let top = top.max(free).min(area.bottom() - size.y);
        let chip = egui::Rect::from_min_size(egui::pos2(left, top), size);
        painter.rect_filled(chip, CornerRadius::same(4), theme::CONSOLE);
        painter.galley(chip.min + egui::vec2(4.0, 2.0), galley, theme::CONSOLE_TEXT);
        free = chip.bottom() + 2.0;
    }
}

/// The period chosen: the span up to the last measure; with "All", every measure kept.
pub(super) fn period(plot: &Plot, span: Span, x: XAxis) -> (f64, f64) {
    let last = plot.last(x);
    match span.length(x) {
        Some(length) => {
            let start = (last - length).max(0.0);
            (start, start + length)
        }
        None => match plot.extent(x) {
            Some((first, end)) if end > first => (first, end),
            Some((first, end)) => (first - 0.5, end + 0.5),
            None => (0.0, 1.0),
        },
    }
}

/// `shown` moved into `period`, its width kept when it fits.
pub(super) fn within(shown: (f64, f64), period: (f64, f64)) -> (f64, f64) {
    let width = shown.1 - shown.0;
    if width >= period.1 - period.0 {
        period
    } else if shown.0 < period.0 {
        (period.0, period.0 + width)
    } else if shown.1 > period.1 {
        (period.1 - width, period.1)
    } else {
        shown
    }
}

/// The lowest and highest values with a margin of 5 % each side; a fixed one around a flat curve.
pub(super) fn padded((low, high): (f64, f64)) -> (f64, f64) {
    let margin = if high > low { (high - low) * 0.05 } else { (low.abs() * 0.05).max(1.0) };
    (low - margin, high + margin)
}

/// The identity of a curve in its chart: its legend entry, shown or hidden.
fn curve_id(name: &str) -> egui::Id {
    egui::Id::new(("curve", name))
}
