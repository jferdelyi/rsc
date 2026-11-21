//! How the console's lines fold at its width, kept as the console changes: only the lines that
//! changed are folded again.

use std::collections::VecDeque;

use super::{Console, Fold, Shown};

/// Where each line falls once the long ones are folded at the console's width: a line takes as
/// many rows as it needs, one at least. The first row of a line holds what comes before its body
/// and whole units of it; the next rows, whole units.
///
/// It follows the console as it changes (`update`) rather than folding every line again: lines
/// only come at the end, go from the front, and only the last one still changes while it is not
/// ended. A console of a million lines that receives one folds one or two lines, not a million.
pub struct Wrap {
    cols: usize,
    /// The number, in the console, of the first line it folds.
    first: u64,
    /// The first row of each line, then one past the last row, each counted from `base`: the rows
    /// of the lines gone at the front are taken out by `base`, not by rewriting every start.
    starts: VecDeque<usize>,
    base: usize,
    /// The characters the first row of each line holds, and those its next rows hold.
    caps: VecDeque<(usize, usize)>,
}

/// The rows a line takes at a width, the characters of its first row and of its next rows.
fn fold_rows(fold: Fold, cols: usize) -> (usize, (usize, usize)) {
    let unit = fold.unit.max(1);
    let next = (cols / unit * unit).max(unit);
    let first = if cols > fold.body + unit { fold.body + (cols - fold.body) / unit * unit } else { cols };
    let rows = if fold.width <= first { 1 } else { 1 + (fold.width - first).div_ceil(next) };
    (rows, (first, next))
}

impl Wrap {
    /// From how each line folds, and the width of the console in characters.
    pub fn new(folds: impl Iterator<Item = Fold>, cols: usize) -> Self {
        let mut wrap = Self { cols: cols.max(1), first: 0, starts: VecDeque::from([0]), base: 0, caps: VecDeque::new() };
        for fold in folds {
            wrap.push(fold);
        }
        wrap
    }

    /// The lines of a console, as `shown`, at a width.
    pub fn of(console: &Console, shown: Shown, cols: usize) -> Self {
        let mut wrap = Self::new(console.lines().iter().map(|line| line.fold(shown)), cols);
        wrap.first = console.first();
        wrap
    }

    /// Follows the console since the last fold, `shown` and the width unchanged: the lines gone
    /// from the front are taken out, the last line known is folded again, the new ones are added.
    /// Returns how many lines it folded.
    pub fn update(&mut self, console: &Console, shown: Shown) -> usize {
        let lines = console.lines();
        let known = self.caps.len();
        let gone = console.first().checked_sub(self.first).and_then(|gone| usize::try_from(gone).ok());
        // Cleared, or a console it never folded: everything again.
        let Some(gone) = gone.filter(|&gone| gone < known && known - gone <= lines.len()) else {
            *self = Self::of(console, shown, self.cols);
            return lines.len();
        };
        for _ in 0..gone {
            self.starts.pop_front();
            self.caps.pop_front();
        }
        self.first = console.first();
        self.base = self.starts.front().copied().unwrap_or(self.base);
        // The last line known may have grown: folded again, with every line after it.
        let from = known - gone - 1;
        self.starts.truncate(from + 1);
        self.caps.truncate(from);
        for line in lines.iter().skip(from) {
            self.push(line.fold(shown));
        }
        lines.len() - from
    }

    fn push(&mut self, fold: Fold) {
        let (rows, caps) = fold_rows(fold, self.cols);
        let end = self.starts.back().copied().unwrap_or(self.base) + rows;
        self.starts.push_back(end);
        self.caps.push_back(caps);
    }

    pub fn cols(&self) -> usize {
        self.cols
    }

    pub fn rows(&self) -> usize {
        self.starts.back().copied().unwrap_or(self.base) - self.base
    }

    /// The line a row belongs to, by its index, the first character the row shows, and the
    /// characters it holds at most.
    pub fn at_row(&self, row: usize) -> Option<(usize, usize, usize)> {
        if row >= self.rows() {
            return None;
        }
        let row = row + self.base;
        let line = self.starts.partition_point(|&start| start <= row) - 1;
        let (first, next) = self.caps[line];
        Some(match row - self.starts[line] {
            0 => (line, 0, first),
            r => (line, first + (r - 1) * next, next),
        })
    }

    /// The row that shows the character `col` of a line. The end of a full row stays on it.
    pub fn row_of(&self, line: usize, col: usize) -> usize {
        let (Some(&start), Some(&end), Some(&(first, next))) = (self.starts.get(line), self.starts.get(line + 1), self.caps.get(line)) else {
            return self.rows();
        };
        let row = if col < first { 0 } else { 1 + (col - first) / next };
        start - self.base + row.min(end - start - 1)
    }
}
