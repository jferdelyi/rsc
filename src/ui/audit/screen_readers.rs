//! The text received, for screen readers (WCAG 1.3.1, 4.1.2, 1.1.1, 4.1.3): the console's
//! lines and its keys, the terminal's rows, the charts' values, the state line announced.

use super::*;

/// What a screen reader gets of the painted area named `name` (the console, the terminal's screen):
/// its role, its description, and its lines, each its text, its description and where it shows.
fn read_lines(harness: &Harness<'_, App>, name: &str) -> (Role, String, Vec<(String, String, Rect)>) {
    let scale = harness.ctx.pixels_per_point();
    let area = harness.get_by(|node| node.label().as_deref() == Some(name) && node.role() != Role::RadioButton);
    let lines = area
        .children_recursive()
        .map(|line| line.accesskit_node())
        .filter(|line| line.role() == Role::Label)
        .map(|line| {
            let at = line.bounding_box().map_or(Rect::NOTHING, |b| Rect::from_min_max(Pos2::new(b.x0 as f32 / scale, b.y0 as f32 / scale), Pos2::new(b.x1 as f32 / scale, b.y1 as f32 / scale)));
            (line.value().unwrap_or_default(), line.description().unwrap_or_default(), at)
        })
        .collect();
    let area = area.accesskit_node();
    (area.role(), area.description().unwrap_or_default(), lines)
}

/// The console's lines as shown and copied: their time first, as the window's settings ask.
fn shown_lines(harness: &Harness<'_, App>) -> Vec<String> {
    let shown = super::super::Shown { timestamps: true, hex: false };
    harness.state().sessions[0].console.lines().iter().map(|line| line.display(shown)).collect()
}

/// The window on the line console, 200 lines received after the first ones.
fn console_of_200() -> (Harness<'static, App>, FakePorts) {
    let (mut harness, fake) = open(Screen::Line, Theme::Light, WINDOW, 1.0, false);
    for n in 0..200 {
        fake.device_sends(format!("mesure {n:03}\r\n").as_bytes());
    }
    until(&mut harness, "200 lines", |app| app.sessions[0].console.text(super::super::Shown::default()).contains("mesure 199"));
    harness.run();
    (harness, fake)
}

/// The console gives screen readers the lines it shows (WCAG 1.3.1, 4.1.2): it is a log, each line
/// in view an element of it, its text as shown and copied; a line sent and a word of RSC say what
/// they are, as « > » and « # » show it. Empty, it gives what it says on screen.
#[test]
fn the_console_gives_its_lines_to_screen_readers() {
    let (mut harness, _fake) = console_of_200();
    let now = still_clock();
    harness.state_mut().sessions[0].console.sent("AT+GMR", b"AT+GMR\r\n", now);
    harness.state_mut().sessions[0].console.notice("Reconnexion arrêtée", now);
    harness.run();
    let (role, _, lines) = read_lines(&harness, "Console");
    assert_eq!(role, Role::Log, "the console is a log");
    assert!(lines.len() >= 10, "{} line(s) given, the console full of them", lines.len());
    let all = shown_lines(&harness);
    let given: Vec<&str> = lines.iter().map(|(text, _, _)| text.as_str()).collect();
    let last: Vec<&str> = all[all.len() - given.len()..].iter().map(String::as_str).collect();
    assert_eq!(given, last, "the lines given are the last ones, in order: the console follows them");
    let window = Rect::from_min_size(Pos2::ZERO, WINDOW);
    assert!(lines.iter().all(|(_, _, at)| at.height() > 0.0 && window.contains_rect(*at)), "a line given out of the window: {lines:?}");
    let said: Vec<&str> = lines.iter().map(|(_, description, _)| description.as_str()).collect();
    assert_eq!(said[said.len() - 2..], ["envoyé", "message de RSC"], "a line sent, then a word of RSC");
    assert!(said[..said.len() - 2].iter().all(|description| description.is_empty()), "a line received says nothing more: {said:?}");
    harness.state_mut().sessions[0].console.clear();
    harness.run();
    let (_, said, lines) = read_lines(&harness, "Console");
    assert!(lines.is_empty(), "an empty console gives lines: {lines:?}");
    assert_eq!(said, "Connecté : ce que l'appareil envoie s'affiche ici.", "an empty console gives what it says on screen");
}

/// The console's element is the part of it in view, where a screen reader frames it: not the whole
/// height of its lines, mostly out of the window (an earlier tree: 924 × 923 from y = -562).
#[test]
fn the_console_is_where_it_shows() {
    let (harness, _fake) = console_of_200();
    let console = elements(&harness).into_iter().find(|element| element.name == "Console").expect("the console");
    let window = Rect::from_min_size(Pos2::ZERO, WINDOW);
    assert!(window.contains_rect(console.rect), "the console at {:?}, out of the window", console.rect);
}

/// The console scrolls by the keyboard (WCAG 2.1.1): Page Up and Page Down a page at a time, a line
/// kept; Home and End to its first and last lines. Going up stops the following and End starts it
/// again, as the wheel does; the lines given to screen readers follow the view.
#[test]
fn the_console_scrolls_by_the_keyboard() {
    let (mut harness, _fake) = console_of_200();
    harness.get_by(|node| node.label().as_deref() == Some("Console")).focus();
    harness.run();
    assert!(focused(&harness).is_some_and(|element| element.name == "Console"), "the console has the keyboard");
    let all = shown_lines(&harness);
    let place = |harness: &Harness<'static, App>, first: bool| {
        let (_, _, lines) = read_lines(harness, "Console");
        let text = if first { lines.first() } else { lines.last() }.map(|(text, _, _)| text.clone()).unwrap_or_default();
        all.iter().position(|line| *line == text).unwrap_or_else(|| panic!("« {text} » is no line of the console"))
    };
    let press = |harness: &mut Harness<'static, App>, key: Key| {
        harness.key_press(key);
        harness.run();
    };
    let top = place(&harness, true);
    press(&mut harness, Key::PageUp);
    let bottom = place(&harness, false);
    assert!(bottom.abs_diff(top) <= 1, "Page Up: the last line in view is line {bottom}, the first one was {top}");
    assert!(!harness.state().sessions[0].view.follows(), "going up stops the following");
    press(&mut harness, Key::Home);
    assert_eq!(place(&harness, true), 0, "Home: the console's first line in view");
    let bottom = place(&harness, false);
    press(&mut harness, Key::PageDown);
    let top = place(&harness, true);
    assert!(top.abs_diff(bottom) <= 1, "Page Down: the first line in view is line {top}, the last one was {bottom}");
    press(&mut harness, Key::End);
    assert_eq!(place(&harness, false), all.len() - 1, "End: the console's last line in view");
    assert!(harness.state().sessions[0].view.follows(), "End starts the following again");
}

/// The terminal's screen gives screen readers its rows (WCAG 1.3.1, 4.1.2): each row with text an
/// element of it, blank ones left out; where the cursor is, in its description.
#[test]
fn the_terminal_screen_gives_its_rows() {
    let (mut harness, fake) = open(Screen::Terminal, Theme::Light, WINDOW, 1.0, false);
    fake.device_sends(b"\x1b[2J\x1b[H\x1b[?25hbonjour\r\nmonde");
    until(&mut harness, "two rows", |app| app.sessions[0].terminal.as_ref().is_some_and(|terminal| terminal.screen().contents().trim_end() == "bonjour\nmonde"));
    harness.run();
    let (role, said, rows) = read_lines(&harness, "Écran du terminal");
    assert_eq!(role, Role::Terminal, "the screen is a terminal");
    let given: Vec<&str> = rows.iter().map(|(text, _, _)| text.as_str()).collect();
    assert_eq!(given, ["bonjour", "monde"], "its rows with text");
    assert_eq!(said, "Curseur : ligne 2, colonne 6", "where the cursor is");
}

/// Each chart says its curves and their values (WCAG 1.1.1): for each curve, its last value, then
/// its lowest and highest in the part shown; a curve hidden by its legend says so; paused, the chart
/// says so.
#[test]
fn every_chart_says_its_curves_and_their_values() {
    let (mut harness, fake) = open(Screen::Curves, Theme::Light, WINDOW, 1.0, false);
    harness.state_mut().sessions[0].plot.clear();
    harness.state_mut().settings.edit_tab_profile(0, |profile| profile.plot_below = vec!["hum".into()]);
    for (n, (temp, hum)) in [(21.5, 40.0), (20.75, 42.5), (22.0, 41.0)].into_iter().enumerate() {
        fake.device_sends(format!("temp={temp} hum={hum}\r\n").as_bytes());
        until(&mut harness, "measure", |app| app.sessions[0].plot.curves().first().is_some_and(|curve| curve.points().len() == n + 1));
    }
    harness.run();
    let said = |harness: &Harness<'static, App>, chart: &str| harness.get_by(|node| node.label().as_deref() == Some(chart)).accesskit_node().description().unwrap_or_default();
    assert_eq!(said(&harness, "Courbes : temp"), "temp : 22, de 20.75 à 22");
    assert_eq!(said(&harness, "Courbes : hum"), "hum : 41, de 40 à 42.5");
    harness.get_by(|node| node.role() == Role::CheckBox && node.label().as_deref() == Some("hum")).click();
    harness.run();
    assert_eq!(said(&harness, "Courbes : hum"), "hum : masquée", "a curve hidden by its legend");
    harness.get_by_label("Pause").click();
    harness.run();
    assert_eq!(said(&harness, "Courbes : temp"), "temp : 22, de 20.75 à 22 ; en pause", "paused");
}

/// Closing the port is announced once (heard twice in the real window, 19/11/2026): frame by frame,
/// the times Windows would announce the state line, by AccessKit's rule (accesskit_windows 0.32.1,
/// adapter.rs:225 and 272): an announced element that comes with a name, or whose name changes.
#[test]
fn closing_the_port_is_announced_once() {
    let (mut harness, _fake) = open(Screen::Line, Theme::Light, WINDOW, 1.0, false);
    let state = |harness: &Harness<'static, App>| harness.query_by(|node| node.role() == Role::Status).map(|node| node.accesskit_node().label().unwrap_or_default());
    let mut before = state(&harness);
    let mut heard = Vec::new();
    harness.get_by_label("Déconnecter").click();
    for _ in 0..60 {
        harness.step();
        std::thread::sleep(Duration::from_millis(10));
        let now = state(&harness);
        if let Some(name) = now.as_ref().filter(|name| !name.is_empty() && before.as_ref() != Some(*name)) {
            heard.push(name.clone());
        }
        before = now;
    }
    assert!(!harness.state().sessions[0].is_open(), "the port closed");
    assert_eq!(heard, ["Déconnecté"], "announced while the port closed");
}

/// The state line is a status that screen readers announce when it changes, politely (WCAG 4.1.3):
/// its words are those shown. It is one element on the row of the tabs or under the connection bar,
/// for a move not to be read again.
#[test]
fn the_state_line_is_announced() {
    let status = |harness: &Harness<'static, App>| {
        let status = harness.get_by(|node| node.role() == Role::Status);
        let status = status.accesskit_node();
        (status.locate().0, status.label().unwrap_or_default(), status.live())
    };
    let (mut harness, _fake) = open(Screen::Line, Theme::Light, WINDOW, 1.0, false);
    let (id, said, live) = status(&harness);
    assert_eq!(live, Live::Polite, "announced, politely");
    assert!(said.starts_with("Connecté à COM23"), "connected, it says « {said} »");
    // Where a screen reader frames it: where its words show.
    let place = elements(&harness).into_iter().find(|element| element.role == Role::Status).map(|element| element.rect);
    let words = elements(&harness).into_iter().find(|element| element.role == Role::Label && element.value.starts_with("Connecté à")).expect("the state's words").rect;
    assert!(place.is_some_and(|place| place.contains_rect(words)), "the status at {place:?}, its words at {words:?}");
    harness.get_by_label("Déconnecter").click();
    until(&mut harness, "closed", |app| !app.sessions[0].is_open());
    harness.run();
    assert_eq!(status(&harness).1, "Déconnecté", "closed");
    let (harness, _fake) = open(Screen::Line, Theme::Light, LEAST, 1.0, false);
    assert!(state_on_tabs_row(&elements(&harness)), "at the least size, the state is up on the row of the tabs");
    assert_eq!(status(&harness).0, id, "the same element, up on the row of the tabs");
}
