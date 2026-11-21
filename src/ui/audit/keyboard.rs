//! The keyboard and the focus (WCAG 2.1.1, 2.1.2, 2.4.3, 2.4.7, 2.4.11): the measure
//! `keyboard`, the focus ring, the order of Tab, Ctrl+Tab out of the terminal, the separation at the
//! arrows, the settings that keep the focus.

use super::*;

/// What the focus changes around `rect`: the image focused against the same without the focus.
/// Gives the pixels changed, those changed by 3:1 or more, and the largest change.
fn focus_change(harness: &mut Harness<'static, App>, rect: Rect) -> (usize, usize, f64) {
    let Some(id) = harness.ctx.memory(|memory| memory.focused()) else {
        return (0, 0, 1.0);
    };
    let focused = harness.render().expect("drawn");
    harness.ctx.memory_mut(|memory| memory.surrender_focus(id));
    harness.run();
    let plain = harness.render().expect("drawn");
    harness.ctx.memory_mut(|memory| memory.request_focus(id));
    harness.run();
    let scale = harness.ctx.pixels_per_point();
    let area = rect.expand(4.0);
    let x = ((area.min.x * scale).max(0.0) as u32)..((area.max.x * scale).max(0.0) as u32).min(focused.width());
    let y = ((area.min.y * scale).max(0.0) as u32)..((area.max.y * scale).max(0.0) as u32).min(focused.height());
    let (mut changed, mut strong, mut most) = (0, 0, 1.0_f64);
    for row in y {
        for column in x.clone() {
            let [a, b] = [focused.get_pixel(column, row).0, plain.get_pixel(column, row).0];
            if a != b {
                let change = ratio(Color32::from_rgb(a[0], a[1], a[2]), Color32::from_rgb(b[0], b[1], b[2]));
                changed += 1;
                strong += usize::from(change >= 3.0);
                most = most.max(change);
            }
        }
    }
    (changed, strong, most)
}

/// The settings' window, and what is in it, when it is open.
fn settings_window(harness: &Harness<'_, App>) -> Option<(Rect, Vec<NodeId>)> {
    harness.root().children_recursive().find(|node| node.accesskit_node().role() == Role::Window).map(|node| {
        let inside: Vec<NodeId> = node.children_recursive().map(|child| child.accesskit_node().locate().0).collect();
        (node.rect(), inside)
    })
}

/// Tab from nothing focused, until the focus comes back to the first element, stays put, or
/// `MOST_TABS`: each element reached, in order, how its focus shows, and whether it is hidden
/// under the settings' window. Then the elements that take the focus and were never reached.
fn walk(harness: &mut Harness<'static, App>, screen: &str, order: &mut String, missed: &mut String) {
    if let Some(id) = harness.ctx.memory(|memory| memory.focused()) {
        harness.ctx.memory_mut(|memory| memory.surrender_focus(id));
    }
    harness.run();
    let window = Rect::from_min_size(Pos2::ZERO, WINDOW);
    let before = elements(harness);
    let settings = settings_window(harness);
    let mut seen: Vec<NodeId> = Vec::new();
    for step in 1..=MOST_TABS {
        harness.key_press(Key::Tab);
        harness.run();
        let Some(element) = elements(harness).into_iter().find(|element| element.focused) else {
            writeln!(order, "{screen};{step};·;\"(aucun focus)\";;;;;;;;").unwrap();
            continue;
        };
        if seen.first() == Some(&element.id) {
            writeln!(order, "{screen};{step};·;\"(retour au premier)\";;;;;;;;").unwrap();
            break;
        }
        if seen.last() == Some(&element.id) {
            writeln!(order, "{screen};{step};·;\"(le focus ne bouge plus)\";;;;;;;;").unwrap();
            break;
        }
        let hidden = settings.as_ref().is_some_and(|(rect, inside)| !inside.contains(&element.id) && rect.contains_rect(element.rect));
        let (changed, strong, most) = focus_change(harness, element.rect);
        writeln!(
            order,
            "{screen};{step};{:?};{};{:.0};{:.0};{:.0};{:.0};{changed};{strong};{most:.2};{}",
            element.role,
            cell(&element.name),
            element.rect.min.x,
            element.rect.min.y,
            element.rect.width(),
            element.rect.height(),
            if hidden { "oui" } else { "non" }
        )
        .unwrap();
        seen.push(element.id);
    }
    for element in before.iter().filter(|element| element.focus && window.intersects(element.rect) && !seen.contains(&element.id)) {
        writeln!(missed, "{screen};{:?};{}", element.role, cell(&element.name)).unwrap();
    }
    println!("{screen} : {} éléments atteints au clavier", seen.len());
}

/// Tab through the window and the settings, in both themes; then the keys that could leave the
/// terminal's screen, and the keys on the separation of the console and the curves.
#[test]
#[ignore = "a measure of accessibility, run by hand on a machine with a GPU"]
fn keyboard() {
    let folder = folder();
    let mut order = String::from("ecran;pas;role;nom;x;y;largeur;hauteur;pixels changes;pixels changes a 3:1 ou plus;plus grand ecart;cache sous la fenetre des reglages\n");
    let mut missed = String::from("ecran;role;nom\n");
    for theme in [Theme::Light, Theme::Dark] {
        for screen in [Screen::Line, Screen::Curves, Screen::Terminal, Screen::Settings(Section::Link)] {
            let (mut harness, _fake) = open(screen, theme, WINDOW, 1.0, true);
            walk(&mut harness, &format!("{}-{}", screen.name(), theme_name(theme)), &mut order, &mut missed);
        }
    }
    fs::write(folder.join("clavier-ordre.csv"), order).unwrap();
    fs::write(folder.join("clavier-jamais-atteints.csv"), missed).unwrap();

    let mut cases = String::from("cas;touche;resultat\n");
    // The terminal's screen keeps Tab, the arrows and Escape for the device: does a key leave it?
    let (mut harness, fake) = open(Screen::Terminal, Theme::Light, WINDOW, 1.0, false);
    let keys = [("Tab", Modifiers::NONE, Key::Tab), ("Maj+Tab", Modifiers::SHIFT, Key::Tab), ("Échap", Modifiers::NONE, Key::Escape), ("Ctrl+Tab", Modifiers::CTRL, Key::Tab), ("F6", Modifiers::NONE, Key::F6)];
    for (name, modifiers, key) in keys {
        harness.get_by_label("Écran du terminal").click();
        harness.run();
        let sent = fake.written().len();
        harness.key_press_modifiers(modifiers, key);
        harness.run();
        let start = Instant::now();
        while fake.written().len() == sent && start.elapsed() < Duration::from_millis(500) {
            harness.step();
            std::thread::sleep(Duration::from_millis(10));
        }
        let still = elements(&harness).iter().any(|element| element.focused && element.name == "Écran du terminal");
        let bytes = fake.written()[sent..].to_vec();
        writeln!(cases, "sortir de l'écran du terminal;{name};{} ; envoyé à l'appareil : {bytes:02X?}", if still { "le focus reste sur l'écran" } else { "le focus quitte l'écran" }).unwrap();
    }
    // The separation of the console and the curves: reached, named, moved by keys?
    let (mut harness, _fake) = open(Screen::Curves, Theme::Light, WINDOW, 1.0, false);
    let split = elements(&harness).into_iter().find(|element| element.focus && element.rect.height() <= 12.0 && element.rect.width() > 300.0);
    match split {
        None => writeln!(cases, "séparation console et courbes;·;absente de l'arbre ou sans focus").unwrap(),
        Some(split) => {
            writeln!(cases, "séparation console et courbes;·;dans l'arbre : rôle {:?}, nom « {} », {:.0}×{:.0}", split.role, split.name, split.rect.width(), split.rect.height()).unwrap();
            let id = split.id;
            for key in [Key::ArrowUp, Key::ArrowDown, Key::PageUp, Key::PageDown] {
                harness.get_by(move |node| node.locate().0 == id).focus();
                harness.run();
                let focused = elements(&harness).iter().any(|element| element.focused && element.id == split.id);
                let before = harness.state().sessions[0].plot_share;
                harness.key_press(key);
                harness.run();
                let after = harness.state().sessions[0].plot_share;
                writeln!(cases, "séparation console et courbes;{key:?};focus pris : {} ; part des courbes {before:.3} puis {after:.3}", if focused { "oui" } else { "non" }).unwrap();
            }
        }
    }
    fs::write(folder.join("clavier-cas.csv"), cases).unwrap();
}

/// Tab from nothing focused, until the focus comes back to the first element, stays put, or
/// `MOST_TABS`; `each` sees the window at each element. The elements met, in order.
fn tab_walk(harness: &mut Harness<'static, App>, mut each: impl FnMut(&Harness<'static, App>, &Element)) -> Vec<Element> {
    unfocus(harness);
    let mut met: Vec<Element> = Vec::new();
    for _ in 0..MOST_TABS {
        harness.key_press(Key::Tab);
        harness.run();
        let Some(element) = focused(harness) else { continue };
        if met.first().is_some_and(|first| first.id == element.id) || met.last().is_some_and(|last| last.id == element.id) {
            break;
        }
        each(&*harness, &element);
        met.push(element);
    }
    met
}

/// The rectangles stroked in `color` and `width` in the window's last image.
fn stroked(harness: &Harness<'_, App>, color: Color32, width: f32) -> Vec<Rect> {
    fn look(shape: &egui::Shape, color: Color32, width: f32, found: &mut Vec<Rect>) {
        match shape {
            egui::Shape::Vec(shapes) => shapes.iter().for_each(|shape| look(shape, color, width, found)),
            egui::Shape::Rect(rect) if rect.stroke.color == color && (rect.stroke.width - width).abs() < 0.01 => found.push(rect.rect),
            _ => {}
        }
    }
    let mut found = Vec::new();
    for clipped in &harness.output().shapes {
        look(&clipped.shape, color, width, &mut found);
    }
    found
}

/// The focus ring around what shows of the focused widget (the console scrolls), in the light
/// theme: 2 px of accent, close around it.
fn has_ring(harness: &Harness<'_, App>) -> bool {
    let Some(shown) = harness.ctx.memory(|memory| memory.focused()).and_then(|id| harness.ctx.read_response(id)).map(|response| response.interact_rect) else {
        return false;
    };
    stroked(harness, Palette::of(false).accent, 2.0).iter().any(|ring| ring.contains_rect(shown) && ring.area() <= shown.expand(6.0).area())
}

/// The terminal's screen keeps Tab for the device; Ctrl+Tab and Ctrl+Shift+Tab leave it, and the
/// device receives nothing of them.
#[test]
fn ctrl_tab_leaves_the_terminal_screen_and_sends_nothing() {
    let screen = "Écran du terminal";
    let (mut harness, fake) = open(Screen::Terminal, Theme::Light, WINDOW, 1.0, false);
    harness.get_by_label(screen).click();
    harness.run();
    assert!(focused(&harness).is_some_and(|element| element.name == screen), "a click gives the screen the keyboard");
    let sent = fake.written().len();
    harness.key_press_modifiers(Modifiers::CTRL, Key::Tab);
    harness.run();
    assert!(focused(&harness).is_some_and(|element| element.name != screen), "Ctrl+Tab leaves the screen");
    harness.key_press_modifiers(Modifiers::CTRL | Modifiers::SHIFT, Key::Tab);
    harness.run();
    assert!(focused(&harness).is_some_and(|element| element.name == screen), "Ctrl+Shift+Tab comes back to it");
    // The thread of the port writes on its own time.
    let start = Instant::now();
    while start.elapsed() < Duration::from_millis(300) {
        harness.step();
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(fake.written().len(), sent, "nothing sent to the device");
}

/// The separation of the console and the curves is a control: named, a value, moved by the arrows.
#[test]
fn the_separation_moves_with_the_arrows() {
    let (mut harness, _fake) = open(Screen::Curves, Theme::Light, WINDOW, 1.0, false);
    let name = "Séparation console et courbes";
    harness.get_by(|node| node.label().as_deref() == Some(name)).focus();
    harness.run();
    assert!(focused(&harness).is_some_and(|element| element.name == name && element.role == Role::Slider), "focused, as a slider");
    let share = |harness: &Harness<'static, App>| harness.state().sessions[0].plot_share;
    let before = share(&harness);
    harness.key_press(Key::ArrowUp);
    harness.run();
    assert!((share(&harness) - (before + 0.05)).abs() < 1e-4, "up: the curves take 5 % more, {before} then {}", share(&harness));
    harness.key_press(Key::ArrowDown);
    harness.key_press(Key::ArrowDown);
    harness.run();
    assert!((share(&harness) - (before - 0.05)).abs() < 1e-4, "down twice: 5 % less than at first");
    for _ in 0..30 {
        harness.key_press(Key::ArrowUp);
    }
    harness.run();
    assert!((share(&harness) - super::super::middle::PLOT_SHARES.end()).abs() < 1e-4, "no further than its bound");
    assert!(focused(&harness).is_some_and(|element| element.name == name), "the arrows leave the focus on it");
}

/// At the keyboard, each control reached by Tab shows the focus ring.
#[test]
fn every_focused_control_shows_the_focus_ring() {
    for screen in [Screen::Line, Screen::Curves, Screen::Terminal, Screen::Settings(Section::Link)] {
        let (mut harness, _fake) = open(screen, Theme::Light, WINDOW, 1.0, false);
        let mut missing = Vec::new();
        let met = tab_walk(&mut harness, |harness, element| {
            if !has_ring(harness) {
                missing.push(element.called());
            }
        });
        assert!(!met.is_empty(), "{}: Tab reaches something", screen.name());
        assert!(missing.is_empty(), "{}: no focus ring on {missing:?}", screen.name());
    }
}

/// A click gives the focus without the ring; the ring comes with the keyboard.
#[test]
fn a_click_shows_no_focus_ring() {
    let (mut harness, _fake) = open(Screen::Line, Theme::Light, WINDOW, 1.0, false);
    harness.get_by(|node| node.role() == Role::TextInput).click();
    harness.run();
    let field = focused(&harness).expect("the field takes the focus");
    assert!(!has_ring(&harness), "no ring after a click on {}", field.called());
    harness.key_press(Key::Tab);
    harness.run();
    let next = focused(&harness).expect("Tab moves the focus");
    assert!(has_ring(&harness), "after Tab, the ring on {}", next.called());
}

/// The settings open, Tab stays in them, and the window behind does not answer the pointer.
#[test]
fn the_settings_keep_the_focus() {
    let (mut harness, _fake) = open(Screen::Settings(Section::Link), Theme::Light, WINDOW, 1.0, false);
    let (_, inside) = settings_window(&harness).expect("the settings' window");
    let met = tab_walk(&mut harness, |_, _| {});
    let outside: Vec<String> = met.iter().filter(|element| !inside.contains(&element.id)).map(Element::called).collect();
    assert!(outside.is_empty(), "Tab left the settings for {outside:?}");
    harness.get_by_label("COM4").click();
    harness.run();
    assert_eq!(harness.state().settings.active, 0, "the tab behind the settings stays as it was");
}

/// The settings opened at the keyboard, from "Settings": the next Tab goes into them.
#[test]
fn the_settings_opened_at_the_keyboard_take_the_next_tab() {
    let (mut harness, _fake) = open(Screen::Line, Theme::Light, WINDOW, 1.0, false);
    unfocus(&mut harness);
    for _ in 0..MOST_TABS {
        if focused(&harness).is_some_and(|element| element.name == "Réglages") {
            break;
        }
        harness.key_press(Key::Tab);
        harness.run();
    }
    harness.key_press(Key::Enter);
    harness.run();
    let (_, inside) = settings_window(&harness).expect("Enter opens the settings");
    harness.key_press(Key::Tab);
    harness.run();
    assert!(focused(&harness).is_some_and(|element| inside.contains(&element.id)), "Tab goes into the settings, not to {:?}", focused(&harness).map(|element| element.called()));
}

/// Tab goes left to right along a row, and the settings' "Close" comes after their sections.
#[test]
fn tab_follows_the_reading_order() {
    for screen in [Screen::Line, Screen::Curves, Screen::Settings(Section::Link)] {
        let (mut harness, _fake) = open(screen, Theme::Light, WINDOW, 1.0, false);
        let met = tab_walk(&mut harness, |_, _| {});
        for pair in met.windows(2) {
            let (a, b) = (&pair[0], &pair[1]);
            if (a.rect.center().y - b.rect.center().y).abs() < 4.0 {
                assert!(b.rect.center().x > a.rect.center().x, "{}: « {} » before « {} », on the same row, to its right", screen.name(), a.called(), b.called());
            }
        }
        if matches!(screen, Screen::Settings(_)) {
            let at = |name: &str| met.iter().position(|element| element.name == name).unwrap_or_else(|| panic!("{name}: reached"));
            assert!(at("Fermer") > at("À propos"), "« Fermer » after the sections");
        }
    }
}
