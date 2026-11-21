//! The terminal mode on screen: the device's screen drawn cell by cell in the console's colours,
//! its cursor, its history under the wheel, the cells selected with the mouse, and the keys typed
//! turned into bytes for the device.

use eframe::egui::accesskit::Role;
use eframe::egui::text::{LayoutJob, TextFormat};
use eframe::egui::{self, Color32, CornerRadius, FontId, Margin, Pos2, Rect, Sense, Stroke, StrokeKind};

use super::theme;
use crate::terminal::{self, Key, Modifiers, Terminal};

/// A cell of what is shown: its row, then its column.
type Cell = (u16, u16);

#[derive(Default)]
pub struct TerminalView {
    /// Where the selection began, and where it ends now.
    selection: Option<(Cell, Cell)>,
    /// The part of a row the wheel turned and that did not make a whole row yet.
    wheel: f32,
    /// The view takes the keyboard at the next frame.
    pub focus: bool,
    /// Where a character is drawn: the corner of the screen, the size of a cell.
    #[cfg(test)]
    cells: Option<(Pos2, egui::Vec2)>,
}

/// What the user asked of the terminal in a frame.
#[derive(Default)]
pub struct Asked {
    /// The bytes the keys typed and the text pasted make, for the device.
    pub send: Vec<u8>,
    /// A text to put in the clipboard.
    pub copy: Option<String>,
    pub clear: bool,
}

/// Where the cells are on screen.
struct Grid {
    rect: Rect,
    char_width: f32,
    row_height: f32,
}

impl Grid {
    fn cell(&self, (row, col): Cell, width: u16) -> Rect {
        let corner = Pos2::new(self.rect.left() + f32::from(col) * self.char_width, self.rect.top() + f32::from(row) * self.row_height);
        Rect::from_min_size(corner, egui::vec2(f32::from(width) * self.char_width, self.row_height))
    }

    /// The cell under a point. `between` gives the gap between two cells nearest to it, as a caret
    /// would be, for where a selection begins or ends.
    fn place(&self, point: Pos2, (rows, cols): (u16, u16), between: bool) -> Cell {
        let row = ((point.y - self.rect.top()) / self.row_height).floor().clamp(0.0, f32::from(rows - 1));
        let col = (point.x - self.rect.left()) / self.char_width;
        let col = if between { col.round() } else { col.floor() }.clamp(0.0, f32::from(cols));
        (row as u16, col as u16)
    }
}

/// How a run of cells is drawn: its colours once inverse and bold are taken in, its line.
#[derive(Clone, Copy, PartialEq)]
struct Style {
    fg: Color32,
    /// None on the console's own background.
    bg: Option<Color32>,
    underline: bool,
    italic: bool,
}

impl Style {
    fn of(cell: &vt100::Cell) -> Self {
        // Bold shows as the bright form of the first 8 colours, as in the line console.
        let fg = match cell.fgcolor() {
            vt100::Color::Idx(index @ 0..=7) if cell.bold() => vt100::Color::Idx(index + 8),
            color => color,
        };
        let (mut fg, mut bg) = (theme::terminal_color(fg, theme::CONSOLE_TEXT), theme::terminal_color(cell.bgcolor(), theme::CONSOLE));
        if cell.inverse() {
            std::mem::swap(&mut fg, &mut bg);
        }
        if cell.dim() {
            fg = fg.gamma_multiply(0.6);
        }
        Self { fg, bg: (bg != theme::CONSOLE).then_some(bg), underline: cell.underline(), italic: cell.italic() }
    }

    fn job(self, text: String, font: &FontId) -> LayoutJob {
        let underline = if self.underline { Stroke::new(1.0_f32, self.fg) } else { Stroke::NONE };
        let mut job = LayoutJob::single_section(text, TextFormat { font_id: font.clone(), color: self.fg, underline, italics: self.italic, ..TextFormat::default() });
        job.wrap.max_width = f32::INFINITY;
        job
    }
}

/// A character as the console's font can draw it. Ten of the line-drawing set have a glyph in no
/// font of its family: the scan lines are drawn as the line nearest their height, the pictures of
/// control characters as a dot. The screen keeps them as they are, a copy gives them as xterm does.
pub(super) fn drawn_as(c: char) -> char {
    match c {
        '\u{23ba}' => '\u{af}',
        '\u{23bb}' | '\u{23bc}' => '\u{2500}',
        '\u{23bd}' => '_',
        '\u{2409}' | '\u{240c}' | '\u{240d}' | '\u{240a}' | '\u{2424}' | '\u{240b}' => '\u{b7}',
        c => c,
    }
}

/// The text of a cell as drawn: a blank for an empty cell.
fn cell_text(cell: &vt100::Cell) -> String {
    if cell.has_contents() { cell.contents().chars().map(drawn_as).collect() } else { " ".to_owned() }
}

/// The arms of a single line or corner, up, down, left, right: these are drawn as lines from the
/// middle of their cell to its edges, so that they join from a cell to the next whatever the height
/// of a row. A glyph is shorter than a row, and leaves gaps.
fn arms(text: &str) -> Option<[bool; 4]> {
    let mut chars = text.chars();
    let (Some(c), None) = (chars.next(), chars.next()) else {
        return None;
    };
    Some(match c {
        '\u{2500}' => [false, false, true, true],
        '\u{2502}' => [true, true, false, false],
        '\u{250c}' => [false, true, false, true],
        '\u{2510}' => [false, true, true, false],
        '\u{2514}' => [true, false, false, true],
        '\u{2518}' => [true, false, true, false],
        '\u{251c}' => [true, true, false, true],
        '\u{2524}' => [true, true, true, false],
        '\u{252c}' => [false, true, true, true],
        '\u{2534}' => [true, false, true, true],
        '\u{253c}' => [true, true, true, true],
        _ => return None,
    })
}

/// The lines of a cell whose character is a line or a corner, a pixel wide, on the pixels' middles.
fn draw_arms(painter: &egui::Painter, cell: Rect, [up, down, left, right]: [bool; 4], color: Color32) {
    let pixels = painter.ctx().pixels_per_point();
    let middle = |value: f32| ((value * pixels).floor() + 0.5) / pixels;
    let (x, y) = (middle(cell.center().x), middle(cell.center().y));
    let stroke = Stroke::new(1.0 / pixels, color);
    let half = 0.5 / pixels;
    for (on, from, to) in [
        (up, Pos2::new(x, cell.top()), Pos2::new(x, y + half)),
        (down, Pos2::new(x, y - half), Pos2::new(x, cell.bottom())),
        (left, Pos2::new(cell.left(), y), Pos2::new(x + half, y)),
        (right, Pos2::new(x - half, y), Pos2::new(cell.right(), y)),
    ] {
        if on {
            painter.line_segment([from, to], stroke);
        }
    }
}

/// The terminal's key for a key of the window: the keys that send something of their own, and a
/// character with Ctrl or Alt held. Text typed comes as text.
fn key_of(key: egui::Key, modifiers: Modifiers) -> Option<Key> {
    use egui::Key as K;
    Some(match key {
        K::Enter => Key::Enter,
        K::Backspace => Key::Backspace,
        K::Tab => Key::Tab,
        K::Escape => Key::Escape,
        K::ArrowUp => Key::Up,
        K::ArrowDown => Key::Down,
        K::ArrowRight => Key::Right,
        K::ArrowLeft => Key::Left,
        K::Home => Key::Home,
        K::End => Key::End,
        K::Insert => Key::Insert,
        K::Delete => Key::Delete,
        K::PageUp => Key::PageUp,
        K::PageDown => Key::PageDown,
        K::F1 => Key::F(1),
        K::F2 => Key::F(2),
        K::F3 => Key::F(3),
        K::F4 => Key::F(4),
        K::F5 => Key::F(5),
        K::F6 => Key::F(6),
        K::F7 => Key::F(7),
        K::F8 => Key::F(8),
        K::F9 => Key::F(9),
        K::F10 => Key::F(10),
        K::F11 => Key::F(11),
        K::F12 => Key::F(12),
        K::Space if modifiers.ctrl || modifiers.alt => Key::Char(' '),
        key if modifiers.ctrl || modifiers.alt => {
            let mut symbol = key.symbol_or_name().chars();
            match (symbol.next(), symbol.next()) {
                (Some(c), None) => Key::Char(c.to_ascii_lowercase()),
                _ => return None,
            }
        }
        _ => return None,
    })
}

impl TerminalView {
    pub fn clear_selection(&mut self) {
        self.selection = None;
    }

    /// The text selected, if any.
    pub fn selected_text(&self, terminal: &Terminal) -> Option<String> {
        let (a, b) = self.selection?;
        (a != b).then(|| terminal.text_between(a, b))
    }

    /// The point on screen in the middle of a cell, as of the last frame.
    #[cfg(test)]
    pub(crate) fn point(&self, (row, col): Cell) -> Pos2 {
        let (corner, size) = self.cells.expect("the terminal was drawn");
        corner + egui::vec2((f32::from(col) + 0.5) * size.x, (f32::from(row) + 0.5) * size.y)
    }

    #[cfg(test)]
    pub(crate) fn select(&mut self, a: Cell, b: Cell) {
        self.selection = Some((a, b));
    }

    /// Draws the terminal in what is left of `ui`, at as many rows and columns as it holds, and
    /// takes the keyboard when it has the focus. `hint` is what an empty screen says.
    pub fn show(&mut self, ui: &mut egui::Ui, terminal: &mut Terminal, font_size: f32, hint: &str) -> Asked {
        let mut asked = Asked::default();
        let font = FontId::monospace(font_size);
        let (char_width, row_height) = ui.ctx().fonts_mut(|fonts| (fonts.glyph_width(&font, 'M'), fonts.row_height(&font)));
        egui::Frame::new().fill(theme::CONSOLE).corner_radius(CornerRadius::same(10)).inner_margin(Margin::symmetric(14, 12)).show(ui, |ui| {
            ui.set_min_size(ui.available_size());
            let area = ui.available_rect_before_wrap();
            let rows = ((area.height() / row_height).floor() as u16).max(1);
            let cols = ((area.width() / char_width).floor() as u16).max(1);
            terminal.resize(rows, cols);
            let (rect, response) = ui.allocate_exact_size(area.size(), Sense::click_and_drag());
            let grid = Grid { rect, char_width, row_height };
            #[cfg(test)]
            {
                self.cells = Some((rect.min, egui::vec2(char_width, row_height)));
            }
            response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Other, true, t!("Écran du terminal", "Terminal screen")));
            if std::mem::take(&mut self.focus) || response.clicked() || response.drag_started() {
                response.request_focus();
            }
            if response.has_focus() {
                // Tab, the arrows and Escape go to the device, not to the next widget.
                ui.memory_mut(|memory| memory.set_focus_lock_filter(response.id, egui::EventFilter { tab: true, horizontal_arrows: true, vertical_arrows: true, escape: true }));
                self.keys(ui, terminal, &mut asked);
            }
            self.wheel(ui, &response, terminal, row_height);
            self.mouse(ui, &response, terminal, &grid);
            self.menu(&response, terminal, &mut asked);
            // An empty screen says what to do, and shows no cursor over it.
            let empty = terminal.screen().contents().trim().is_empty() && terminal.history() == 0;
            self.paint(ui, terminal, &grid, &font, response.has_focus(), !empty);
            if empty {
                ui.painter().text(rect.min, egui::Align2::LEFT_TOP, hint, font.clone(), theme::CONSOLE_TIME);
            }
            // To screen readers, a terminal and its rows with text (WCAG 1.3.1, 4.1.2); where the
            // cursor is when it is drawn, else what an empty screen says.
            let screen = terminal.screen();
            let said = if empty {
                Some(hint.to_owned())
            } else if !screen.hide_cursor() && terminal.scrolled_back() == 0 {
                let (row, col) = screen.cursor_position();
                Some(tr!("Curseur : ligne {}, colonne {}", "Cursor: row {}, column {}", row + 1, col + 1))
            } else {
                None
            };
            if super::widgets::text_area(&response, rect, Role::Terminal, said.as_deref()) {
                let (_, cols) = terminal.size();
                let rows = screen.rows(0, cols).zip(0_u16..).filter(|(text, _)| !text.trim().is_empty());
                let rows = rows.map(|(text, row)| (response.id.with(("row", row)), grid.cell((row, 0), cols), text.trim_end().to_owned(), None));
                super::widgets::text_lines(ui, &response, rows);
            }
        });
        asked
    }

    /// The keys typed and the text pasted, as the device is to receive them. Ctrl+C copies the
    /// selection when there is one and interrupts otherwise; with Shift, it only ever copies.
    fn keys(&mut self, ui: &egui::Ui, terminal: &mut Terminal, asked: &mut Asked) {
        let modes = terminal.modes();
        let (events, held) = ui.input(|input| (input.events.clone(), input.modifiers));
        let mut send = Vec::new();
        for event in events {
            match event {
                // Alt and a letter come as a key, below, that sends ESC before the letter.
                egui::Event::Text(text) if !(held.alt && !held.ctrl) => send.extend(terminal::typed(&text, false)),
                egui::Event::Key { key, pressed: true, modifiers, .. } => {
                    let modifiers = Modifiers { shift: modifiers.shift, alt: modifiers.alt, ctrl: modifiers.ctrl };
                    if let Some(bytes) = key_of(key, modifiers).and_then(|key| terminal::key(key, modifiers, modes)) {
                        send.extend(bytes);
                    }
                }
                egui::Event::Copy => match self.selected_text(terminal) {
                    Some(text) => asked.copy = Some(text),
                    None if !held.shift => send.push(0x03),
                    None => {}
                },
                // Ctrl+X, which nano quits with.
                egui::Event::Cut if !held.shift => send.push(0x18),
                egui::Event::Paste(text) => send.extend(terminal::pasted(&text, modes)),
                _ => {}
            }
        }
        if !send.is_empty() {
            // Typing brings the screen back from the history, as a terminal does.
            terminal.scroll_back(0);
            self.selection = None;
            asked.send = send;
        }
    }

    /// The wheel over the screen goes up and down the history.
    fn wheel(&mut self, ui: &egui::Ui, response: &egui::Response, terminal: &mut Terminal, row_height: f32) {
        if !response.hovered() {
            return;
        }
        let delta = ui.input(|input| input.smooth_scroll_delta.y);
        if delta == 0.0 {
            return;
        }
        self.wheel += delta / row_height;
        let rows = self.wheel.trunc();
        if rows != 0.0 {
            self.wheel -= rows;
            let back = (terminal.scrolled_back() as f32 + rows).max(0.0);
            terminal.scroll_back(back as usize);
        }
    }

    /// Dragging selects, a click unselects, a double click takes a word, a triple click the row.
    fn mouse(&mut self, ui: &egui::Ui, response: &egui::Response, terminal: &Terminal, grid: &Grid) {
        let size = terminal.size();
        if response.drag_started() {
            let origin = ui.ctx().input(|input| input.pointer.press_origin()).or(response.interact_pointer_pos());
            self.selection = origin.map(|origin| {
                let at = grid.place(origin, size, true);
                (at, at)
            });
        }
        if response.dragged()
            && let (Some(pointer), Some((anchor, _))) = (response.interact_pointer_pos(), self.selection)
        {
            self.selection = Some((anchor, grid.place(pointer, size, true)));
        }
        if response.clicked() {
            self.selection = None;
        }
        if let Some(pointer) = response.interact_pointer_pos().or(response.hover_pos()) {
            let (row, col) = grid.place(pointer, size, false);
            if response.triple_clicked() {
                self.selection = Some(((row, 0), (row, size.1)));
            } else if response.double_clicked() {
                self.selection = terminal.word_at(row, col).map(|(start, end)| ((row, start), (row, end)));
            }
        }
    }

    fn menu(&mut self, response: &egui::Response, terminal: &Terminal, asked: &mut Asked) {
        response.context_menu(|ui| {
            let selected = self.selected_text(terminal);
            if ui.add_enabled(selected.is_some(), egui::Button::new(t!("Copier", "Copy"))).clicked() {
                asked.copy = selected;
                ui.close();
            }
            if ui.button(t!("Copier tout l'écran", "Copy the whole screen")).clicked() {
                asked.copy = Some(terminal.screen().contents());
                ui.close();
            }
            if ui.button(t!("Effacer l'écran", "Clear the screen")).clicked() {
                asked.clear = true;
                ui.close();
            }
        });
    }

    /// The screen row by row, in runs of cells of one style: their backgrounds, the selection over
    /// them, the text, then the cursor. A double-width character is drawn alone on its two cells.
    fn paint(&self, ui: &egui::Ui, terminal: &Terminal, grid: &Grid, font: &FontId, focused: bool, cursor: bool) {
        let screen = terminal.screen();
        let (rows, cols) = terminal.size();
        let painter = ui.painter();
        let mut texts = Vec::new();
        let mut lines = Vec::new();
        for row in 0..rows {
            let mut col = 0;
            while col < cols {
                let Some(cell) = screen.cell(row, col) else {
                    break;
                };
                let style = Style::of(cell);
                let start = col;
                let mut text = String::new();
                if cell.is_wide() {
                    text.push_str(&cell_text(cell));
                    col += 2;
                } else {
                    while let Some(next) = screen.cell(row, col).filter(|next| col < cols && !next.is_wide() && Style::of(next) == style) {
                        let content = cell_text(next);
                        match arms(&content) {
                            Some(arms) => {
                                text.push(' ');
                                lines.push((grid.cell((row, col), 1), arms, style.fg));
                            }
                            None => text.push_str(&content),
                        }
                        col += 1;
                    }
                }
                let rect = grid.cell((row, start), col.min(cols) - start);
                if let Some(bg) = style.bg {
                    painter.rect_filled(rect, CornerRadius::ZERO, bg);
                }
                if !text.trim().is_empty() || style.underline {
                    texts.push((rect.min, style.job(text, font)));
                }
            }
        }
        if let Some((a, b)) = self.selection.map(|(a, b)| if a <= b { (a, b) } else { (b, a) }).filter(|(a, b)| a != b) {
            for row in a.0..=b.0 {
                let from = if row == a.0 { a.1 } else { 0 };
                let to = if row == b.0 { b.1 } else { cols };
                if to > from {
                    painter.rect_filled(grid.cell((row, from), to - from), CornerRadius::ZERO, theme::CONSOLE_SELECTION);
                }
            }
        }
        for (corner, job) in texts {
            let galley = painter.layout_job(job);
            painter.galley(corner, galley, theme::CONSOLE_TEXT);
        }
        for (cell, arms, color) in lines {
            draw_arms(painter, cell, arms, color);
        }
        if cursor && !screen.hide_cursor() && terminal.scrolled_back() == 0 {
            let (row, col) = screen.cursor_position();
            let rect = grid.cell((row, col.min(cols - 1)), 1);
            if focused {
                painter.rect_filled(rect, CornerRadius::ZERO, theme::CONSOLE_TEXT);
                if let Some(cell) = screen.cell(row, col).filter(|cell| cell.has_contents()) {
                    let style = Style { fg: theme::CONSOLE, bg: None, ..Style::of(cell) };
                    let galley = painter.layout_job(style.job(cell_text(cell), font));
                    painter.galley(rect.min, galley, theme::CONSOLE);
                }
            } else {
                painter.rect_stroke(rect, CornerRadius::ZERO, Stroke::new(1.0_f32, theme::CONSOLE_TEXT), StrokeKind::Inside);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_keys_of_the_window_become_the_terminal_s() {
        let none = Modifiers::default();
        let ctrl = Modifiers { ctrl: true, ..Modifiers::default() };
        assert_eq!(key_of(egui::Key::ArrowUp, none), Some(Key::Up));
        assert_eq!(key_of(egui::Key::F12, none), Some(Key::F(12)));
        assert_eq!(key_of(egui::Key::A, none), None, "a letter alone comes as text");
        assert_eq!(key_of(egui::Key::A, ctrl), Some(Key::Char('a')));
        assert_eq!(key_of(egui::Key::OpenBracket, ctrl), Some(Key::Char('[')));
        assert_eq!(key_of(egui::Key::Space, ctrl), Some(Key::Char(' ')));
    }

    #[test]
    fn the_lines_and_corners_are_drawn_as_their_arms() {
        assert_eq!(arms("\u{250c}"), Some([false, true, false, true]), "a corner: down and right");
        assert_eq!(arms("\u{253c}"), Some([true; 4]));
        // The line-drawing set's lines and corners, j to x, all of them.
        let drawn = crate::terminal::LINE_DRAWING[10..25].iter().filter(|&&c| arms(&c.to_string()).is_some()).count();
        assert_eq!(drawn, 11, "j k l m n q t u v w x");
        assert_eq!(arms("a"), None);
        assert_eq!(arms("\u{2550}"), None, "double lines stay glyphs");
    }

    #[test]
    fn a_cell_is_drawn_in_the_colours_it_shows() {
        let mut terminal = Terminal::new(1, 10, 0);
        terminal.receive(b"a\x1b[1;31mb\x1b[0;7mc\x1b[0;48;5;196md\x1b[0;38;2;1;2;3me");
        let style = |col| Style::of(terminal.screen().cell(0, col).expect("a cell"));
        assert!(style(0) == Style { fg: theme::CONSOLE_TEXT, bg: None, underline: false, italic: false });
        assert!(style(1).fg == theme::terminal_color(vt100::Color::Idx(9), theme::CONSOLE_TEXT), "bold red is bright red");
        assert!(style(2) == Style { fg: theme::CONSOLE, bg: Some(theme::CONSOLE_TEXT), underline: false, italic: false }, "inverse");
        assert!(style(3).bg == Some(Color32::from_rgb(255, 0, 0)), "colour 196 of the cube");
        assert!(style(4).fg == Color32::from_rgb(1, 2, 3), "a colour of its own");
    }
}
