//! What a frame of the console draws: its rows, where the text goes, the place of a point; and
//! the lines in view given to screen readers.

use eframe::egui::{self, Pos2, Rect};

use super::Look;
use crate::console::{Console, Kind, Position, Wrap};

/// The lines in view given to screen readers, as painted and as copied, each where its rows in view
/// show; a line sent and a word of RSC say what they are, as « > » and « # » show it.
pub(super) fn read(ui: &mut egui::Ui, response: &egui::Response, scene: &Scene) {
    let Scene { console, wrap, look, .. } = *scene;
    let clip = ui.clip_rect();
    let mut lines: Vec<(usize, Rect)> = Vec::new();
    for row in scene.rows_in(clip) {
        let Some((index, _, _)) = wrap.at_row(row) else {
            break;
        };
        let shown = scene.row_rect(row).intersect(clip);
        match lines.last_mut() {
            Some((last, rect)) if *last == index => *rect = rect.union(shown),
            _ => lines.push((index, shown)),
        }
    }
    let lines = lines.into_iter().map(|(index, rect)| {
        let line = &console.lines()[index];
        let what = match line.kind {
            Kind::Received => None,
            Kind::Sent => Some(t!("envoyé", "sent")),
            Kind::Notice => Some(t!("message de RSC", "message from RSC")),
        };
        (response.id.with(("line", console.first() + index as u64)), rect, line.display(look.shown), what)
    });
    super::widgets::text_lines(ui, response, lines);
}

/// What a frame draws: the console and its rows, where the text goes, and the size of its cells.
pub(super) struct Scene<'a> {
    pub(super) console: &'a Console,
    pub(super) wrap: &'a Wrap,
    pub(super) look: &'a Look,
    pub(super) rect: Rect,
    pub(super) char_width: f32,
    pub(super) row_height: f32,
}

impl Scene<'_> {
    /// The rows in view: those `clip` shows, painted and given to screen readers.
    pub(super) fn rows_in(&self, clip: Rect) -> std::ops::Range<usize> {
        let first = ((clip.top() - self.rect.top()) / self.row_height).floor().max(0.0) as usize;
        let last = (((clip.bottom() - self.rect.top()) / self.row_height).ceil().max(0.0) as usize).min(self.wrap.rows());
        first..last.max(first)
    }

    /// Where row `row` shows.
    pub(super) fn row_rect(&self, row: usize) -> Rect {
        Rect::from_min_size(Pos2::new(self.rect.left(), self.rect.top() + row as f32 * self.row_height), egui::vec2(self.rect.width(), self.row_height))
    }

    /// The place in the console of a point on screen. `between` gives the gap between two
    /// characters nearest to the point, as a caret would be; else, the character under it.
    pub(super) fn place(&self, point: Pos2, between: bool) -> Position {
        let shown = self.look.shown;
        let row = ((point.y - self.rect.top()) / self.row_height).floor().max(0.0) as usize;
        let cell = (point.x - self.rect.left()) / self.char_width;
        let cell = (if between { cell.round() } else { cell.floor() }).max(0.0) as usize;
        match self.wrap.at_row(row) {
            Some((index, from, cap)) => {
                let width = self.console.lines()[index].width(shown);
                Position { line: self.console.first() + index as u64, col: (from + cell.min(cap)).min(width) }
            }
            None => self.console.end(shown),
        }
    }
}
