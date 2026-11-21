//! Colour alone, drags and labels (WCAG 1.4.1, 2.5.7, 3.3.2): a tab's state and each curve told
//! without colour, the separation and the paused curves moved without a drag, the fields' labels.

use super::*;

/// What is painted in `area` of the last frame, by kind: the shape of a mark.
fn marks_in(harness: &Harness<'_, App>, area: Rect) -> Vec<&'static str> {
    fn look(shape: &egui::Shape, area: Rect, found: &mut Vec<&'static str>) {
        match shape {
            egui::Shape::Vec(shapes) => shapes.iter().for_each(|shape| look(shape, area, found)),
            egui::Shape::Circle(circle) if area.contains(circle.center) => found.push(if circle.fill.a() > 0 { "disque" } else { "anneau" }),
            egui::Shape::LineSegment { points, .. } if area.contains(points[0]) => found.push("trait"),
            egui::Shape::Path(path) if path.points.first().is_some_and(|point| area.contains(*point)) => found.push("chemin"),
            _ => {}
        }
    }
    let mut found = Vec::new();
    for clipped in &harness.output().shapes {
        look(&clipped.shape, area, &mut found);
    }
    found.sort_unstable();
    found
}

/// Two tabs more, one waiting for its lost port, one closed on a problem: a tab in each state.
fn four_states(harness: &mut Harness<'static, App>) {
    let ctx = harness.ctx.clone();
    for (port, state, problem) in [("COM7", State::Waiting { port: "COM7".into(), tried: None }, None), ("COM9", State::Closed, Some("COM9 : accès refusé"))] {
        let app = harness.state_mut();
        app.add_tab(&ctx);
        let i = app.sessions.len() - 1;
        app.settings.tabs[i].port = port.into();
        app.sessions[i].state = state;
        app.sessions[i].problem = problem.map(str::to_owned);
    }
    harness.state_mut().settings.active = 0;
    harness.run();
}

/// Each curve is told by its name, not only by its colour (WCAG 1.4.1): besides the legend, its name
/// stands at the end of its line, in each chart, running or paused, with room or at the least size.
#[test]
fn every_curve_can_be_told_without_its_colour() {
    for (size, paused) in [(WINDOW, false), (WINDOW, true), (LEAST, false)] {
        let (mut harness, _fake) = open(Screen::Curves, Theme::Light, size, 1.0, false);
        harness.state_mut().settings.edit_tab_profile(0, |profile| profile.plot_below = vec!["hum".into()]);
        harness.run();
        if paused {
            harness.get_by_label("Pause").click();
            harness.run();
        }
        let charts: Vec<String> = elements(&harness).into_iter().filter(|element| element.name.starts_with("Courbes :")).map(|element| element.name).collect();
        assert_eq!(charts.len(), 2, "two charts");
        for name in charts {
            harness.get_by(|node| node.label().as_deref() == Some(name.as_str())).scroll_to_me();
            harness.run();
            let chart = elements(&harness).into_iter().find(|element| element.name == name).expect("the chart").rect;
            let painted = texts(&harness);
            for curve in name.trim_start_matches("Courbes : ").split(", ") {
                let times = painted.iter().filter(|(rect, text, _)| text == curve && chart.contains(rect.center())).count();
                assert!(times >= 2, "{size:?}, paused {paused}: « {curve} » written {times} time(s) in its chart, its legend's alone");
            }
        }
    }
}

/// The curves' names keep clear of the legend, a few measures in as when the curves fill the part
/// shown: seen over the legend in the real window, a few measures in (19/11/2026).
#[test]
fn the_curves_names_keep_clear_of_the_legend() {
    for many in [false, true] {
        let (mut harness, fake) = open(Screen::Curves, Theme::Light, WINDOW, 1.0, false);
        harness.state_mut().sessions[0].plot.clear();
        let count = if many { 120 } else { 5 };
        for n in 0..count {
            fake.device_sends(format!("temperature={} humidite={}\r\n", 40.0 - 0.01 * n as f64, 10.0 + 0.01 * n as f64).as_bytes());
            until(&mut harness, "measure", |app| app.sessions[0].plot.curves().first().is_some_and(|curve| curve.points().len() == n + 1));
        }
        if many {
            harness.state_mut().settings.edit_tab_profile(0, |profile| profile.plot_x = crate::plot::XAxis::Number);
            harness.state_mut().sessions[0].plot_view.span = super::super::plot_view::Span::Short;
        }
        harness.run();
        let legend: Vec<Rect> = elements(&harness).into_iter().filter(|element| element.role == Role::CheckBox && ["temperature", "humidite"].contains(&element.name.as_str())).map(|element| element.rect).collect();
        assert_eq!(legend.len(), 2, "the legend's two entries");
        for name in ["temperature", "humidite"] {
            // The legend's own text, a tenth of a point past its entry, is not a name at an end.
            let names: Vec<Rect> = texts(&harness).into_iter().filter(|(rect, text, _)| text == name && !legend.iter().any(|entry| entry.expand(1.0).contains_rect(*rect))).map(|(rect, _, _)| rect).collect();
            assert!(!names.is_empty(), "{count} measures: « {name} » not written at its end");
            for rect in names {
                assert!(legend.iter().all(|entry| !entry.intersects(rect)), "{count} measures: « {name} » at {rect:?} over the legend {legend:?}");
            }
        }
    }
}

/// The send line and the search of « À propos » keep a visible label once text is typed in them
/// (WCAG 3.3.2): a word before each, which begins its name (2.5.3).
#[test]
fn the_send_line_and_the_search_keep_a_visible_label() {
    let labelled = |harness: &Harness<'static, App>, field: &str, word: &str| {
        let rect = elements(harness).into_iter().find(|element| element.role == Role::TextInput && element.name == field).unwrap_or_else(|| panic!("the field « {field} »")).rect;
        field.starts_with(word) && texts(harness).iter().any(|(at, text, _)| text == word && at.right() <= rect.left() + 1.0 && at.right() >= rect.left() - 40.0 && rect.y_range().contains(at.center().y))
    };
    let (mut harness, _fake) = open(Screen::Line, Theme::Light, WINDOW, 1.0, false);
    harness.state_mut().sessions[0].input = "AT+GMR".into();
    harness.run();
    assert!(labelled(&harness, "Ligne à envoyer", "Ligne"), "no « Ligne » before the send line, a line typed");
    harness.state_mut().settings.edit_tab_profile(0, |profile| profile.hex_input = true);
    harness.run();
    assert!(labelled(&harness, "Octets à envoyer", "Octets"), "no « Octets » before the send line in hexadecimal");
    let (mut harness, _fake) = open(Screen::Settings(Section::About), Theme::Light, WINDOW, 1.0, false);
    if let Some(window) = harness.state_mut().window.as_mut() {
        window.about_search = "serial".into();
    }
    harness.run();
    assert!(labelled(&harness, "Chercher un composant ou une licence", "Chercher"), "no « Chercher » before the search, a name typed");
}

/// Each tab says its state without colour (WCAG 1.4.1): to screen readers, in its description, in
/// the words of the state line; on screen, by the shape of its mark, one a state.
#[test]
fn a_tab_says_its_state_without_colour() {
    let (mut harness, _fake) = open(Screen::Line, Theme::Light, WINDOW, 1.0, false);
    four_states(&mut harness);
    let says = |name: &str, said: &str| match name {
        "Carte Nano n°3" => said.starts_with("Connecté à COM23"),
        "COM4" => said == "Déconnecté",
        "COM7" => said.contains("COM7 perdu"),
        _ => said.starts_with("Déconnecté") && said.contains("accès refusé"),
    };
    let mut shapes = Vec::new();
    for name in ["Carte Nano n°3", "COM4", "COM7", "COM9"] {
        let tab = harness.get_by(|node| node.role() == Role::Button && node.label().as_deref() == Some(name));
        let said = tab.accesskit_node().description().unwrap_or_default();
        assert!(says(name, &said), "the tab « {name} » says « {said} » of its state");
        let rect = tab.rect();
        let mark = marks_in(&harness, Rect::from_min_max(egui::pos2(rect.left() - 24.0, rect.top()), egui::pos2(rect.left(), rect.bottom())));
        assert!(!mark.is_empty(), "the tab « {name} » has no mark");
        shapes.push((name, mark));
    }
    for (n, (name, mark)) in shapes.iter().enumerate() {
        for (other, other_mark) in &shapes[n + 1..] {
            assert_ne!(mark, other_mark, "« {name} » and « {other} » bear the same mark: only the colour tells their states apart");
        }
    }
}

/// The separation of the console and the curves moves without a drag (WCAG 2.5.7): a click on it
/// opens a choice of heights; a drag still moves it, and opens nothing.
#[test]
fn the_separation_moves_by_a_click() {
    let (mut harness, _fake) = open(Screen::Curves, Theme::Light, WINDOW, 1.0, false);
    let name = "Séparation console et courbes";
    let at = elements(&harness).into_iter().find(|element| element.name == name).expect("the separation").rect.center();
    let before = harness.state().sessions[0].plot_share;
    harness.event(egui::Event::PointerMoved(at));
    harness.event(egui::Event::PointerButton { pos: at, button: egui::PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE });
    harness.run();
    let to = at + egui::vec2(0.0, -60.0);
    harness.event(egui::Event::PointerMoved(to));
    harness.run();
    harness.event(egui::Event::PointerButton { pos: to, button: egui::PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE });
    harness.run();
    let dragged = harness.state().sessions[0].plot_share;
    assert!(dragged > before + 0.05, "the drag moved the curves' share from {before} to {dragged}");
    assert!(harness.query_by_label("Trois quarts").is_none(), "a drag opened the choice of heights");
    harness.get_by(|node| node.label().as_deref() == Some(name)).click();
    harness.run();
    harness.get_by(|node| node.role() == Role::Button && node.label().as_deref() == Some("Trois quarts")).click();
    harness.run();
    let share = harness.state().sessions[0].plot_share;
    assert!((share - 0.75).abs() < 1e-6, "« Trois quarts » chosen: the curves' share is {share}");
}

/// Paused, the curves move and zoom by buttons (WCAG 2.5.7, and 2.1.1: the wheel is no keyboard):
/// « Plus tôt » and « Plus tard » by half the width shown, « Rapprocher » and « Éloigner » to half or
/// twice that width, never out of the part chosen; while the curves run, no such button.
#[test]
fn paused_curves_move_and_zoom_by_buttons() {
    const BUTTONS: [&str; 4] = ["Plus tôt", "Plus tard", "Rapprocher", "Éloigner"];
    let (mut harness, _fake) = open(Screen::Curves, Theme::Light, WINDOW, 1.0, false);
    for name in BUTTONS {
        assert!(harness.query_by_label(name).is_none(), "« {name} » while the curves run");
    }
    harness.get_by_label("Pause").click();
    harness.run();
    let shown = |harness: &Harness<'static, App>| {
        let memory = PlotMemory::load(&harness.ctx, egui::Id::new(("curves", 0_usize)).with(("chart", 0_usize))).expect("the chart drawn");
        (memory.bounds().min()[0], memory.bounds().max()[0])
    };
    let press = |harness: &mut Harness<'static, App>, name: &str| {
        harness.get_by(|node| node.role() == Role::Button && node.label().as_deref() == Some(name)).click();
        harness.run();
    };
    let (from, to) = shown(&harness);
    let whole = to - from;
    let near = |a: f64, b: f64| (a - b).abs() <= whole * 1e-6;
    press(&mut harness, "Rapprocher");
    let (a, b) = shown(&harness);
    assert!(near(b - a, whole / 2.0) && near((a + b) / 2.0, (from + to) / 2.0), "« Rapprocher »: {a} to {b}, from {from} to {to}");
    press(&mut harness, "Plus tôt");
    let (c, d) = shown(&harness);
    assert!(near(c, a - whole / 4.0) && near(d, b - whole / 4.0), "« Plus tôt »: {c} to {d}, from {a} to {b}");
    for _ in 0..3 {
        press(&mut harness, "Plus tard");
    }
    let (e, f) = shown(&harness);
    assert!(near(f, to) && near(f - e, whole / 2.0), "« Plus tard », three times: {e} to {f}, the part ends at {to}");
    press(&mut harness, "Éloigner");
    press(&mut harness, "Éloigner");
    let (g, h) = shown(&harness);
    assert!(near(g, from) && near(h, to), "« Éloigner », twice: {g} to {h}, the part is {from} to {to}");
}
