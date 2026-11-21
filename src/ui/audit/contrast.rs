//! Contrast (WCAG 1.4.3, 1.4.11): the formula, every pair of colours the window draws, the
//! measure `contrast`, and the tests that keep the pairs at AA.

use super::*;

/// What a pair of colours must reach.
#[derive(Clone, Copy)]
enum Need {
    /// Text: 4.5:1 (1.4.3, AA). RSC writes at 12 to 16 px, under the 24 px (or 18.7 px bold) of
    /// large text.
    Text,
    /// A component's edge or state, a focus mark, a graphic that carries information: 3:1
    /// (1.4.11, AA).
    Graphic,
    /// No threshold: read beside the others (a button's fill, when its text names it).
    Info,
}

impl Need {
    fn least(self) -> Option<f64> {
        match self {
            Need::Text => Some(4.5),
            Need::Graphic => Some(3.0),
            Need::Info => None,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Need::Text => "texte (1.4.3)",
            Need::Graphic => "composant ou graphique (1.4.11)",
            Need::Info => "information",
        }
    }
}

/// Two colours drawn one on the other, and where.
struct Pair {
    theme: &'static str,
    place: String,
    front: Color32,
    back: Color32,
    need: Need,
}

/// A colour with transparency, as egui keeps it (premultiplied), laid on an opaque one.
fn over(top: Color32, below: Color32) -> Color32 {
    let rest = 1.0 - f32::from(top.a()) / 255.0;
    let channel = |t: u8, b: u8| (f32::from(t) + f32::from(b) * rest).round().min(255.0) as u8;
    Color32::from_rgb(channel(top.r(), below.r()), channel(top.g(), below.g()), channel(top.b(), below.b()))
}

fn hex(color: Color32) -> String {
    format!("#{:02X}{:02X}{:02X}", color.r(), color.g(), color.b())
}

/// Every pair of colours the window draws, read in theme.rs, widgets.rs and the views, and in
/// egui's own drawing where RSC leaves it to egui.
fn pairs() -> Vec<Pair> {
    use Need::{Graphic, Info, Text};
    const TONES: [(Tone, &str); 4] = [(Tone::Neutral, "neutre"), (Tone::Busy, "en cours"), (Tone::Good, "bon"), (Tone::Bad, "erreur")];
    const ANSI: [&str; 8] = ["noir", "rouge", "vert", "jaune", "bleu", "magenta", "cyan", "blanc"];
    let mut pairs = Vec::new();
    // egui's colours as the theme sets them, where RSC leaves the drawing to egui.
    let ctx = egui::Context::default();
    theme::install(&ctx);
    for dark in [false, true] {
        let theme = if dark { "sombre" } else { "clair" };
        let p = Palette::of(dark);
        let visuals = ctx.style_of(if dark { egui::Theme::Dark } else { egui::Theme::Light }).visuals.clone();
        let mut add = |place: &str, front: Color32, back: Color32, need: Need| pairs.push(Pair { theme, place: place.to_owned(), front, back, need });
        let selection = |back: Color32| over(visuals.selection.bg_fill, back);
        let control = visuals.widgets.inactive.bg_stroke.color;
        let [primary, primary_hover, primary_press] = theme::button_fills(p.primary);
        let [secondary, secondary_hover, secondary_press] = theme::button_fills(p.secondary);
        // Texts.
        add("texte sur la page (réglages : liste des sections)", p.text, p.page, Text);
        add("texte sur la carte", p.text, p.card, Text);
        add("texte dans un champ", p.text, p.field, Text);
        add("texte grisé sur la page (réglages : aide de la section)", p.muted, p.page, Text);
        add("texte grisé sur la carte (notes, état du journal, menus)", p.muted, p.card, Text);
        add("texte d'exemple d'un champ vide (weak_text_color d'egui)", over(visuals.weak_text_color(), p.field), p.field, Text);
        for (tone, name) in TONES.into_iter().skip(1) {
            add(&format!("ligne d'état « {name} » sur la carte"), p.tone(tone, dark), p.card, Text);
        }
        add("bouton principal : texte au repos", p.on_primary, primary, Text);
        add("bouton principal : texte survolé", p.on_primary, primary_hover, Text);
        add("bouton principal : texte enfoncé ou focalisé", p.on_primary, primary_press, Text);
        add("bouton secondaire : texte au repos", p.on_secondary, secondary, Text);
        add("bouton secondaire : texte survolé", p.on_secondary, secondary_hover, Text);
        add("bouton secondaire : texte enfoncé ou focalisé", p.on_secondary, secondary_press, Text);
        add("onglet choisi : texte sur son fond", p.text, selection(p.card), Text);
        add("texte sélectionné dans un champ", p.text, selection(p.field), Text);
        add("choix segmenté : texte du choix retenu", p.on_primary, p.primary, Text);
        add("choix segmenté : texte des autres choix", p.text, p.field, Text);
        // Components and graphics.
        add("bord d'un contrôle (liste, champ, case) contre la carte", control, p.card, Graphic);
        add("bord d'un contrôle contre son fond", control, p.field, Graphic);
        // Information : the edge tells the field, not its fill.
        add("fond d'un champ contre la carte · information : son bord le distingue", p.field, p.card, Info);
        add("contour de survol (accent) contre la carte", p.accent, p.card, Graphic);
        add("contour de survol (accent) contre un champ", p.accent, p.field, Graphic);
        add("contour de focus de RSC (accent, 2 px) contre la carte", p.accent, p.card, Graphic);
        add("contour de focus de RSC contre la page (réglages : sections)", p.accent, p.page, Graphic);
        add("contour de focus de RSC contre un champ", p.accent, p.field, Graphic);
        add("contour de focus de RSC contre la console, le terminal, les courbes", p.accent, theme::CONSOLE, Graphic);
        // Information: RSC's own focus ring is drawn outside, against the card.
        add("bouton principal enfoncé : contour d'egui contre son fond · information : le contour de focus de RSC est à l'extérieur", p.accent, primary_press, Info);
        add("bouton secondaire enfoncé : contour d'egui contre son fond · information : le contour de focus de RSC est à l'extérieur", p.accent, secondary_press, Info);
        add("bouton icône sans cadre (×, +) : icône grisée sur la carte", p.muted, p.card, Graphic);
        add("icône d'information grisée sur la carte", p.muted, p.card, Graphic);
        add("bouton icône : icône sur son fond", p.on_secondary, secondary, Graphic);
        add("pastille du profil : icône (accent) sur son fond", p.accent, p.field, Graphic);
        add("interrupteur éteint : piste contre la carte", p.switch_off, p.card, Graphic);
        add("interrupteur allumé : piste contre la carte", p.accent, p.card, Graphic);
        add("interrupteur éteint : bouton blanc contre la piste", Color32::WHITE, p.switch_off, Graphic);
        // Information : the state is told by the track against the card.
        add("interrupteur allumé : bouton blanc contre la piste · information : l'état se lit à la piste, contre la carte", Color32::WHITE, p.accent, Info);
        // Information : the line of the accent under it tells the tab chosen.
        add("onglet choisi : son fond contre la carte · information : le trait d'accent le marque", selection(p.card), p.card, Info);
        add("onglet choisi : trait d'accent contre la carte", p.accent, p.card, Graphic);
        add("section choisie des réglages : barre d'accent contre la page", p.accent, p.page, Graphic);
        add("choix segmenté : fond du choix retenu contre le cadre", p.primary, p.field, Graphic);
        add("choix segmenté : bord du cadre contre la carte", control, p.card, Graphic);
        for (tone, name) in TONES {
            add(&format!("point d'état « {name} » (onglet, ligne d'état) sur la carte"), p.tone(tone, dark), p.card, Graphic);
        }
        add("séparation console et courbes : poignée au repos", p.muted, p.card, Graphic);
        add("séparation console et courbes : poignée survolée", p.text, p.card, Graphic);
        add("marque du journal en cours sur son bouton", p.tone(Tone::Bad, dark), secondary, Graphic);
        add("barre du profil changé (réglages) contre la carte", p.accent, p.card, Graphic);
        for index in 0..6 {
            add(&format!("menu « Haut ou bas » : carré de la courbe {} sur sa pastille du fond de la console", index + 1), theme::curve(index), theme::CONSOLE, Graphic);
        }
        add("bouton principal : fond contre la carte (son texte le nomme)", p.primary, p.card, Info);
        add("bouton secondaire : fond contre la carte (son texte ou son icône le nomme)", p.secondary, p.card, Info);
        add("carte contre la page", p.card, p.page, Info);
    }
    // The console, the terminal and the curves are dark in both themes.
    let mut add = |place: &str, front: Color32, back: Color32, need: Need| pairs.push(Pair { theme: "les deux", place: place.to_owned(), front, back, need });
    add("console : texte reçu", theme::CONSOLE_TEXT, theme::CONSOLE, Text);
    add("console : heure d'une ligne, invite d'un écran vide", theme::CONSOLE_TIME, theme::CONSOLE, Text);
    add("console : ligne envoyée", theme::CONSOLE_SENT, theme::CONSOLE, Text);
    add("console : message de RSC", theme::CONSOLE_NOTICE, theme::CONSOLE, Text);
    add("console : texte sur la sélection", theme::CONSOLE_TEXT, theme::CONSOLE_SELECTION, Text);
    // Information : the text selected stays readable, the line above.
    add("console : fond de la sélection contre la console · information : le texte sélectionné reste lisible", theme::CONSOLE_SELECTION, theme::CONSOLE, Info);
    add("terminal : curseur contre le fond", theme::CONSOLE_TEXT, theme::CONSOLE, Graphic);
    for index in 0..16_u8 {
        let name = format!("console : couleur ANSI {index} ({}{})", ANSI[usize::from(index % 8)], if index >= 8 { " vif" } else { "" });
        // Black: the device's program chooses the background it is written on.
        if index == 0 {
            add(&format!("{name} · information : le programme de l'appareil choisit aussi le fond"), theme::ink(Kind::Received, Color::Ansi(index), true), theme::CONSOLE, Info);
        } else {
            add(&name, theme::ink(Kind::Received, Color::Ansi(index), true), theme::CONSOLE, Text);
        }
    }
    // The charts take egui's dark field colour as their background (egui_plot, extreme_bg_color).
    let dark = Palette::of(true);
    add("courbes : titre", theme::CONSOLE_TEXT, theme::CONSOLE, Text);
    add("courbes : « en pause », invite sans mesure", theme::CONSOLE_TIME, theme::CONSOLE, Text);
    add("courbes : graduations à pleine force (egui_plot les estompe quand elles se serrent)", dark.text, dark.field, Text);
    add("courbes : texte de la légende", dark.text, over(dark.field.gamma_multiply(0.75), dark.field), Text);
    for index in 0..6 {
        add(&format!("courbe {} sur le fond du graphique", index + 1), theme::curve(index), dark.field, Graphic);
    }
    pairs
}

/// Every pair drawn, its ratio against its threshold: `contraste.csv`.
#[test]
#[ignore = "a measure of accessibility, run by hand"]
fn contrast() {
    let mut csv = String::from("theme;endroit;avant-plan;fond;ratio;exigence;seuil;verdict\n");
    let (mut count, mut failing) = (0, 0);
    for pair in pairs() {
        let measured = ratio(pair.front, pair.back);
        let verdict = match pair.need.least() {
            Some(least) if measured >= least => "conforme",
            Some(_) => "en défaut",
            None => "information",
        };
        count += 1;
        if verdict == "en défaut" {
            failing += 1;
            println!("{} · {} · {:.2}:1, il faut {}", pair.theme, pair.place, measured, pair.need.name());
        }
        let least = pair.need.least().map_or_else(|| "·".to_owned(), |least| format!("{least}"));
        writeln!(csv, "{};{};{};{};{measured:.2};{};{least};{verdict}", pair.theme, cell(&pair.place), hex(pair.front), hex(pair.back), cell(pair.need.name())).unwrap();
    }
    println!("{count} paires, {failing} en défaut");
    fs::write(folder().join("contraste.csv"), csv).unwrap();
}

/// The formula of the measure, on ratios known from WCAG's own examples.
#[test]
fn the_contrast_formula_gives_the_known_ratios() {
    assert!((ratio(Color32::BLACK, Color32::WHITE) - 21.0).abs() < 1e-9, "black on white: 21:1");
    assert!((ratio(Color32::WHITE, Color32::WHITE) - 1.0).abs() < 1e-9, "the same colour: 1:1");
    // #767676 is the lightest grey that passes 4.5:1 on white (4.54:1); #777777 fails (4.48:1).
    assert!((ratio(Color32::from_gray(0x76), Color32::WHITE) - 4.54).abs() < 0.005);
    assert!(ratio(Color32::from_gray(0x77), Color32::WHITE) < 4.5);
    assert_eq!(over(Color32::from_rgba_premultiplied(0, 0, 0, 0), Color32::WHITE), Color32::WHITE, "transparent: the colour below");
    // A grey of 64 at half opacity (128 of 255), premultiplied: 64 + 255 × 127/255 = 191.
    assert_eq!(over(Color32::from_rgba_premultiplied(64, 64, 64, 128), Color32::WHITE), Color32::from_gray(191), "half grey on white");
}

/// Each pair of colours drawn reaches its threshold (WCAG 2.2 AA, 1.4.3 and 1.4.11).
#[test]
fn the_colours_drawn_reach_wcag_aa() {
    let failing: Vec<String> = pairs()
        .into_iter()
        .filter_map(|pair| {
            let least = pair.need.least()?;
            let measured = ratio(pair.front, pair.back);
            (measured < least).then(|| format!("{} · {} : {measured:.2}:1, il faut {least}:1", pair.theme, pair.place))
        })
        .collect();
    assert!(failing.is_empty(), "{failing:#?}");
}
