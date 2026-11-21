//! What the axes of the curves say: the vertical marks by steps of 1, 2 or 5, the marks of the
//! horizontal axis, a value written short, and what the pointer shows near a point.

use eframe::egui;
use egui_plot::{GridInput, GridMark, HoverPosition};

use crate::plot::XAxis;

/// The least distance between two marks of a chart's vertical axis, in points. egui_plot writes a
/// mark only past `MARK_HIDDEN` points from the next, and fades it up to `MARK_GAP`: at this gap,
/// each mark shows in full colour.
pub(super) const MARK_GAP: f32 = 24.0;
pub(super) const MARK_HIDDEN: f32 = 18.0;
/// The least distance between two lines of the grid, in points (egui_plot's own): it gives the
/// scale to the grid's steps.
pub(super) const GRID_LEAST: f32 = 8.0;

/// The step of a chart's vertical marks at `per_point` values a point, and the step of the finer
/// lines between them. The step is 1, 2 or 5 times a power of ten, the smallest that puts the marks
/// `MARK_GAP` points apart; the finer lines, a fifth or a quarter of it, stay too close for a mark.
/// egui_plot's own steps go by tens: a chart of 1014 to 1018 had marks of 1 at 19 points, too
/// close, and none of 10 in view (18/11/2026).
pub(super) fn vertical_steps(per_point: f64) -> (f64, f64) {
    let least = f64::from(MARK_GAP) * per_point;
    let power = 10f64.powf(least.log10().floor());
    [(1.0, 5.0), (2.0, 4.0), (5.0, 5.0), (10.0, 5.0)]
        .into_iter()
        .map(|(times, fine)| (times * power, times * power / fine))
        .find(|(step, _)| *step >= least * (1.0 - 1e-9))
        .unwrap_or((10.0 * power, 2.0 * power))
}

/// The marks and lines of a chart's vertical axis, given to egui_plot (`y_grid_spacer`): a line at
/// every fine step; a mark at every step; a bolder line at every ten steps. With them, the step of
/// the marks, which their labels follow.
pub(super) fn vertical_marks(input: &GridInput) -> (Vec<GridMark>, f64) {
    let per_point = input.base_step_size / f64::from(GRID_LEAST);
    if !per_point.is_finite() || per_point <= 0.0 {
        return (Vec::new(), 1.0);
    }
    let (step, fine) = vertical_steps(per_point);
    let every = (step / fine).round() as i64;
    let (low, high) = (input.bounds.0.min(input.bounds.1), input.bounds.0.max(input.bounds.1));
    let (first, last) = ((low / fine).ceil() as i64, (high / fine).floor() as i64);
    // Bounds that make no sense on this scale: no grid rather than a frame spent on it.
    if last - first > 10_000 {
        return (Vec::new(), step);
    }
    let marks = (first..=last)
        .map(|n| {
            let step_size = if n % (10 * every) == 0 {
                10.0 * step
            } else if n % every == 0 {
                step
            } else {
                fine
            };
            GridMark { value: n as f64 * fine, step_size }
        })
        .collect();
    (marks, step)
}

/// A vertical mark's value, with the decimals the step of the marks needs, the same for every mark
/// of the axis: 0.5 takes one (egui_plot's own rounds the decimals of a step of 5 away, and would
/// write 0).
pub(super) fn vertical_label(value: f64, step: f64) -> String {
    let decimals = (-step.log10() - 1e-9).ceil().max(0.0) as usize;
    egui::emath::format_with_decimals_in_range(value, decimals..=decimals)
}

/// A value, with three decimals at most and no zero at its end.
pub(super) fn number(value: f64) -> String {
    let text = format!("{value:.3}");
    let text = if text.contains('.') { text.trim_end_matches('0').trim_end_matches('.') } else { &text };
    if text == "-0" { "0".to_owned() } else { text.to_owned() }
}

/// A mark of the horizontal axis: "12 s", or a number of a measure, whole.
pub(super) fn mark_of(x: XAxis, value: f64) -> String {
    match x {
        XAxis::Time => format!("{} s", number(value)),
        XAxis::Number if value.fract() == 0.0 => number(value),
        XAxis::Number => String::new(),
    }
}

/// What the pointer shows near a point: the curve, its value, and its time or number.
pub(super) fn hover(x: XAxis, position: &HoverPosition<'_>) -> Option<String> {
    let HoverPosition::NearDataPoint { plot_name, position, .. } = position else {
        return None;
    };
    let place = match x {
        XAxis::Time => format!("{} s", number(position.x)),
        XAxis::Number => format!("n° {}", number(position.x.round())),
    };
    Some(format!("{plot_name}\n{}\n{place}", number(position.y)))
}
