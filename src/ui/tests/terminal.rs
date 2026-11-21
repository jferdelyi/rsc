//! The terminal mode: its screen and shortcuts, the keys to the device, Ctrl+C and Ctrl+V,
//! the screen's size, a word taken by a double click, the glyphs of lines and corners.

use super::*;

/// A tab on COM7 in the terminal mode, connected, its screen drawn.
fn terminal_on_com7(fake: &FakePorts, test: &str) -> Harness<'static, App> {
    let mut settings = on("COM7");
    settings.general.terminal = true;
    let mut harness = harness(fake, settings, test);
    harness.get_by_label("Connecter").click();
    until(&mut harness, "connected", |app| app.sessions[0].is_open());
    harness.run();
    harness
}

#[test]
fn the_terminal_switch_shows_the_screen_and_says_its_shortcuts() {
    let fake = FakePorts::with(&["COM7"]);
    let mut harness = harness(&fake, on("COM7"), "terminal-switch");
    harness.get_by_label("Connecter").click();
    until(&mut harness, "connected", |app| app.sessions[0].is_open());
    fake.device_sends(b"boot ok\r\n");
    until(&mut harness, "a line", |app| console_text(app, 0).contains("boot ok"));
    harness.get_by_label("Terminal").click();
    harness.run();
    let app = harness.state();
    assert!(app.settings.profile_of(0).terminal && app.sessions[0].terminal.is_some());
    assert!(harness.query_by_label("Envoyer").is_none(), "no send line: the keys go to the device");
    harness.get_by_label_contains("Ctrl+C : copie la sélection, sinon interrompt · Ctrl+V : colle · Ctrl+Maj+C / Ctrl+Maj+V : toujours copier / coller");
    let (rows, cols) = harness.state().sessions[0].terminal.as_ref().expect("the screen").size();
    harness.get_by_label_contains(&format!("Le clavier va à l'appareil · {cols}×{rows}"));
    for greyed in ["Horodatage", "Affichage hex", "Défilement auto"] {
        assert!(is_disabled(&harness, greyed), "{greyed} is the line console's");
    }
    // The line console goes on underneath: the journal and the curves read it.
    fake.device_sends(b"in the terminal\r\n");
    until(&mut harness, "the line console fed", |app| console_text(app, 0).contains("in the terminal"));
    assert!(screen(harness.state()).contains("in the terminal"));
    harness.get_by_label("Terminal").click();
    harness.run();
    assert!(harness.state().sessions[0].terminal.is_none());
    harness.get_by_label("Envoyer");
    assert!(console_text(harness.state(), 0).contains("boot ok\nin the terminal\n"));
}

#[test]
fn the_keys_go_to_the_device_one_by_one_tab_and_arrows_included() {
    let fake = FakePorts::with(&["COM7"]);
    let mut harness = terminal_on_com7(&fake, "terminal-keys");
    harness.event(egui::Event::Text("ls".into()));
    harness.key_press(egui::Key::Tab);
    harness.key_press(egui::Key::ArrowUp);
    harness.key_press_modifiers(egui::Modifiers::CTRL, egui::Key::A);
    harness.key_press(egui::Key::Escape);
    harness.key_press(egui::Key::Enter);
    until(&mut harness, "sent", |_| fake.written() == b"ls\t\x1b[A\x01\x1b\r");
    assert!(!console_text(harness.state(), 0).contains("> "), "nothing echoed by RSC: the device echoes");
}

#[test]
fn ctrl_c_copies_a_selection_and_interrupts_without_one() {
    let fake = FakePorts::with(&["COM7"]);
    let mut harness = terminal_on_com7(&fake, "terminal-copy");
    fake.device_sends(b"hello world");
    until(&mut harness, "drawn", |app| screen(app).contains("hello"));
    harness.event(egui::Event::Copy);
    until(&mut harness, "Ctrl+C sent", |_| fake.written() == b"\x03");
    harness.event_modifiers(egui::Event::Copy, egui::Modifiers::CTRL | egui::Modifiers::SHIFT);
    harness.run();
    assert_eq!(fake.written(), b"\x03", "Ctrl+Shift+C never interrupts");
    harness.state_mut().sessions[0].terminal_view.select((0, 0), (0, 5));
    harness.event(egui::Event::Copy);
    harness.step();
    let copied = harness.output().platform_output.commands.iter().any(|command| matches!(command, egui::OutputCommand::CopyText(text) if text == "hello"));
    assert!(copied, "the selection copied");
    assert_eq!(fake.written(), b"\x03", "and nothing sent");
    // Ctrl+X, which nano quits with.
    harness.event(egui::Event::Cut);
    until(&mut harness, "Ctrl+X sent", |_| fake.written() == b"\x03\x18");
}

#[test]
fn what_is_pasted_goes_as_typed_and_bracketed_when_the_device_asks() {
    let fake = FakePorts::with(&["COM7"]);
    let mut harness = terminal_on_com7(&fake, "terminal-paste");
    harness.event(egui::Event::Paste("echo a\necho b".into()));
    until(&mut harness, "pasted", |_| fake.written() == b"echo a\recho b");
    fake.device_sends(b"\x1b[?2004h");
    until(&mut harness, "the mode", |app| app.sessions[0].terminal.as_ref().is_some_and(|terminal| terminal.modes().bracketed_paste));
    harness.event(egui::Event::Paste("ls".into()));
    until(&mut harness, "bracketed", |_| fake.written().ends_with(b"\x1b[200~ls\x1b[201~"));
}

#[test]
fn the_device_learns_the_size_of_the_screen_it_draws_on() {
    let fake = FakePorts::with(&["COM7"]);
    let mut harness = terminal_on_com7(&fake, "terminal-size");
    let (rows, cols) = harness.state().sessions[0].terminal.as_ref().expect("the screen").size();
    assert!(rows > 24 && cols > 80, "the view's size, not 80×24: {cols}×{rows}");
    fake.device_sends(b"\x1b[18t\x1b[999;999H\x1b[6n");
    let expected = format!("\x1b[8;{rows};{cols}t\x1b[{rows};{cols}R").into_bytes();
    until(&mut harness, "answered", |_| fake.written() == expected);
}

#[test]
fn a_double_click_takes_a_word_of_the_screen() {
    let fake = FakePorts::with(&["COM7"]);
    let mut settings = on("COM7");
    settings.general.terminal = true;
    let path = test_settings("terminal-word");
    settings.save(&path).expect("settings written");
    // A frame every sixtieth of a second, as on a screen: by default the harness's clock moves a
    // quarter of a second a frame, and two clicks are never within a double click's delay.
    let fake_link = fake.clone();
    let mut harness = Harness::builder().with_size(egui::vec2(1100.0, 720.0)).with_step_dt(1.0 / 60.0).build_eframe(move |cc| App::new(&cc.egui_ctx, Some(path.clone()), Machine::of(fake_link.clone()), clock));
    harness.get_by_label("Connecter").click();
    until(&mut harness, "connected", |app| app.sessions[0].is_open());
    fake.device_sends(b"ls /dev/ttyUSB0 -l");
    until(&mut harness, "drawn", |app| screen(app).contains("ttyUSB0"));
    let at = harness.state().sessions[0].terminal_view.point((0, 8));
    harness.hover_at(at);
    // egui counts clicks from its clock's start: a second goes by, for the first two clicks not to
    // count as a third.
    for _ in 0..60 {
        harness.step();
    }
    let selected = |harness: &Harness<'static, App>| {
        let app = harness.state();
        app.sessions[0].terminal_view.selected_text(app.sessions[0].terminal.as_ref().expect("the screen"))
    };
    for (clicks, expected) in [(2, "/dev/ttyUSB0"), (3, "ls /dev/ttyUSB0 -l")] {
        for _ in 0..clicks {
            for pressed in [true, false] {
                harness.event(egui::Event::PointerButton { pos: at, button: egui::PointerButton::Primary, pressed, modifiers: egui::Modifiers::NONE });
                harness.step();
            }
        }
        assert_eq!(selected(&harness).as_deref(), Some(expected), "{clicks} clicks");
        for _ in 0..60 {
            harness.step();
        }
    }
}

/// Read in the console's font itself: a character drawn by a font of its family that is not its
/// first would not be as wide as a cell.
#[test]
fn the_lines_and_corners_have_their_glyph_in_the_console_s_font() {
    use ab_glyph::Font;
    let fonts = egui::FontDefinitions::default();
    let name = &fonts.families[&egui::FontFamily::Monospace][0];
    let face = ab_glyph::FontRef::try_from_slice_and_index(&fonts.font_data[name].font, fonts.font_data[name].index).expect(name);
    // The double lines and the blocks of other menus, written as codes: the test of the window's
    // text reads this file.
    let others = "\u{2550}\u{2551}\u{2554}\u{2557}\u{255a}\u{255d}\u{2580}\u{2584}\u{2588}\u{2591}\u{2593}";
    let missing: String = crate::terminal::LINE_DRAWING.iter().map(|&c| terminal_view::drawn_as(c)).chain(others.chars()).filter(|&c| face.glyph_id(c).0 == 0).collect();
    assert!(missing.is_empty(), "the console's font lacks {missing}");
    // Ten of the set are drawn as another character, the lines and corners as themselves.
    let replaced = crate::terminal::LINE_DRAWING.iter().filter(|&&c| terminal_view::drawn_as(c) != c).count();
    assert_eq!(replaced, 10);
    assert_eq!(terminal_view::drawn_as('\u{250c}'), '\u{250c}');
}
