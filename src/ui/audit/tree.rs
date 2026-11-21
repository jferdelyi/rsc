//! Names and targets (WCAG 4.1.2, 2.5.8): the measure `tree`, a name for each control, the pill's
//! whole name.

use super::*;

/// What is wrong with a target, against the criteria the tree can answer.
fn findings(element: &Element, all: &[Element], window: Rect) -> Vec<String> {
    let mut found = Vec::new();
    if !element.target() {
        return found;
    }
    if element.name.trim().is_empty() && element.value.trim().is_empty() {
        found.push("sans nom (4.1.2)".to_owned());
    }
    if format!("{}{}", element.name, element.value).chars().any(|c| ('\u{E000}'..='\u{F8FF}').contains(&c)) {
        found.push("un glyphe d'icône dans le nom ou la valeur (4.1.2)".to_owned());
    }
    if matches!(element.role, Role::TextInput | Role::MultilineTextInput | Role::SearchInput) && element.name.trim().is_empty() {
        found.push(if element.placeholder.is_empty() { "champ sans libellé (3.3.2)".to_owned() } else { "texte d'exemple seul, sans libellé (3.3.2)".to_owned() });
    }
    if !window.contains_rect(element.rect) {
        found.push("hors de la fenêtre, en tout ou en partie".to_owned());
    }
    // A label is text that can be selected, not a control: 2.5.8 is about controls.
    let control = |element: &Element| element.target() && element.role != Role::Label;
    if control(element) && element.small() {
        let center = element.rect.center();
        let near = all
            .iter()
            .filter(|other| control(other) && other.id != element.id && window.intersects(other.rect))
            .filter(|other| !other.rect.contains_rect(element.rect) && !element.rect.contains_rect(other.rect))
            .find(|other| if other.small() { other.rect.center().distance(center) < 24.0 } else { other.rect.distance_to_pos(center) < 12.0 });
        let size = format!("{:.0}×{:.0}", element.rect.width(), element.rect.height());
        found.push(match near {
            None => format!("cible de {size}, sous 24×24, espacée assez (2.5.8)"),
            Some(other) => format!("cible de {size}, sous 24×24, trop près de « {} » (2.5.8)", other.called()),
        });
    }
    found
}

/// The tree of each screen: `arbre.csv`, and what is wrong in `arbre-constats.csv`.
#[test]
#[ignore = "a measure of accessibility, run by hand"]
fn tree() {
    let header = "ecran;role;nom;valeur;texte d'exemple;x;y;largeur;hauteur;cible;focalisable;constats\n";
    let (mut all_rows, mut found_rows) = (String::from(header), String::from(header));
    let window = Rect::from_min_size(Pos2::ZERO, WINDOW);
    for screen in Screen::all() {
        let (harness, _fake) = open(screen, Theme::Light, WINDOW, 1.0, false);
        let elements = elements(&harness);
        let (mut targets, mut wrong) = (0, 0);
        for element in &elements {
            let found = findings(element, &elements, window);
            targets += usize::from(element.target());
            let row = format!(
                "{};{:?};{};{};{};{:.0};{:.0};{:.0};{:.0};{};{};{}\n",
                screen.name(),
                element.role,
                cell(&element.name),
                cell(&element.value),
                cell(&element.placeholder),
                element.rect.min.x,
                element.rect.min.y,
                element.rect.width(),
                element.rect.height(),
                if element.click { "oui" } else { "non" },
                if element.focus { "oui" } else { "non" },
                cell(&found.join(" · "))
            );
            all_rows.push_str(&row);
            if !found.is_empty() {
                wrong += 1;
                found_rows.push_str(&row);
            }
        }
        println!("{} : {} éléments, {targets} cibles, {wrong} avec un constat", screen.name(), elements.len());
    }
    let folder = folder();
    fs::write(folder.join("arbre.csv"), all_rows).unwrap();
    fs::write(folder.join("arbre-constats.csv"), found_rows).unwrap();
}

/// Each control has a name, no name or value is an icon's glyph, and the pill is there once.
/// Out of reach of RSC: egui's scroll bars, and egui_plot's axes, whose identifiers it has not;
/// the focus ring shows on them.
#[test]
fn every_control_has_a_name() {
    let window = Rect::from_min_size(Pos2::ZERO, WINDOW);
    for screen in Screen::all() {
        let (harness, _fake) = open(screen, Theme::Light, WINDOW, 1.0, false);
        let all = elements(&harness);
        // egui_plot's axes: the one below a chart, as wide as it, and the one at its left, as high.
        let charts: Vec<Rect> = all.iter().filter(|element| element.name.starts_with("Courbes :")).map(|element| element.rect).collect();
        let axis_of = |rect: Rect, chart: &Rect| {
            let below = (rect.left() - chart.left()).abs() < 1.0 && (rect.width() - chart.width()).abs() < 1.0 && (rect.top() - chart.bottom()).abs() < 2.0;
            let left = (rect.top() - chart.top()).abs() < 1.0 && (rect.height() - chart.height()).abs() < 1.0 && (rect.right() - chart.left()).abs() < 2.0;
            below || left
        };
        let mut wrong = Vec::new();
        for element in &all {
            if format!("{}{}", element.name, element.value).chars().any(|c| ('\u{E000}'..='\u{F8FF}').contains(&c)) {
                wrong.push(format!("glyphe : {:?} {}", element.role, element.called()));
            }
            if !element.target() || matches!(element.role, Role::Label | Role::TextRun) || !window.intersects(element.rect) {
                continue;
            }
            let plot_axis = element.role == Role::Unknown && element.name.is_empty() && charts.iter().any(|chart| axis_of(element.rect, chart));
            if element.role == Role::ScrollBar || plot_axis {
                continue;
            }
            if element.name.trim().is_empty() {
                wrong.push(format!("sans nom : {}", element.called()));
            }
        }
        let pills = all.iter().filter(|element| element.name.starts_with("Profil Carte Nano")).count();
        assert_eq!(pills, 1, "{}: the pill, once", screen.name());
        assert!(wrong.is_empty(), "{}: {wrong:#?}", screen.name());
    }
}

/// The pill gives screen readers its whole text, though the bar shows it cut (WCAG 4.1.2): seen cut
/// in the real window, « Profil Périphérique sé… » (18/11/2026).
#[test]
fn the_pill_gives_its_whole_name() {
    let (mut harness, _fake) = open(Screen::Line, Theme::Light, WINDOW, 1.0, false);
    let settings = &mut harness.state_mut().settings;
    settings.rename_profile("Carte Nano n°3", "Carte Nano B100042 rev D").expect("renamed");
    harness.run();
    let whole = "Profil Carte Nano B100042 rev D";
    let named: Vec<String> = elements(&harness).into_iter().filter(|element| element.role == Role::Button && element.name.starts_with("Profil ")).map(|element| element.name).collect();
    assert_eq!(named, vec![whole.to_owned()], "the pill's name");
    assert!(texts(&harness).iter().any(|(_, text, _)| text == "Profil Carte Nano B100…"), "the bar shows the pill cut");
}
