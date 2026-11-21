//! The console on screen: only the rows in view are laid out, long lines fold at its width, the
//! text is selected with the mouse character by character, and a minimap at its right shows the
//! whole console and moves through it.

mod input;
mod paint;
mod scene;
#[cfg(test)]
mod tests;

use eframe::egui::accesskit::Role;
use eframe::egui::{self, CornerRadius, FontId, Margin, Pos2, Rect, Sense};

use super::theme;
use super::widgets;
use crate::console::{Console, Position, Shown, Wrap};
use scene::{Scene, read};

/// The room kept at the right of the text for the scroll bar.
const SCROLL_BAR: f32 = 12.0;
/// The minimap's width, and the gap before it.
const MINIMAP: f32 = 72.0;
const MINIMAP_GAP: f32 = 10.0;
/// A row of the console in the minimap, at most: below this, rows share a pixel.
const MINIMAP_ROW: f32 = 3.0;
/// Under this width, the console has no room for its minimap.
const MINIMAP_FROM: f32 = 420.0;

/// How the console is shown.
pub struct Look {
    pub font_size: f32,
    pub shown: Shown,
    pub ansi_colors: bool,
    pub minimap: bool,
}

/// What the user asked of the console in a frame.
#[derive(Default)]
pub struct Asked {
    /// A text to put in the clipboard.
    pub copy: Option<String>,
    pub clear: bool,
    /// What was typed while the console had the focus: it belongs to the send line.
    pub typed: String,
}

pub struct ConsoleView {
    /// Where the selection began, and where it ends now.
    selection: Option<(Position, Position)>,
    /// The console follows its last line: it stops when the user goes up, starts again when they
    /// come back down to the bottom.
    follow: bool,
    /// The console goes down to its last line at the next frame.
    to_bottom: bool,
    /// Where the view was at the last frame: its offset, and whether it was at the bottom.
    last_offset: f32,
    was_at_bottom: bool,
    /// The rows of the console, kept while neither its text, its width nor its look changes.
    wrap: Option<((u64, usize, Shown), Wrap)>,
    /// Where the minimap asked the console to scroll to.
    scroll_to: Option<f32>,
    /// Where a character is drawn: the corner of the text, the size of a cell.
    #[cfg(test)]
    cells: Option<(Pos2, egui::Vec2)>,
}

impl Default for ConsoleView {
    fn default() -> Self {
        Self {
            selection: None,
            follow: true,
            to_bottom: false,
            last_offset: 0.0,
            was_at_bottom: true,
            wrap: None,
            scroll_to: None,
            #[cfg(test)]
            cells: None,
        }
    }
}


/// Whether the console follows its last line after a frame: it stops when the view went up and
/// left the bottom; it starts again when the view came back to the bottom.
fn follow_next(follow: bool, moved_up: bool, was_at_bottom: bool, at_bottom: bool) -> bool {
    if follow { !moved_up || at_bottom } else { !was_at_bottom && at_bottom }
}

impl ConsoleView {
    pub fn follows(&self) -> bool {
        self.follow
    }

    /// Turns the following on or off: on, the console goes down to its last line.
    pub fn set_follow(&mut self, on: bool) {
        if on && !self.follow {
            self.to_bottom = true;
        }
        self.follow = on;
    }

    /// The view at an offset, as the minimap puts it.
    #[cfg(test)]
    pub(crate) fn jump_to(&mut self, offset: f32) {
        self.scroll_to = Some(offset);
    }

    /// The offset of the view at the last frame.
    #[cfg(test)]
    pub(crate) fn offset(&self) -> f32 {
        self.last_offset
    }

    pub fn clear_selection(&mut self) {
        self.selection = None;
    }

    pub fn select_all(&mut self, console: &Console, shown: Shown) {
        self.selection = Some((Position { line: console.first(), col: 0 }, console.end(shown)));
    }

    /// The text selected, if any.
    pub fn selected_text(&self, console: &Console, shown: Shown) -> Option<String> {
        let (a, b) = self.selection?;
        (a != b).then(|| console.text_between(a, b, shown))
    }

    /// The point on screen between the characters `col - 1` and `col` of a row, as of the last
    /// frame: where a drag that selects from `col` begins.
    #[cfg(test)]
    pub(crate) fn gap(&self, row: usize, col: usize) -> Pos2 {
        let (corner, size) = self.cells.expect("the console was drawn");
        corner + egui::vec2(col as f32 * size.x + 1.0, (row as f32 + 0.5) * size.y)
    }

    /// Draws the console in what is left of `ui`. `hint` is what an empty console says.
    pub fn show(&mut self, ui: &mut egui::Ui, console: &Console, look: &Look, hint: &str) -> Asked {
        let mut asked = Asked::default();
        let font = FontId::monospace(look.font_size);
        let (char_width, row_height) = ui.ctx().fonts_mut(|fonts| (fonts.glyph_width(&font, 'M'), fonts.row_height(&font)));
        egui::Frame::new().fill(theme::CONSOLE).corner_radius(CornerRadius::same(10)).inner_margin(Margin::symmetric(14, 12)).show(ui, |ui| {
            // The scroll bar in the dark theme's colours, whatever the window's.
            ui.style_mut().visuals = ui.ctx().style_of(egui::Theme::Dark).visuals.clone();
            ui.set_min_size(ui.available_size());
            let area = ui.available_rect_before_wrap();
            let minimap = look.minimap && area.width() >= MINIMAP_FROM;
            let text_area = if minimap { Rect::from_min_max(area.min, Pos2::new(area.right() - MINIMAP - MINIMAP_GAP, area.bottom())) } else { area };
            let cols = (((text_area.width() - SCROLL_BAR) / char_width).floor() as usize).max(1);
            let key = (console.revision(), cols, look.shown);
            // Folded again only where the console changed; from the start when its width or what it
            // shows changes.
            let wrap = match self.wrap.take() {
                Some((kept, wrap)) if kept == key => wrap,
                Some((kept, mut wrap)) if (kept.1, kept.2) == (key.1, key.2) => {
                    wrap.update(console, look.shown);
                    wrap
                }
                _ => Wrap::of(console, look.shown, cols),
            };
            // A jump (the minimap) is not stuck to the bottom in its frame: egui would put the view
            // back at the last row, and the following would never stop.
            let jump = self.scroll_to.take();
            let mut scroll = egui::ScrollArea::vertical().id_salt("console").auto_shrink([false, false]).stick_to_bottom(self.follow && jump.is_none());
            if std::mem::take(&mut self.to_bottom) {
                // Past the end: the scroll area brings it back to its last row.
                scroll = scroll.vertical_scroll_offset(wrap.rows() as f32 * row_height + text_area.height());
            } else if let Some(offset) = jump {
                scroll = scroll.vertical_scroll_offset(offset);
            }
            let mut drawn_at = 0.0;
            let output = ui
                .scope_builder(egui::UiBuilder::new().max_rect(text_area), |ui| {
                    scroll.show_viewport(ui, |ui, viewport| {
                        drawn_at = viewport.top();
                        let size = egui::vec2(ui.available_width(), (wrap.rows() as f32 * row_height).max(viewport.height()));
                        let (rect, response) = ui.allocate_exact_size(size, Sense::click_and_drag());
                        super::widgets::name(&response, t!("Console", "Console"));
                        let scene = Scene { console, wrap: &wrap, look, rect, char_width, row_height };
                        #[cfg(test)]
                        {
                            self.cells = Some((rect.min, egui::vec2(char_width, row_height)));
                        }
                        // A page of the keyboard: the view less a row, kept from the page before.
                        self.interact(ui, &response, &scene, viewport.height() - row_height, &mut asked);
                        self.paint(ui, &scene, &font);
                        if console.is_empty() {
                            ui.painter().text(rect.min, egui::Align2::LEFT_TOP, hint, font.clone(), theme::CONSOLE_TIME);
                        }
                        // To screen readers, a log and the lines in view (WCAG 1.3.1, 4.1.2).
                        if super::widgets::text_area(&response, rect.intersect(ui.clip_rect()), Role::Log, console.is_empty().then_some(hint)) {
                            read(ui, &response, &scene);
                        }
                    })
                })
                .inner;
            let offset = output.state.offset.y;
            // Following its end, the scroll area moves once the rows are drawn, and asks for no
            // other frame (egui 0.36.2, scroll_area.rs:1282): the last lines would show, and reach
            // screen readers, only at the next event. They are drawn at once.
            if (offset - drawn_at).abs() > 0.5 {
                ui.ctx().request_repaint();
            }
            let at_bottom = offset + output.inner_rect.height() >= output.content_size.y - 2.0;
            self.follow = follow_next(self.follow, offset < self.last_offset - 0.5, self.was_at_bottom, at_bottom);
            self.last_offset = offset;
            self.was_at_bottom = at_bottom;
            if minimap {
                let strip = Rect::from_min_max(Pos2::new(area.right() - MINIMAP, area.top()), area.max);
                let view = (output.state.offset.y / row_height, output.inner_rect.height() / row_height);
                self.minimap(ui, &Scene { console, wrap: &wrap, look, rect: strip, char_width, row_height }, view);
            }
            self.wrap = Some((key, wrap));
        });
        asked
    }
}
