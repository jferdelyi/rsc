//! The measures a device writes, as curves. A line made only of values, as the serial plotter of
//! Arduino reads them ("21.5 40", "temp=21.5 hum=40"), adds a point to each curve it names, at the
//! time the line came and with the line's number among the lines of measures. Any other line is
//! left out: a log line makes no curve.

use std::collections::VecDeque;
use std::ops::Range;

use serde::{Deserialize, Serialize};

use crate::console::{Kind, Line};

/// What the curves are drawn against.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum XAxis {
    /// The seconds since the first measure.
    #[default]
    Time,
    /// The number of the line of measures: 1 for the first, 2 for the next. A device that sends
    /// its measures in bursts dates them all of one time; their numbers spread them.
    Number,
}

/// The values of a line, each with the name of its curve: a bare value is named by its place among
/// the bare ones, "1", "2"... None when the line is not made only of values. With a `prefix`, only
/// the lines that begin with it are read, without it.
pub fn parse(text: &str, prefix: &str) -> Option<Vec<(String, f64)>> {
    let text = text.trim();
    let text = if prefix.is_empty() { text } else { text.strip_prefix(prefix)?.trim_start() };
    let mut values = Vec::new();
    let mut bare = 0;
    // "temp: 21.5": a name whose value is the next piece.
    let mut named: Option<&str> = None;
    for piece in text.split(|c: char| c.is_whitespace() || c == ',' || c == ';').filter(|piece| !piece.is_empty()) {
        if let Some(name) = named.take() {
            values.push((name.to_owned(), number(piece)?));
            continue;
        }
        match piece.split_once(['=', ':']) {
            Some(("", _)) => return None,
            Some((name, "")) => named = Some(name),
            Some((name, value)) => values.push((name.to_owned(), number(value)?)),
            None => {
                bare += 1;
                values.push((bare.to_string(), number(piece)?));
            }
        }
    }
    (named.is_none() && !values.is_empty()).then_some(values)
}

/// A number as a device writes it: "21.5", "-3", "1.5e3"; never "nan" or "inf".
fn number(text: &str) -> Option<f64> {
    text.parse::<f64>().ok().filter(|value| value.is_finite())
}

pub struct Curve {
    pub name: String,
    /// Time in seconds since the first measure, and value; in the order they came.
    points: VecDeque<[f64; 2]>,
    /// The number of the line each point came in, in the same order.
    numbers: VecDeque<f64>,
}

impl Curve {
    fn new(name: String) -> Self {
        Self { name, points: VecDeque::new(), numbers: VecDeque::new() }
    }

    pub fn points(&self) -> &VecDeque<[f64; 2]> {
        &self.points
    }

    /// The places of its first and last points on the axis.
    pub fn extent(&self, x: XAxis) -> Option<(f64, f64)> {
        match x {
            XAxis::Time => Some((self.points.front()?[0], self.points.back()?[0])),
            XAxis::Number => Some((*self.numbers.front()?, *self.numbers.back()?)),
        }
    }

    /// The indexes of the points from `from` to `to` on the axis.
    fn within(&self, x: XAxis, from: f64, to: f64) -> Range<usize> {
        let (first, end) = match x {
            XAxis::Time => (self.points.partition_point(|point| point[0] < from), self.points.partition_point(|point| point[0] <= to)),
            XAxis::Number => (self.numbers.partition_point(|&number| number < from), self.numbers.partition_point(|&number| number <= to)),
        };
        first..end.max(first)
    }

    /// The lowest and the highest values from `from` to `to` on the axis: the scale they need. None
    /// without a point there.
    pub fn range(&self, x: XAxis, from: f64, to: f64) -> Option<(f64, f64)> {
        self.points.range(self.within(x, from, to)).fold(None, |range, point| {
            let value = point[1];
            Some(range.map_or((value, value), |(low, high): (f64, f64)| (low.min(value), high.max(value))))
        })
    }

    /// The last value from `from` to `to` on the axis. None without a point there.
    pub fn last_within(&self, x: XAxis, from: f64, to: f64) -> Option<f64> {
        self.points.range(self.within(x, from, to)).next_back().map(|point| point[1])
    }

    /// The points from `from` to `to` on the axis, as places on it and values, reduced to what
    /// `columns` pixels can show: in each column, its first, lowest, highest and last points, in
    /// their order, so that no peak is lost. The points just outside are kept, for the curve to
    /// enter and leave the view. With four points a column or fewer, all of them.
    pub fn reduced(&self, x: XAxis, from: f64, to: f64, columns: usize) -> Vec<[f64; 2]> {
        let within = self.within(x, from, to);
        let around = within.start.saturating_sub(1)..(within.end + 1).min(self.points.len());
        match x {
            XAxis::Time => reduce(self.points.range(around).copied(), from, to, columns),
            XAxis::Number => {
                let values = self.points.range(around.clone()).map(|point| point[1]);
                reduce(self.numbers.range(around).copied().zip(values).map(|(number, value)| [number, value]), from, to, columns)
            }
        }
    }
}

/// `points`, in the order of their places, reduced to `columns` from `from` to `to`: see
/// `Curve::reduced`.
fn reduce(points: impl ExactSizeIterator<Item = [f64; 2]>, from: f64, to: f64, columns: usize) -> Vec<[f64; 2]> {
    let columns = columns.max(1);
    if points.len() <= 4 * columns || to <= from {
        return points.collect();
    }
    let width = (to - from) / columns as f64;
    let column = |point: &[f64; 2]| ((point[0] - from) / width).floor().clamp(-1.0, columns as f64) as i64;
    let mut reduced = Vec::with_capacity(4 * columns + 8);
    let mut group: Vec<[f64; 2]> = Vec::new();
    let flush = |group: &mut Vec<[f64; 2]>, reduced: &mut Vec<[f64; 2]>| {
        if group.is_empty() {
            return;
        }
        let lowest = (0..group.len()).min_by(|&a, &b| group[a][1].total_cmp(&group[b][1])).unwrap_or(0);
        let highest = (0..group.len()).max_by(|&a, &b| group[a][1].total_cmp(&group[b][1])).unwrap_or(0);
        let mut kept = [0, lowest, highest, group.len() - 1];
        kept.sort_unstable();
        let mut previous = None;
        for index in kept {
            if previous != Some(index) {
                reduced.push(group[index]);
                previous = Some(index);
            }
        }
        group.clear();
    };
    let mut current = None;
    for point in points {
        let here = column(&point);
        if current != Some(here) {
            flush(&mut group, &mut reduced);
            current = Some(here);
        }
        group.push(point);
    }
    flush(&mut group, &mut reduced);
    reduced
}

pub struct Plot {
    /// In the order they first appeared.
    curves: Vec<Curve>,
    /// When the first measure came: the zero of the time.
    start: Option<crate::console::Time>,
    /// Each line read, for the export: its time and its values by curve.
    rows: VecDeque<(f64, Vec<(usize, f64)>)>,
    /// The lines of measures read since the first: the number of the last.
    count: u64,
    /// The points each curve keeps, and the lines the export keeps.
    limit: usize,
}

impl Plot {
    pub fn new(limit: usize) -> Self {
        Self { curves: Vec::new(), start: None, rows: VecDeque::new(), count: 0, limit: limit.max(1) }
    }

    pub fn curves(&self) -> &[Curve] {
        &self.curves
    }

    pub fn is_empty(&self) -> bool {
        self.curves.is_empty()
    }

    /// The place of the last measure on the axis: its time, or its number.
    pub fn last(&self, x: XAxis) -> f64 {
        match x {
            XAxis::Time => self.rows.back().map_or(0.0, |row| row.0),
            XAxis::Number => self.count as f64,
        }
    }

    /// The places of the first and the last points kept, of all the curves, on the axis.
    pub fn extent(&self, x: XAxis) -> Option<(f64, f64)> {
        self.curves.iter().filter_map(|curve| curve.extent(x)).reduce(|(first, last), (start, end)| (first.min(start), last.max(end)))
    }

    pub fn set_limit(&mut self, limit: usize) {
        self.limit = limit.max(1);
        self.trim();
    }

    /// Reads a line the device sent: whether it was one of measures.
    pub fn read(&mut self, line: &Line, prefix: &str) -> bool {
        if line.kind != Kind::Received {
            return false;
        }
        let Some(values) = parse(line.text(), prefix) else {
            return false;
        };
        let start = *self.start.get_or_insert(line.time);
        let time = (line.time - start).num_microseconds().map_or(0.0, |micros| micros as f64 / 1e6);
        self.count += 1;
        let number = self.count as f64;
        let mut row = Vec::with_capacity(values.len());
        for (name, value) in values {
            let index = if let Some(index) = self.curves.iter().position(|curve| curve.name == name) {
                index
            } else {
                self.curves.push(Curve::new(name));
                self.curves.len() - 1
            };
            self.curves[index].points.push_back([time, value]);
            self.curves[index].numbers.push_back(number);
            row.push((index, value));
        }
        self.rows.push_back((time, row));
        self.trim();
        true
    }

    pub fn clear(&mut self) {
        self.curves.clear();
        self.start = None;
        self.rows.clear();
        self.count = 0;
    }

    /// The measures kept, for a spreadsheet set to French: a BOM, semicolons, decimal commas, so
    /// that Excel in French opens them as numbers. A column of time, then one per curve.
    ///
    /// A curve's name comes from the device: one that begins as a formula would ("@SUM(1)", "+cmd")
    /// gets an apostrophe before it, so that a spreadsheet shows it as text and runs nothing. The
    /// quotes alone do not stop it.
    pub fn csv(&self, time_header: &str) -> String {
        let quoted = |text: &str| {
            let text = if text.starts_with(['=', '+', '-', '@', '\t', '\r']) { format!("'{text}") } else { text.to_owned() };
            format!("\"{}\"", text.replace('"', "\"\""))
        };
        let decimal = |value: f64| value.to_string().replace('.', ",");
        let mut text = String::from("\u{feff}");
        let header: Vec<String> = std::iter::once(quoted(time_header)).chain(self.curves.iter().map(|curve| quoted(&curve.name))).collect();
        text.push_str(&header.join(";"));
        text.push_str("\r\n");
        for (time, values) in &self.rows {
            let mut cells = vec![String::new(); self.curves.len()];
            for &(index, value) in values {
                cells[index] = decimal(value);
            }
            text.push_str(&decimal((time * 1000.0).round() / 1000.0));
            for cell in cells {
                text.push(';');
                text.push_str(&cell);
            }
            text.push_str("\r\n");
        }
        text
    }

    fn trim(&mut self) {
        for curve in &mut self.curves {
            while curve.points.len() > self.limit {
                curve.points.pop_front();
                curve.numbers.pop_front();
            }
        }
        while self.rows.len() > self.limit {
            self.rows.pop_front();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::console::{Console, at};

    fn values(text: &str) -> Option<Vec<(String, f64)>> {
        parse(text, "")
    }

    fn named(pairs: &[(&str, f64)]) -> Option<Vec<(String, f64)>> {
        Some(pairs.iter().map(|&(name, value)| (name.to_owned(), value)).collect())
    }

    /// The lines of what a device sent, each at its own time.
    fn lines(sent: &[(&[u8], u32)]) -> Console {
        let mut console = Console::new(1000);
        for &(bytes, millis) in sent {
            console.receive(bytes, at(millis / 1000, millis % 1000));
        }
        console
    }

    #[test]
    fn bare_values_are_named_by_their_place() {
        assert_eq!(values("21.5 40"), named(&[("1", 21.5), ("2", 40.0)]));
        assert_eq!(values("1,2;3\t4"), named(&[("1", 1.0), ("2", 2.0), ("3", 3.0), ("4", 4.0)]));
        assert_eq!(values("  -1.5e3  .5 7. "), named(&[("1", -1500.0), ("2", 0.5), ("3", 7.0)]));
    }

    #[test]
    fn named_values_take_their_name() {
        assert_eq!(values("temp=21.5 hum=40"), named(&[("temp", 21.5), ("hum", 40.0)]));
        assert_eq!(values("temp:21.5,hum:40"), named(&[("temp", 21.5), ("hum", 40.0)]));
        assert_eq!(values("temp: 21.5, hum: 40"), named(&[("temp", 21.5), ("hum", 40.0)]));
        assert_eq!(values("v3.3=5.1 10"), named(&[("v3.3", 5.1), ("1", 10.0)]), "named and bare in one line");
    }

    #[test]
    fn a_line_with_anything_else_is_not_one_of_measures() {
        assert_eq!(values("I (5140) devices: Init capport"), None, "an ESP-IDF log line");
        assert_eq!(values("load 3.2"), None);
        assert_eq!(values("12:30:45"), None, "a clock time");
        assert_eq!(values("temp:"), None, "a name without its value");
        assert_eq!(values("=3"), None, "a value without its name");
        assert_eq!(values("nan 1"), None);
        assert_eq!(values("inf"), None);
        assert_eq!(values(""), None);
        assert_eq!(values(" , ; "), None);
    }

    #[test]
    fn with_a_prefix_only_its_lines_are_read() {
        assert_eq!(parse("PLOT 1 2", "PLOT"), named(&[("1", 1.0), ("2", 2.0)]));
        assert_eq!(parse("PLOT:a=1", "PLOT:"), named(&[("a", 1.0)]));
        assert_eq!(parse("1 2", "PLOT"), None, "no prefix, not read");
        assert_eq!(parse("PLOTTER 1", "PLOT"), None, "the prefix is not a word's start");
    }

    #[test]
    fn curves_come_in_order_and_their_time_counts_from_the_first_measure() {
        let console = lines(&[(b"boot ok\r\n", 500), (b"a=1 b=10\r\n", 1000), (b"b=20\r\n", 1250), (b"c=5 a=2\r\n", 3000)]);
        let mut plot = Plot::new(100);
        let read: Vec<bool> = console.lines().iter().map(|line| plot.read(line, "")).collect();
        assert_eq!(read, [false, true, true, true]);
        let names: Vec<&str> = plot.curves().iter().map(|curve| curve.name.as_str()).collect();
        assert_eq!(names, ["a", "b", "c"]);
        let points = |i: usize| plot.curves()[i].points().iter().copied().collect::<Vec<_>>();
        assert_eq!(points(0), [[0.0, 1.0], [2.0, 2.0]]);
        assert_eq!(points(1), [[0.0, 10.0], [0.25, 20.0]]);
        assert_eq!(points(2), [[2.0, 5.0]]);
        assert_eq!(plot.last(XAxis::Time), 2.0);
    }

    #[test]
    fn each_line_of_measures_has_its_number_from_one_shared_by_its_values() {
        let console = lines(&[(b"a=1 b=10\r\n", 0), (b"boot ok\r\n", 100), (b"b=20\r\n", 200), (b"c=5 a=2\r\n", 300)]);
        let mut plot = Plot::new(100);
        for line in console.lines() {
            plot.read(line, "");
        }
        let numbered = |i: usize| plot.curves()[i].reduced(XAxis::Number, 0.0, 10.0, 100);
        assert_eq!(numbered(0), [[1.0, 1.0], [3.0, 2.0]], "the log line has no number");
        assert_eq!(numbered(1), [[1.0, 10.0], [2.0, 20.0]]);
        assert_eq!(numbered(2), [[3.0, 5.0]], "the line's number, shared by its values");
        assert_eq!((plot.last(XAxis::Number), plot.extent(XAxis::Number)), (3.0, Some((1.0, 3.0))));
        plot.clear();
        plot.read(&console.lines()[3], "");
        assert_eq!(plot.curves()[0].reduced(XAxis::Number, 0.0, 10.0, 100), [[1.0, 5.0]], "cleared: from one again");
    }

    #[test]
    fn the_numbers_go_on_when_the_oldest_points_go() {
        let sent: Vec<(Vec<u8>, u32)> = (0..10).map(|n| (format!("{n}\r\n").into_bytes(), n * 100)).collect();
        let sent: Vec<(&[u8], u32)> = sent.iter().map(|(bytes, millis)| (bytes.as_slice(), *millis)).collect();
        let console = lines(&sent);
        let mut plot = Plot::new(4);
        for line in console.lines() {
            plot.read(line, "");
        }
        assert_eq!(plot.curves()[0].reduced(XAxis::Number, 0.0, 100.0, 100), [[7.0, 6.0], [8.0, 7.0], [9.0, 8.0], [10.0, 9.0]]);
        assert_eq!(plot.extent(XAxis::Number), Some((7.0, 10.0)));
        assert_eq!(plot.extent(XAxis::Time), Some((0.6, 0.9)));
    }

    #[test]
    fn lines_sent_and_notices_make_no_curve() {
        let mut console = Console::new(10);
        console.sent("1 2", b"1 2\n", at(0, 0));
        console.notice("3 4", at(0, 0));
        let mut plot = Plot::new(10);
        assert!(console.lines().iter().all(|line| !plot.read(line, "")));
        assert!(plot.is_empty());
    }

    #[test]
    fn each_curve_keeps_its_latest_points_only() {
        let sent: Vec<(Vec<u8>, u32)> = (0..10).map(|n| (format!("{n}\r\n").into_bytes(), n * 100)).collect();
        let sent: Vec<(&[u8], u32)> = sent.iter().map(|(bytes, millis)| (bytes.as_slice(), *millis)).collect();
        let console = lines(&sent);
        let mut plot = Plot::new(4);
        for line in console.lines() {
            plot.read(line, "");
        }
        let kept: Vec<f64> = plot.curves()[0].points().iter().map(|point| point[1]).collect();
        assert_eq!(kept, [6.0, 7.0, 8.0, 9.0]);
        plot.set_limit(2);
        assert_eq!(plot.curves()[0].points().len(), 2);
        assert_eq!(plot.csv("t").lines().count(), 3, "the export keeps as many lines");
    }

    /// A curve of `count` points, one a millisecond, its value given by `value`.
    fn curve(count: usize, value: impl Fn(usize) -> f64) -> Curve {
        Curve { name: "a".into(), points: (0..count).map(|n| [n as f64 / 1000.0, value(n)]).collect(), numbers: (1..=count).map(|n| n as f64).collect() }
    }

    #[test]
    fn few_points_are_all_kept() {
        let curve = curve(40, |n| n as f64);
        assert_eq!(curve.reduced(XAxis::Time, 0.0, 1.0, 10).len(), 40);
    }

    #[test]
    fn along_the_numbers_the_reduction_keeps_the_peaks_too() {
        let curve = curve(10_000, |n| if n == 4320 { 99.0 } else { 0.0 });
        let reduced = curve.reduced(XAxis::Number, 1.0, 10_000.0, 100);
        assert!(reduced.len() <= 4 * 100 + 2, "{} points", reduced.len());
        assert!(reduced.contains(&[4321.0, 99.0]), "the peak, at its number");
        assert_eq!((reduced.first(), reduced.last()), (Some(&[1.0, 0.0]), Some(&[10_000.0, 0.0])));
    }

    #[test]
    fn the_range_is_the_lowest_and_highest_values_of_the_period() {
        let ramp = curve(10_000, |n| n as f64);
        assert_eq!(ramp.range(XAxis::Time, 2.0, 3.0), Some((2000.0, 3000.0)), "the points around are not counted");
        assert_eq!(ramp.range(XAxis::Number, 1.0, 10.0), Some((0.0, 9.0)));
        assert_eq!(ramp.range(XAxis::Time, 20.0, 30.0), None, "no point there");
        let flat = curve(5, |_| 21.5);
        assert_eq!(flat.range(XAxis::Time, 0.0, 1.0), Some((21.5, 21.5)));
    }

    #[test]
    fn many_points_are_reduced_to_four_a_column_at_most_and_keep_their_peaks() {
        // 10 000 points over 10 s, a spike at 4.321 s; 100 columns of 0.1 s.
        let curve = curve(10_000, |n| if n == 4321 { 99.0 } else if n == 7777 { -99.0 } else { (n as f64 / 500.0).sin() });
        let reduced = curve.reduced(XAxis::Time, 0.0, 10.0, 100);
        assert!(reduced.len() <= 4 * 100 + 2, "{} points", reduced.len());
        assert!(reduced.contains(&[4.321, 99.0]), "the highest point is kept");
        assert!(reduced.contains(&[7.777, -99.0]), "the lowest point is kept");
        assert!(reduced.windows(2).all(|pair| pair[0][0] < pair[1][0]), "in the order of time");
        assert_eq!(reduced.first(), Some(&[0.0, 0.0]));
        assert_eq!(reduced.last().map(|point| point[0]), Some(9.999));
    }

    #[test]
    fn only_the_view_is_reduced_with_a_point_either_side() {
        let curve = curve(10_000, |n| n as f64);
        let reduced = curve.reduced(XAxis::Time, 2.0, 3.0, 10);
        assert_eq!(reduced.first(), Some(&[1.999, 1999.0]), "the point before the view");
        assert_eq!(reduced.last(), Some(&[3.001, 3001.0]), "the point after it");
        assert!(reduced.len() <= 4 * 10 + 2 + 4);
    }

    #[test]
    fn clearing_starts_the_time_again() {
        let console = lines(&[(b"1\r\n", 0), (b"2\r\n", 5000)]);
        let mut plot = Plot::new(10);
        plot.read(&console.lines()[0], "");
        plot.clear();
        assert!(plot.is_empty());
        plot.read(&console.lines()[1], "");
        assert_eq!(plot.curves()[0].points()[0], [0.0, 2.0]);
    }

    #[test]
    fn the_export_opens_in_excel_as_numbers() {
        let console = lines(&[(b"temp=21.5 hum=40\r\n", 0), (b"hum=41\r\n", 1500), (b"\"x\"=3\r\n", 2001)]);
        let mut plot = Plot::new(10);
        for line in console.lines() {
            plot.read(line, "");
        }
        assert_eq!(
            plot.csv("temps (s)"),
            "\u{feff}\"temps (s)\";\"temp\";\"hum\";\"\"\"x\"\"\"\r\n0;21,5;40;\r\n1,5;;41;\r\n2,001;;;3\r\n"
        );
        assert_eq!(Plot::new(10).csv("t"), "\u{feff}\"t\"\r\n", "the header alone");
    }

    #[test]
    fn a_name_made_as_a_formula_is_exported_as_text() {
        // A device that names its curves as formulas: Excel would run them.
        let console = lines(&[(b"@SUM(1)=5 +cmd=2 -x=-3 ok=4\r\n", 0)]);
        let mut plot = Plot::new(10);
        plot.read(&console.lines()[0], "");
        assert_eq!(plot.csv("t"), "\u{feff}\"t\";\"'@SUM(1)\";\"'+cmd\";\"'-x\";\"ok\"\r\n0;5;2;-3;4\r\n", "names as text, numbers as numbers");
    }
}
