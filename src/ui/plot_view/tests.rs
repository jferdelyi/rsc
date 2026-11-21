//! The tests of the curves: the vertical marks, the zoom, the scale, the numbers, the values
//! written short, and the time of a frame, measured by hand.

use std::time::{Duration, Instant};

use egui_plot::{GridInput, PlotMemory};

use super::*;
use super::charts::{padded, within};
use super::marks::{GRID_LEAST, MARK_GAP, MARK_HIDDEN, mark_of, number, vertical_label, vertical_marks, vertical_steps};
use crate::console::{Console, at};

/// At any scale, the vertical step is 1, 2 or 5 times a power of ten, its marks stand
/// `MARK_GAP` points apart at least, and no smaller such step would do; on the chart seen
/// without marks (1014 to 1018, 13.6 points a unit), marks at 1014, 1016 and 1018.
#[test]
fn the_vertical_step_is_1_2_or_5_and_keeps_its_marks_apart() {
    let gap = f64::from(MARK_GAP);
    for exponent in -6..=6 {
        for per_point in [1.0, 1.3, 2.0, 2.4, 4.17, 5.0, 7.7, 9.99].map(|times| times * 10f64.powi(exponent)) {
            let (step, fine) = vertical_steps(per_point);
            let times = step / 10f64.powf((step.log10() + 1e-9).floor());
            assert!([1.0, 2.0, 5.0].iter().any(|known| (times - known).abs() < 1e-6), "a step of {step} at {per_point} a point");
            assert!(step / per_point >= gap * (1.0 - 1e-6), "a step of {step} at {per_point} a point: marks {} points apart", step / per_point);
            let smaller = step / if (times - 5.0).abs() < 1e-6 { 2.5 } else { 2.0 };
            assert!(smaller / per_point < gap, "a step of {step} at {per_point} a point, where {smaller} would do");
            let every = step / fine;
            assert!((every - every.round()).abs() < 1e-6 && fine / per_point < f64::from(MARK_HIDDEN), "finer lines of {fine} under a step of {step}: no mark between two marks");
        }
    }
    let marked = |bounds: (f64, f64), per_point: f64| {
        let (marks, step) = vertical_marks(&GridInput { bounds, base_step_size: per_point * f64::from(GRID_LEAST) });
        marks.iter().filter(|mark| mark.step_size / per_point >= gap).map(|mark| vertical_label(mark.value, step)).collect::<Vec<String>>()
    };
    assert_eq!(marked((1013.8, 1018.2), 1.0 / 13.6), ["1014", "1016", "1018"]);
    assert_eq!(marked((0.0, 1.0), 1.0 / 50.0), ["0.0", "0.5", "1.0"], "a step of 0.5: one decimal on every mark");
}

/// A frame of the curves on a panel of 1100 by 400, `events` given to it.
fn frame(ctx: &egui::Context, plot: &Plot, view: &mut View, paused: bool, events: Vec<egui::Event>) {
    drawn(ctx, plot, view, Options { paused, x: XAxis::Time, below: &[] }, events);
}

fn drawn(ctx: &egui::Context, plot: &Plot, view: &mut View, options: Options<'_>, events: Vec<egui::Event>) -> Asked {
    let input = egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1100.0, 400.0))), events, ..Default::default() };
    let mut asked = Asked::default();
    let mut output = ctx.run_ui(input, |ui| {
        asked = show(ui, plot, view, options, egui::Id::new("curves"));
    });
    output.textures_delta.clear();
    asked
}

/// The places at the charts' left and right edges at the last frame.
fn shown_x(view: &View) -> (f64, f64) {
    let (_, from, to) = view.shown.expect("shown");
    (from, to)
}

/// The values at the bottom and the top of the chart above at the last frame.
fn shown_y(view: &View) -> (f64, f64) {
    view.shown_y[0].expect("shown")
}

/// Ten seconds of a curve between -1 and 1, a point every 10 ms.
fn waves() -> Plot {
    let mut console = Console::new(1001);
    let start = at(0, 0);
    for n in 0..1000 {
        console.receive(format!("a={:.3}\n", (f64::from(n) / 50.0).sin()).as_bytes(), start + chrono::TimeDelta::milliseconds(i64::from(n) * 10));
    }
    let mut plot = Plot::new(1000);
    for line in console.lines() {
        plot.read(line, "");
    }
    plot
}

/// On a chart against time, the zoom acts on the time only; the vertical axes keep
/// the scale of the whole period chosen, so that nothing jumps nor is cut.
#[test]
fn paused_a_zoom_acts_on_the_time_only() {
    let plot = waves();
    let ctx = egui::Context::default();
    theme::install(&ctx);
    let mut view = View { span: Span::All, ..View::default() };
    frame(&ctx, &plot, &mut view, true, vec![]);
    frame(&ctx, &plot, &mut view, true, vec![egui::Event::PointerMoved(egui::pos2(550.0, 220.0))]);
    let (before_x, before_y) = (shown_x(&view), shown_y(&view));
    frame(&ctx, &plot, &mut view, true, vec![egui::Event::Zoom(2.0)]);
    frame(&ctx, &plot, &mut view, true, vec![]);
    let (after_x, after_y) = (shown_x(&view), shown_y(&view));
    assert!(after_x.1 - after_x.0 < (before_x.1 - before_x.0) * 0.75, "the time is zoomed: {before_x:?} then {after_x:?}");
    assert_eq!(after_y, before_y, "the vertical axis keeps its scale");
}

/// Measures of three curves of very different scales, `count` lines 100 ms apart.
fn measures(count: u32) -> Plot {
    let mut console = Console::new(count as usize + 1);
    let start = at(0, 0);
    for n in 0..count {
        let line = format!("temp={:.2} pression={:.1} compteur={n}\n", 21.0 + f64::from(n) / 100.0, 1018.0 - f64::from(n) / 50.0);
        console.receive(line.as_bytes(), start + chrono::TimeDelta::milliseconds(i64::from(n) * 100));
    }
    let mut plot = Plot::new(10_000);
    for line in console.lines() {
        plot.read(line, "");
    }
    plot
}

#[test]
fn the_scale_covers_the_whole_period_with_a_margin() {
    let plot = measures(101);
    let ctx = egui::Context::default();
    theme::install(&ctx);
    let mut view = View { span: Span::All, ..View::default() };
    let below = ["pression".to_owned(), "compteur".to_owned()];
    drawn(&ctx, &plot, &mut view, Options { paused: false, x: XAxis::Time, below: &below }, vec![]);
    assert_eq!(view.charts, 2);
    // temp from 21 to 22: 5 % of 1 each side.
    let (low, high) = view.shown_y[0].expect("above");
    assert!((low - 20.95).abs() < 1e-9 && (high - 22.05).abs() < 1e-9, "above: {low} to {high}");
    // pression from 1016 to 1018, compteur from 0 to 100: the chart below goes from 0 to 1018.
    let (low, high) = view.shown_y[1].expect("below");
    assert!((low - -50.9).abs() < 1e-9 && (high - 1068.9).abs() < 1e-9, "below: {low} to {high}");
    assert_eq!(shown_x(&view), (0.0, 10.0), "every measure kept");
}

#[test]
fn a_zoom_in_one_chart_is_followed_by_the_other() {
    let plot = measures(101);
    let ctx = egui::Context::default();
    theme::install(&ctx);
    let mut view = View { span: Span::All, ..View::default() };
    let below = ["pression".to_owned()];
    let options = Options { paused: true, x: XAxis::Time, below: &below };
    drawn(&ctx, &plot, &mut view, options, vec![]);
    // The pointer over the chart below, in the panel's lower half.
    drawn(&ctx, &plot, &mut view, options, vec![egui::Event::PointerMoved(egui::pos2(550.0, 330.0))]);
    drawn(&ctx, &plot, &mut view, options, vec![egui::Event::Zoom(2.0)]);
    drawn(&ctx, &plot, &mut view, options, vec![]);
    let time_of = |slot: usize| {
        let memory = PlotMemory::load(&ctx, egui::Id::new("curves").with(("chart", slot))).expect("drawn");
        (memory.bounds().min()[0], memory.bounds().max()[0])
    };
    let (above, below) = (time_of(0), time_of(1));
    assert!(above.1 - above.0 < 7.5, "zoomed from 10 s: {above:?}");
    assert_eq!(above, below, "the same time in both charts");
}

#[test]
fn along_the_numbers_the_charts_show_the_measures_by_their_number() {
    let plot = measures(250);
    let ctx = egui::Context::default();
    theme::install(&ctx);
    let mut view = View { span: Span::Short, ..View::default() };
    drawn(&ctx, &plot, &mut view, Options { paused: false, x: XAxis::Number, below: &[] }, vec![]);
    assert_eq!((view.charts, shown_x(&view)), (1, (150.0, 250.0)), "the last 100 measures");
    assert_eq!(Span::Short.label(XAxis::Number), "100 mesures");
    assert_eq!((mark_of(XAxis::Number, 200.0), mark_of(XAxis::Number, 200.5), mark_of(XAxis::Time, 12.5)), ("200".into(), String::new(), "12.5 s".into()));
}

#[test]
fn values_are_written_short() {
    assert_eq!([number(21.0), number(21.48), number(1_017.123_456), number(-0.000_1), number(0.5)], ["21", "21.48", "1017.123", "0", "0.5"]);
    assert_eq!(within((2.0, 4.0), (0.0, 10.0)), (2.0, 4.0));
    assert_eq!(within((-1.0, 1.0), (0.0, 10.0)), (0.0, 2.0), "moved into the period");
    assert_eq!(within((9.0, 12.0), (0.0, 10.0)), (7.0, 10.0));
    assert_eq!(within((-5.0, 20.0), (0.0, 10.0)), (0.0, 10.0), "no wider than the period");
    assert_eq!(padded((21.5, 21.5)), (20.425, 22.575), "a flat curve");
}

/// The time of a frame of the curves, five curves of 100 000 points, all shown: a measure,
/// run by hand in release,
/// `cargo test --release --lib frame_time_of_the_curves -- --ignored --nocapture`.
#[test]
#[ignore = "a measure, in release"]
fn frame_time_of_the_curves() {
    const POINTS: u32 = 100_000;
    let mut console = Console::new(POINTS as usize + 1);
    let start = at(0, 0);
    for n in 0..POINTS {
        let x = f64::from(n) / 100.0;
        let line = format!("a={:.3} b={:.3} c={:.3} d={:.3} e={:.3}\n", x.sin(), x.cos(), (x / 3.0).sin() * 2.0, x % 7.0, -x.sin());
        console.receive(line.as_bytes(), start + chrono::TimeDelta::milliseconds(i64::from(n) * 10));
    }
    let mut plot = Plot::new(POINTS as usize);
    for line in console.lines() {
        plot.read(line, "");
    }
    assert_eq!(plot.curves().len(), 5);
    let ctx = egui::Context::default();
    theme::install(&ctx);
    for (span, name) in [(Span::All, "tout"), (Span::Medium, "1 min")] {
        let mut view = View { span, ..View::default() };
        let mut frames: Vec<Duration> = Vec::new();
        // A first frame to warm up, left out; then 30.
        for frame in 0..31 {
            let input = egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1100.0, 400.0))), ..Default::default() };
            let began = Instant::now();
            let mut output = ctx.run_ui(input, |ui| {
                show(ui, &plot, &mut view, Options { paused: false, x: XAxis::Time, below: &[] }, egui::Id::new("measure"));
            });
            let _ = ctx.tessellate(std::mem::take(&mut output.shapes), output.pixels_per_point);
            output.textures_delta.clear();
            if frame > 0 {
                frames.push(began.elapsed());
            }
        }
        frames.sort();
        let ms = |d: Duration| d.as_secs_f64() * 1000.0;
        println!(
            "{name}: median {:.1} ms, quartiles {:.1} to {:.1} ms, min {:.1} ms, max {:.1} ms ({} frames)",
            ms(frames[frames.len() / 2]),
            ms(frames[frames.len() / 4]),
            ms(frames[frames.len() * 3 / 4]),
            ms(frames[0]),
            ms(frames[frames.len() - 1]),
            frames.len()
        );
    }
}
