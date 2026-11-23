//! The tool's look: a card on a quiet page, a dark console in it, in a light and a dark theme.

use eframe::egui::{self, Color32, CornerRadius, FontId, Stroke, StrokeKind, TextStyle, Theme, Visuals};

use crate::console::{Color, Kind};
use crate::settings;

/// The console is dark in both themes: it is a terminal.
pub const CONSOLE: Color32 = Color32::from_rgb(0x0D, 0x11, 0x17);
pub const CONSOLE_TEXT: Color32 = Color32::from_rgb(0xD7, 0xDC, 0xE2);
/// A line's time, and the console's hint when it is empty: 4.71:1 on the console (WCAG 1.4.3).
pub const CONSOLE_TIME: Color32 = Color32::from_rgb(0x75, 0x80, 0x8D);
pub const CONSOLE_SENT: Color32 = Color32::from_rgb(0x79, 0xC0, 0xFF);
pub const CONSOLE_NOTICE: Color32 = Color32::from_rgb(0x9A, 0xA4, 0xAF);
pub const CONSOLE_SELECTION: Color32 = Color32::from_rgb(0x26, 0x4F, 0x78);

/// The 16 colours of a terminal, on the console's background: 0 to 7, then their bright forms. Each
/// stands at 4.5:1 at least on the console, but black, which programs write on a background of
/// their own choosing (WCAG 1.4.3).
const ANSI: [Color32; 16] = [
    Color32::from_rgb(0x48, 0x4F, 0x58),
    Color32::from_rgb(0xF8, 0x51, 0x49),
    Color32::from_rgb(0x3F, 0xB9, 0x50),
    Color32::from_rgb(0xD2, 0x99, 0x22),
    Color32::from_rgb(0x58, 0xA6, 0xFF),
    Color32::from_rgb(0xBC, 0x8C, 0xFF),
    Color32::from_rgb(0x39, 0xC5, 0xCF),
    Color32::from_rgb(0xB1, 0xBA, 0xC4),
    Color32::from_rgb(0x77, 0x7E, 0x89),
    Color32::from_rgb(0xFF, 0x7B, 0x72),
    Color32::from_rgb(0x56, 0xD3, 0x64),
    Color32::from_rgb(0xE3, 0xB3, 0x41),
    Color32::from_rgb(0x79, 0xC0, 0xFF),
    Color32::from_rgb(0xD2, 0xA8, 0xFF),
    Color32::from_rgb(0x56, 0xD4, 0xDD),
    Color32::from_rgb(0xF0, 0xF6, 0xFC),
];

/// The colour a piece of a line is written in. Without `ansi`, what the device sends is all of
/// one colour.
pub fn ink(kind: Kind, color: Color, ansi: bool) -> Color32 {
    match (kind, color) {
        (Kind::Sent, _) => CONSOLE_SENT,
        (Kind::Notice, _) => CONSOLE_NOTICE,
        (Kind::Received, Color::Ansi(index)) if ansi => ANSI[usize::from(index) % ANSI.len()],
        (Kind::Received, _) => CONSOLE_TEXT,
    }
}

/// A colour of the terminal mode: the console's 16, the 256-colour cube and its greys as xterm
/// makes them, or the colour itself; `default` for the terminal's own.
pub fn terminal_color(color: vt100::Color, default: Color32) -> Color32 {
    match color {
        vt100::Color::Default => default,
        vt100::Color::Idx(index @ 0..=15) => ANSI[usize::from(index)],
        vt100::Color::Idx(index @ 16..=231) => {
            let cube = index - 16;
            let level = |step: u8| if step == 0 { 0 } else { 55 + step * 40 };
            Color32::from_rgb(level(cube / 36), level(cube / 6 % 6), level(cube % 6))
        }
        vt100::Color::Idx(index) => {
            let grey = 8 + (index - 232) * 10;
            Color32::from_rgb(grey, grey, grey)
        }
        vt100::Color::Rgb(red, green, blue) => Color32::from_rgb(red, green, blue),
    }
}

/// The colour of a curve, by its place: the console's bright colours that read best on it.
pub fn curve(index: usize) -> Color32 {
    const ORDER: [usize; 6] = [12, 10, 11, 13, 14, 9];
    ANSI[ORDER[index % ORDER.len()]]
}

/// What a state or a message says.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tone {
    Neutral,
    Busy,
    Good,
    Bad,
}

/// The colours of the window around the console.
pub struct Palette {
    pub page: Color32,
    pub card: Color32,
    /// The edge of the card and of the separations.
    pub border: Color32,
    /// The edge of a control (list, field, box, choice in segments): 3:1 against the card and a
    /// field (WCAG 1.4.11).
    pub control: Color32,
    /// The track of a switch turned off: 3:1 against the card.
    pub switch_off: Color32,
    pub text: Color32,
    pub muted: Color32,
    pub field: Color32,
    /// The main button of a row, and what is written on it.
    pub primary: Color32,
    pub on_primary: Color32,
    /// The other buttons.
    pub secondary: Color32,
    pub on_secondary: Color32,
    pub accent: Color32,
}

impl Palette {
    pub fn of(dark: bool) -> Self {
        if dark {
            Self {
                page: Color32::from_rgb(0x0A, 0x0C, 0x10),
                card: Color32::from_rgb(0x16, 0x1A, 0x20),
                border: Color32::from_rgb(0x2B, 0x31, 0x3A),
                control: Color32::from_rgb(0x62, 0x67, 0x6D),
                switch_off: Color32::from_gray(103),
                text: Color32::from_rgb(0xE6, 0xEA, 0xEF),
                muted: Color32::from_rgb(0x8C, 0x96, 0xA2),
                field: Color32::from_rgb(0x0F, 0x12, 0x17),
                primary: Color32::from_rgb(0xF0, 0xF3, 0xF6),
                on_primary: Color32::from_rgb(0x0B, 0x0D, 0x10),
                secondary: Color32::from_rgb(0x2A, 0x30, 0x39),
                on_secondary: Color32::from_rgb(0xE6, 0xEA, 0xEF),
                accent: Color32::from_rgb(0x58, 0xA6, 0xFF),
            }
        } else {
            Self {
                page: Color32::from_rgb(0xF1, 0xF2, 0xF4),
                card: Color32::WHITE,
                border: Color32::from_rgb(0xDD, 0xE1, 0xE6),
                control: Color32::from_rgb(0x8B, 0x8E, 0x91),
                switch_off: Color32::from_gray(148),
                text: Color32::from_rgb(0x1B, 0x1F, 0x24),
                muted: Color32::from_rgb(0x5F, 0x68, 0x73),
                field: Color32::from_rgb(0xF6, 0xF7, 0xF9),
                primary: Color32::from_rgb(0x0B, 0x0D, 0x10),
                on_primary: Color32::WHITE,
                secondary: Color32::from_rgb(0x16, 0x1B, 0x22),
                on_secondary: Color32::WHITE,
                accent: Color32::from_rgb(0x1F, 0x6F, 0xD1),
            }
        }
    }

    pub fn tone(&self, tone: Tone, dark: bool) -> Color32 {
        match tone {
            Tone::Neutral => self.muted,
            Tone::Busy if dark => Color32::from_rgb(0xE3, 0xB3, 0x41),
            Tone::Busy => Color32::from_rgb(0x9A, 0x67, 0x00),
            Tone::Good if dark => Color32::from_rgb(0x3F, 0xB9, 0x50),
            Tone::Good => Color32::from_rgb(0x1A, 0x7F, 0x37),
            Tone::Bad if dark => Color32::from_rgb(0xFF, 0x7B, 0x72),
            Tone::Bad => Color32::from_rgb(0xCF, 0x22, 0x2E),
        }
    }
}

/// The height of a control: fields, selectors and buttons line up on it.
pub const CONTROL_HEIGHT: f32 = 32.0;

/// The width of the focus ring, and its gap to the widget.
pub const FOCUS_RING: f32 = 2.0;

/// The focus ring, around what shows of the widget that has the keyboard's focus (the console
/// scrolls), on its layer: the same for every control, those egui and egui_plot draw included
/// (WCAG 2.4.7). Its accent stands at 3:1 at least against the card, the page, a field and the
/// console.
pub fn focus_ring(ctx: &egui::Context) {
    let Some(response) = ctx.memory(|memory| memory.focused()).and_then(|id| ctx.read_response(id)) else {
        return;
    };
    let accent = Palette::of(ctx.global_style().visuals.dark_mode).accent;
    ctx.layer_painter(response.layer_id).rect_stroke(response.interact_rect.expand(FOCUS_RING), CornerRadius::same(6), Stroke::new(FOCUS_RING, accent), StrokeKind::Outside);
}

/// Spacing, text sizes and colours, for the light and the dark theme alike.
pub fn install(ctx: &egui::Context) {
    for theme in [Theme::Dark, Theme::Light] {
        ctx.style_mut_of(theme, |style| {
            style.spacing.item_spacing = egui::vec2(8.0, 8.0);
            style.spacing.button_padding = egui::vec2(12.0, 6.0);
            style.spacing.interact_size.y = CONTROL_HEIGHT;
            // A box, not a circle: a check box must not read as a radio button.
            style.spacing.icon_width = 16.0;
            style.spacing.icon_width_inner = 10.0;
            style.text_styles = [
                (TextStyle::Heading, FontId::proportional(16.0)),
                (TextStyle::Body, FontId::proportional(14.0)),
                (TextStyle::Button, FontId::proportional(14.0)),
                (TextStyle::Small, FontId::proportional(12.0)),
                (TextStyle::Monospace, FontId::monospace(13.0)),
            ]
            .into();
            paint(&mut style.visuals, theme == Theme::Dark);
        });
    }
    ctx.set_fonts(super::icons::fonts());
}

/// The theme the settings ask for: the system's is followed when it changes.
pub fn preference(theme: settings::Theme) -> egui::ThemePreference {
    match theme {
        settings::Theme::System => egui::ThemePreference::System,
        settings::Theme::Light => egui::ThemePreference::Light,
        settings::Theme::Dark => egui::ThemePreference::Dark,
    }
}

fn paint(visuals: &mut Visuals, dark: bool) {
    let palette = Palette::of(dark);
    visuals.panel_fill = palette.page;
    visuals.window_fill = palette.card;
    visuals.window_stroke = Stroke::new(1.0_f32, palette.border);
    visuals.window_corner_radius = CornerRadius::same(14);
    visuals.extreme_bg_color = palette.field;
    visuals.hyperlink_color = palette.accent;
    visuals.selection.bg_fill = palette.accent.gamma_multiply(if dark { 0.55 } else { 0.25 });
    visuals.selection.stroke = Stroke::new(1.0_f32, palette.text);
    visuals.error_fg_color = palette.tone(Tone::Bad, dark);
    visuals.warn_fg_color = palette.tone(Tone::Busy, dark);
    for widget in [
        &mut visuals.widgets.noninteractive,
        &mut visuals.widgets.inactive,
        &mut visuals.widgets.hovered,
        &mut visuals.widgets.active,
        &mut visuals.widgets.open,
    ] {
        widget.corner_radius = CornerRadius::same(4);
        widget.fg_stroke.color = palette.text;
        widget.weak_bg_fill = palette.field;
        widget.bg_fill = palette.field;
        widget.bg_stroke = Stroke::new(1.0_f32, palette.border);
    }
    visuals.widgets.noninteractive.bg_fill = palette.card;
    visuals.widgets.noninteractive.weak_bg_fill = palette.card;
    // A control at rest is told by its edge: its fill is too close to the card's (WCAG 1.4.11).
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, palette.control);
    // A field's hint: the muted text, 5.27:1 and 6.25:1 on a field, where egui's text at 60 %
    // stood at 4.27:1 in the light theme (WCAG 1.4.3).
    visuals.weak_text_color = Some(palette.muted);
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, palette.accent);
    visuals.widgets.active.bg_stroke = Stroke::new(1.0_f32, palette.accent);
    visuals.widgets.open.bg_stroke = Stroke::new(1.0_f32, palette.accent);
}

/// The main button of a row, a filled pill of a fixed width.
pub fn primary_button(ui: &mut egui::Ui, text: &str, width: f32, enabled: bool) -> egui::Response {
    let palette = Palette::of(ui.visuals().dark_mode);
    let button = egui::Button::new(egui::RichText::new(text).color(palette.on_primary)).corner_radius(CornerRadius::same(16)).min_size(egui::vec2(width, CONTROL_HEIGHT));
    filled_button(ui, button, palette.primary, enabled)
}

/// Any other button: filled, quieter than the main one.
pub fn secondary_button(ui: &mut egui::Ui, text: &str, enabled: bool) -> egui::Response {
    let palette = Palette::of(ui.visuals().dark_mode);
    let button = egui::Button::new(egui::RichText::new(text).color(palette.on_secondary)).corner_radius(CornerRadius::same(8)).min_size(egui::vec2(0.0, CONTROL_HEIGHT));
    filled_button(ui, button, palette.secondary, enabled)
}

/// A filled button that answers the pointer: lighter under it, darker and a little sunk while
/// pressed. The button sets no fill of its own: egui takes it from these states.
pub fn filled_button(ui: &mut egui::Ui, button: egui::Button<'_>, fill: Color32, enabled: bool) -> egui::Response {
    let accent = Palette::of(ui.visuals().dark_mode).accent;
    let [rest, hover, press] = button_fills(fill);
    ui.scope(|ui| {
        let widgets = &mut ui.visuals_mut().widgets;
        widgets.inactive.weak_bg_fill = rest;
        widgets.inactive.bg_stroke = Stroke::NONE;
        widgets.hovered.weak_bg_fill = hover;
        widgets.hovered.bg_stroke = Stroke::NONE;
        widgets.hovered.expansion = 0.0;
        widgets.active.weak_bg_fill = press;
        widgets.active.bg_stroke = Stroke::new(1.0_f32, accent);
        widgets.active.expansion = -1.0;
        ui.add_enabled(enabled, button)
    })
    .inner
}

/// A filled button's colour at rest, under the pointer and pressed: towards white for a dark
/// colour, towards black for a light one. Pressed is also the look of the focus: at 24 %, its text
/// keeps 4.5:1 (32 % left the dark theme's secondary buttons at 4.00:1, WCAG 1.4.3).
pub fn button_fills(fill: Color32) -> [Color32; 3] {
    let luma = (u32::from(fill.r()) * 299 + u32::from(fill.g()) * 587 + u32::from(fill.b()) * 114) / 1000;
    let toward = if luma > 140 { Color32::BLACK } else { Color32::WHITE };
    [fill, mix(fill, toward, 0.16), mix(fill, toward, 0.24)]
}

/// Two colours mixed: `how_far` of the way from `from` to `to`.
pub fn mix(from: Color32, to: Color32, how_far: f32) -> Color32 {
    let channel = |a: u8, b: u8| (f32::from(a) + (f32::from(b) - f32::from(a)) * how_far).round() as u8;
    Color32::from_rgb(channel(from.r(), to.r()), channel(from.g(), to.g()), channel(from.b(), to.b()))
}

/// RSC's logo as the window's icon, drawn by `assets/make_icons.py`. Under macOS, the Dock shows it
/// while RSC runs: there it is the logo on Apple's grid, with the margin every macOS icon keeps around
/// its tile, as in `RSC.app`.
#[cfg(not(target_os = "macos"))]
const WINDOW_ICON: &[u8] = include_bytes!("../../assets/rsc-256.png");
#[cfg(target_os = "macos")]
const WINDOW_ICON: &[u8] = include_bytes!("../../assets/rsc-macos-512.png");

/// The logo as the window's icon.
pub fn icon() -> egui::IconData {
    eframe::icon_data::from_png_bytes(WINDOW_ICON).unwrap_or_default()
}

/// The small mark of a state, in front of a tab, a status or a device: its shape tells the state as
/// its colour does, so that the colour is not alone (WCAG 1.4.1). A disc: open, there; a ring:
/// closed, absent; three quarters of a ring: under way; a cross: a problem.
pub fn mark(ui: &mut egui::Ui, tone: Tone, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(10.0, 10.0), egui::Sense::hover());
    let painter = ui.painter();
    let center = rect.center();
    let stroke = Stroke::new(1.5, color);
    match tone {
        Tone::Good => {
            painter.circle_filled(center, 4.0, color);
        }
        Tone::Neutral => {
            painter.circle_stroke(center, 3.5, stroke);
        }
        Tone::Busy => {
            let quarters = 1.5 * std::f32::consts::PI;
            let arc = (0..=18).map(|n| center + 3.5 * egui::Vec2::angled(-std::f32::consts::FRAC_PI_2 + quarters * n as f32 / 18.0)).collect();
            painter.add(egui::Shape::line(arc, stroke));
        }
        Tone::Bad => {
            let stroke = Stroke::new(2.0, color);
            painter.line_segment([center + egui::vec2(-3.5, -3.5), center + egui::vec2(3.5, 3.5)], stroke);
            painter.line_segment([center + egui::vec2(-3.5, 3.5), center + egui::vec2(3.5, -3.5)], stroke);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_button_shows_the_pointer_over_it_and_the_press() {
        for dark in [false, true] {
            let palette = Palette::of(dark);
            for fill in [palette.primary, palette.secondary] {
                let [rest, hover, press] = button_fills(fill);
                assert!(rest != hover && hover != press && rest != press, "{fill:?}: three looks");
                // Each step further from the colour at rest, in the same direction.
                let distance = |color: Color32| (i32::from(color.r()) - i32::from(rest.r())).abs();
                assert!(distance(press) > distance(hover), "{fill:?}: pressed, further than hovered");
            }
        }
    }

    #[test]
    fn the_window_icon_is_the_one_of_its_system() {
        // Under macOS, the Dock shows the window's icon while RSC runs: there, the logo on Apple's grid.
        let side = if cfg!(target_os = "macos") { 512 } else { 256 };
        let icon = icon();
        assert_eq!((icon.width, icon.height), (side, side));
    }
}
