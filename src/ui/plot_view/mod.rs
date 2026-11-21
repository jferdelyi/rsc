//! The curves under a tab's console: the measures its device writes, against time or against their
//! number. Following the last measure while they run; paused, they stay still, to be zoomed and
//! moved along the horizontal axis. In one chart, or in two stacked charts that share the
//! horizontal axis, each with its own vertical scale.
//!
//! The vertical scale of a chart covers every value of its curves shown, over the whole period
//! chosen: a zoom acts on the horizontal axis only, so that nothing jumps nor is cut.

mod bar;
mod charts;
mod marks;
#[cfg(test)]
mod tests;

use eframe::egui::{self, CornerRadius, Margin, RichText};

use super::icons;
use super::theme;
use super::widgets;
use crate::plot::{Curve, Plot, XAxis};
use bar::bar;
use charts::charts;

/// The part of the measures the curves show, up to the last one.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Span {
    Short,
    #[default]
    Medium,
    Long,
    All,
}

impl Span {
    const ALL: [Span; 4] = [Span::Short, Span::Medium, Span::Long, Span::All];

    /// Its length on the axis: seconds, or measures. None for all of them.
    fn length(self, x: XAxis) -> Option<f64> {
        let lengths = match x {
            XAxis::Time => [10.0, 60.0, 300.0],
            XAxis::Number => [100.0, 1_000.0, 10_000.0],
        };
        match self {
            Span::Short => Some(lengths[0]),
            Span::Medium => Some(lengths[1]),
            Span::Long => Some(lengths[2]),
            Span::All => None,
        }
    }

    fn label(self, x: XAxis) -> &'static str {
        match (self, x) {
            (Span::Short, XAxis::Time) => "10 s",
            (Span::Medium, XAxis::Time) => "1 min",
            (Span::Long, XAxis::Time) => "5 min",
            (Span::Short, XAxis::Number) => t!("100 mesures", "100 measures"),
            (Span::Medium, XAxis::Number) => t!("1 000 mesures", "1,000 measures"),
            (Span::Long, XAxis::Number) => t!("10 000 mesures", "10,000 measures"),
            (Span::All, _) => t!("Tout", "All"),
        }
    }
}

/// How a tab's curves are looked at: the part shown, and what the charts showed at the last frame.
#[derive(Default)]
pub struct View {
    pub span: Span,
    /// The axis, and the places at the charts' left and right edges: what a zoom or a move shows,
    /// paused.
    shown: Option<(XAxis, f64, f64)>,
    /// The values at the bottom and the top of each chart: the one above, the one below.
    pub(super) shown_y: [Option<(f64, f64)>; 2],
    /// The charts drawn: one or two.
    pub(super) charts: usize,
}

/// What the tab's profile and its state say of the curves.
#[derive(Clone, Copy)]
pub struct Options<'a> {
    pub paused: bool,
    pub x: XAxis,
    /// The names of the curves drawn in the chart below.
    pub below: &'a [String],
}

/// What the user asked of the curves in a frame.
#[derive(Default)]
pub struct Asked {
    /// Pause, or resume.
    pub pause: bool,
    pub clear: bool,
    pub export: bool,
    /// The horizontal axis chosen.
    pub x: Option<XAxis>,
    /// A curve moved: its name, and whether it goes to the chart below.
    pub below: Option<(String, bool)>,
}

/// The curves of a chart, each with its place among all the curves: its colour.
type Group<'a> = Vec<(usize, &'a Curve)>;

/// The space between the two charts.
const GAP: f32 = 6.0;
/// The least height of a chart.
const LEAST_HEIGHT: f32 = 60.0;
/// The room a curve takes in a chart's legend, and the legend's margins.
const LEGEND_ROW: f32 = 24.0;
const LEGEND_ROOM: f32 = 16.0;
/// The height of the marks of the horizontal axis, under the lowest chart's area.
const X_AXIS: f32 = 20.0;
/// What the text of those marks takes below the chart, at the end of the curves' area.
const MARKS: f32 = 8.0;
/// What the text of a vertical mark takes above the higher chart, at the top of the curves' area:
/// half its height (16 points), egui_plot writing it centred on its line. Without it, the area cut
/// « 40 » in half over a counter from 0 to 41 (19/11/2026).
const MARKS_ABOVE: f32 = 8.0;

/// The least height of a chart of `curves` curves: its legend holds in it.
fn least_height(curves: usize) -> f32 {
    LEAST_HEIGHT.max(LEGEND_ROW * curves as f32 + LEGEND_ROOM)
}

/// The curves' frame, above and below them, and their bar with the space under it.
const FRAME: f32 = 20.0;
const BAR: f32 = theme::CONTROL_HEIGHT + 8.0;

/// The height the curves take to show their highest chart whole, with their frame and their bar:
/// below it, the window shows them alone, or the console alone (WCAG 1.4.4).
pub fn least_view(plot: &Plot, below: &[String]) -> f32 {
    let lower = plot.curves().iter().filter(|curve| below.contains(&curve.name)).count();
    let groups: Vec<usize> = [plot.curves().len() - lower, lower].into_iter().filter(|&curves| curves > 0).collect();
    let tallest = groups.iter().enumerate().map(|(slot, &curves)| least_height(curves) + if slot + 1 == groups.len() { X_AXIS } else { 0.0 }).fold(0.0, f32::max);
    FRAME + BAR + MARKS_ABOVE + tallest + MARKS
}

/// Draws the curves in what is left of `ui`.
pub fn show(ui: &mut egui::Ui, plot: &Plot, view: &mut View, options: Options<'_>, id: egui::Id) -> Asked {
    let mut asked = Asked::default();
    egui::Frame::new().fill(theme::CONSOLE).corner_radius(CornerRadius::same(10)).inner_margin(Margin::symmetric(12, 10)).show(ui, |ui| {
        // Dark, as the console, whatever the window's theme.
        ui.style_mut().visuals = ui.ctx().style_of(egui::Theme::Dark).visuals.clone();
        ui.set_min_size(ui.available_size());
        bar(ui, plot, view, options, id, &mut asked);
        if plot.is_empty() {
            ui.add_space(8.0);
            ui.label(
                RichText::new(t!(
                    "Aucune mesure pour l'instant. Une ligne faite seulement de valeurs devient des points : « 21.5 40 » ou « temp=21.5 hum=40 ».",
                    "No measure yet. A line made only of values becomes points: \"21.5 40\" or \"temp=21.5 hum=40\"."
                ))
                .color(theme::CONSOLE_TIME),
            );
            return;
        }
        // The charts share the room below the bar, each at its least height at least: when the
        // window is too low for them (the text at 200 %), they scroll (WCAG 1.4.4), by the wheel or
        // the bar; with a mouse, a drag moves the curves, not the area (egui's default).
        // Within the room, however small: egui's areas keep 64 points by default, which would push
        // the send line out. The charts leave MARKS under them, at the end of the area: the text of
        // the axis' marks goes a little below the chart, and the area would cut it; MARKS_ABOVE
        // over them, for the text of the highest vertical mark.
        let room = ui.available_height().max(0.0);
        egui::ScrollArea::vertical().id_salt(id.with("scroll")).auto_shrink([false, false]).min_scrolled_height(0.0).max_height(room).show(ui, |ui| {
            ui.add_space(MARKS_ABOVE);
            charts(ui, plot, view, options, id, (room - MARKS_ABOVE - MARKS).max(0.0));
            ui.add_space(MARKS);
        });
    });
    asked
}
