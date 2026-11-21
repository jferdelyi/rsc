//! The console painted: its rows in view with their colours and the selection, the minimap at
//! its right, and what a row shows of its line.

use eframe::egui::text::LayoutJob;
use eframe::egui::{self, Color32, CornerRadius, FontId, Pos2, Rect, Sense, Stroke, StrokeKind, TextFormat};

use super::{ConsoleView, Look, MINIMAP_ROW, theme};
use super::scene::Scene;
use crate::console::{Color, Line, Position, TIME_WIDTH};

impl ConsoleView {
    pub(super) fn paint(&self, ui: &egui::Ui, scene: &Scene, font: &FontId) {
        let Scene { console, wrap, look, rect, char_width, row_height } = *scene;
        let selection = self.selection.map(|(a, b)| if a <= b { (a, b) } else { (b, a) }).filter(|(a, b)| a != b);
        for row in scene.rows_in(ui.clip_rect()) {
            let Some((index, from, cap)) = wrap.at_row(row) else {
                break;
            };
            let line = &console.lines()[index];
            let width = line.width(look.shown);
            let to = (from + cap).min(width);
            let corner = Pos2::new(rect.left(), rect.top() + row as f32 * row_height);
            if let Some((start, end)) = selection {
                let number = console.first() + index as u64;
                // The end of a line counts as a cell: a selection that goes on to the next line shows it.
                let row_end = if from + cap >= width { width + 1 } else { to };
                if let Some((a, b)) = selected_cells(number, from, row_end, start, end) {
                    let left = corner.x + (a - from) as f32 * char_width;
                    let right = corner.x + (b - from) as f32 * char_width;
                    let cells = Rect::from_min_max(Pos2::new(left, corner.y), Pos2::new(right, corner.y + row_height));
                    ui.painter().rect_filled(cells, CornerRadius::ZERO, theme::CONSOLE_SELECTION);
                }
            }
            if from < to {
                let galley = ui.painter().layout_job(row_job(line, look, from, to, font));
                ui.painter().galley(corner, galley, theme::CONSOLE_TEXT);
            }
        }
    }

    /// The whole console in miniature, each row a bar as long as its text, and the part in view
    /// (its first row and its number of rows) framed. A click or a drag there moves the console.
    /// The scene's rectangle is the minimap's strip.
    pub(super) fn minimap(&mut self, ui: &egui::Ui, scene: &Scene, view: (f32, f32)) {
        let Scene { console, wrap, look, rect: strip, row_height, .. } = *scene;
        let rows = wrap.rows();
        // A shortcut of the pointer, out of the keyboard's way: Tab does not stop on it.
        let response = ui.interact(strip, ui.id().with("minimap"), Sense::CLICK | Sense::DRAG);
        if rows == 0 {
            return;
        }
        let mini_row = (strip.height() / rows as f32).min(MINIMAP_ROW);
        let painter = ui.painter_at(strip);
        let unit = strip.width() / wrap.cols() as f32;
        let pixels = (rows as f32 * mini_row).min(strip.height()).ceil() as usize;
        // One bar per pixel row at most: the row of the console it falls on.
        let step = mini_row.max(1.0);
        let mut y = 0.0;
        while y < pixels as f32 {
            let row = ((y / mini_row) as usize).min(rows - 1);
            if let Some((index, from, cap)) = wrap.at_row(row) {
                let line = &console.lines()[index];
                let to = line.width(look.shown).min(from + cap);
                let height = (step - 1.0).max(1.0);
                // Word by word, in the colours of the text: the miniature reads as the console does.
                for (start, end, color) in ink_runs(&line.display(look.shown), &pieces(line, look), from, to) {
                    let left = strip.left() + (start - from) as f32 * unit;
                    let bar = Rect::from_min_size(Pos2::new(left, strip.top() + y), egui::vec2(((end - start) as f32 * unit).max(1.0), height));
                    painter.rect_filled(bar, CornerRadius::ZERO, color);
                }
            }
            y += step;
        }
        let (top, height) = view;
        let frame = Rect::from_min_size(Pos2::new(strip.left(), strip.top() + top * mini_row), egui::vec2(strip.width(), (height * mini_row).max(4.0)));
        painter.rect_filled(frame, CornerRadius::same(3), Color32::from_white_alpha(14));
        painter.rect_stroke(frame, CornerRadius::same(3), Stroke::new(1.0_f32, Color32::from_white_alpha(40)), StrokeKind::Inside);
        if let Some(pointer) = response.interact_pointer_pos().filter(|_| response.clicked() || response.dragged()) {
            let row = (pointer.y - strip.top()) / mini_row;
            self.scroll_to = Some(((row - height / 2.0) * row_height).max(0.0));
            ui.ctx().request_repaint();
        }
    }
}

/// The cells of a row that a selection covers, as characters of its line: the row shows the
/// characters `from` to `row_end` of the line `number`.
pub(super) fn selected_cells(number: u64, from: usize, row_end: usize, start: Position, end: Position) -> Option<(usize, usize)> {
    if number < start.line || number > end.line {
        return None;
    }
    let a = if number == start.line { start.col } else { 0 }.max(from);
    let b = if number == end.line { end.col } else { usize::MAX }.min(row_end);
    (a < b).then_some((a, b))
}

/// The characters `from` to `to` of a line as shown, each piece in its colour: the time, then the
/// text, span by span.
pub(super) fn row_job(line: &Line, look: &Look, from: usize, to: usize, font: &FontId) -> LayoutJob {
    let mut job = LayoutJob::default();
    let shown = line.display(look.shown);
    for (start, end, color) in pieces(line, look) {
        let (a, b) = (start.max(from), end.min(to));
        if a < b {
            job.append(char_slice(&shown, a, b), 0.0, TextFormat::simple(font.clone(), color));
        }
    }
    job
}

/// A line as shown, cut where its colour changes: each piece from a character to another. Its
/// bytes in hexadecimal are of one colour.
pub(super) fn pieces(line: &Line, look: &Look) -> Vec<(usize, usize, Color32)> {
    let offset = if look.shown.timestamps { TIME_WIDTH } else { 0 };
    let mut starts: Vec<(usize, Color32)> = Vec::new();
    if look.shown.timestamps {
        starts.push((0, theme::CONSOLE_TIME));
    }
    let spans = if line.in_hex(look.shown) { &[][..] } else { line.spans() };
    if spans.first().is_none_or(|span| span.start > 0) {
        starts.push((offset, theme::ink(line.kind, Color::Default, look.ansi_colors)));
    }
    starts.extend(spans.iter().map(|span| (offset + span.start, theme::ink(line.kind, span.color, look.ansi_colors))));
    let ends = starts.iter().skip(1).map(|next| next.0).chain([usize::MAX]);
    starts.iter().zip(ends).map(|(&(start, color), end)| (start, end, color)).collect()
}

/// The runs of a line's characters between `from` and `to` that are not blanks, each with the
/// colour of its piece: what the minimap draws of a row.
pub(super) fn ink_runs(shown: &str, pieces: &[(usize, usize, Color32)], from: usize, to: usize) -> Vec<(usize, usize, Color32)> {
    let chars: Vec<char> = shown.chars().collect();
    let mut runs = Vec::new();
    for &(start, end, color) in pieces {
        let (a, b) = (start.max(from), end.min(to).min(chars.len()));
        if a >= b {
            continue;
        }
        let mut run: Option<usize> = None;
        // A blank after the last character closes the last run.
        let marks = chars[a..b].iter().map(|c| !c.is_whitespace()).chain(std::iter::once(false));
        for (offset, ink) in marks.enumerate() {
            let col = a + offset;
            match (ink, run) {
                (true, None) => run = Some(col),
                (false, Some(first)) => {
                    runs.push((first, col, color));
                    run = None;
                }
                _ => {}
            }
        }
    }
    runs
}

/// The characters `from` to `to` of a text.
pub(super) fn char_slice(text: &str, from: usize, to: usize) -> &str {
    let byte = |index: usize| text.char_indices().nth(index).map_or(text.len(), |(at, _)| at);
    let (start, end) = if text.is_ascii() { (from.min(text.len()), to.min(text.len())) } else { (byte(from), byte(to)) };
    &text[start..end.max(start)]
}
