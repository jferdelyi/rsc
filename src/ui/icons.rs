//! The icons of the window: Lucide's, as a font, each called by its name. Their characters are those
//! of `font/codepoints.json` in lucide-static 1.74.0, the package the font comes from
//! (`assets/fonts/README.md`).

use std::sync::Arc;

use eframe::egui::{self, FontData, FontDefinitions, FontFamily, FontId, RichText};

/// The family of the icons in egui.
const FAMILY: &str = "lucide";

/// An icon: its name in Lucide, and its character in the font.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Icon {
    pub name: &'static str,
    pub glyph: char,
}

impl Icon {
    /// The icon as a text of its own size.
    pub fn text(self, size: f32) -> RichText {
        RichText::new(self.glyph.to_string()).font(FontId::new(size, FontFamily::Name(FAMILY.into())))
    }
}

pub const CHART_LINE: Icon = Icon { name: "chart-line", glyph: '\u{e2a5}' };
pub const CHEVRON_LEFT: Icon = Icon { name: "chevron-left", glyph: '\u{e06e}' };
pub const CHEVRON_RIGHT: Icon = Icon { name: "chevron-right", glyph: '\u{e06f}' };
pub const COPY: Icon = Icon { name: "copy", glyph: '\u{e09e}' };
pub const CPU: Icon = Icon { name: "cpu", glyph: '\u{e0a9}' };
pub const ERASER: Icon = Icon { name: "eraser", glyph: '\u{e28f}' };
pub const FILE_INPUT: Icon = Icon { name: "file-input", glyph: '\u{e0c5}' };
pub const FILE_TEXT: Icon = Icon { name: "file-text", glyph: '\u{e0cc}' };
pub const INFO: Icon = Icon { name: "info", glyph: '\u{e0f9}' };
pub const PALETTE: Icon = Icon { name: "palette", glyph: '\u{e1dd}' };
pub const PAUSE: Icon = Icon { name: "pause", glyph: '\u{e12e}' };
pub const PENCIL: Icon = Icon { name: "pencil", glyph: '\u{e1f9}' };
pub const PLAY: Icon = Icon { name: "play", glyph: '\u{e13c}' };
pub const PLUG: Icon = Icon { name: "plug", glyph: '\u{e37f}' };
pub const PLUS: Icon = Icon { name: "plus", glyph: '\u{e13d}' };
pub const REFRESH: Icon = Icon { name: "refresh-cw", glyph: '\u{e145}' };
pub const ROWS: Icon = Icon { name: "rows-2", glyph: '\u{e439}' };
pub const SEND: Icon = Icon { name: "send", glyph: '\u{e152}' };
pub const SETTINGS: Icon = Icon { name: "settings", glyph: '\u{e154}' };
pub const SLIDERS: Icon = Icon { name: "sliders-horizontal", glyph: '\u{e29a}' };
pub const TAG: Icon = Icon { name: "tag", glyph: '\u{e17f}' };
pub const TERMINAL: Icon = Icon { name: "terminal", glyph: '\u{e181}' };
pub const TRASH: Icon = Icon { name: "trash-2", glyph: '\u{e18e}' };
pub const UPLOAD: Icon = Icon { name: "upload", glyph: '\u{e19e}' };
pub const X: Icon = Icon { name: "x", glyph: '\u{e1b2}' };
pub const ZOOM_IN: Icon = Icon { name: "zoom-in", glyph: '\u{e1b6}' };
pub const ZOOM_OUT: Icon = Icon { name: "zoom-out", glyph: '\u{e1b7}' };

/// Every icon the window uses.
#[cfg(test)]
pub const ALL: [Icon; 27] = [
    CHART_LINE, CHEVRON_LEFT, CHEVRON_RIGHT, COPY, CPU, ERASER, FILE_INPUT, FILE_TEXT, INFO, PALETTE, PAUSE, PENCIL, PLAY, PLUG, PLUS, REFRESH,
    ROWS, SEND, SETTINGS, SLIDERS, TAG, TERMINAL, TRASH, UPLOAD, X, ZOOM_IN, ZOOM_OUT,
];

/// The font of the icons, beside egui's own.
pub fn add_to(fonts: &mut FontDefinitions) {
    fonts.font_data.insert(FAMILY.into(), Arc::new(FontData::from_static(include_bytes!("../../assets/fonts/lucide.ttf"))));
    fonts.families.insert(FontFamily::Name(FAMILY.into()), vec![FAMILY.into()]);
}

/// egui's fonts and the icons'.
pub fn fonts() -> FontDefinitions {
    let mut fonts = egui::FontDefinitions::default();
    add_to(&mut fonts);
    fonts
}

#[cfg(test)]
mod tests {
    use ab_glyph::Font;

    use super::*;

    #[test]
    fn every_icon_is_in_the_font_once() {
        let font = ab_glyph::FontRef::try_from_slice(include_bytes!("../../assets/fonts/lucide.ttf")).expect("the font reads");
        let missing: Vec<&str> = ALL.iter().filter(|icon| font.glyph_id(icon.glyph).0 == 0).map(|icon| icon.name).collect();
        assert!(missing.is_empty(), "not in the font: {missing:?}");
        let mut glyphs: Vec<char> = ALL.iter().map(|icon| icon.glyph).collect();
        glyphs.sort_unstable();
        glyphs.dedup();
        assert_eq!(glyphs.len(), ALL.len(), "two names, one glyph");
        // The check fails where it must: a character the font does not have.
        assert_eq!(font.glyph_id('\u{10fffd}').0, 0);
    }
}
