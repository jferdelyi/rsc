//! 200 % and the least size (WCAG 1.4.4): the measures `zoom` and `looks`, the state whole and
//! where it stands, the sections, the curves and their marks, the window on a Full HD screen.

use super::*;

/// Each screen at 200 %, the window at its least size (900 by 540 points, 1800 by 1080 pixels),
/// light and dark, into `zoom-200/`; the elements partly out of the window into
/// `zoom-200-hors-fenetre.csv`; and whether Ctrl and + enlarge the window, into `zoom-clavier.txt`.
#[test]
#[ignore = "a measure of accessibility, run by hand on a machine with a GPU"]
fn zoom() {
    let folder = folder();
    let captures = folder.join("zoom-200");
    fs::create_dir_all(&captures).unwrap();
    let window = Rect::from_min_size(Pos2::ZERO, LEAST);
    let mut out = String::from("ecran;role;nom;x;y;largeur;hauteur\n");
    for theme in [Theme::Light, Theme::Dark] {
        for screen in Screen::all() {
            let (mut harness, _fake) = open(screen, theme, LEAST, 2.0, true);
            harness.render().expect("drawn").save(captures.join(format!("{}-{}.png", screen.name(), theme_name(theme)))).unwrap();
            if matches!(theme, Theme::Light) {
                for element in elements(&harness).iter().filter(|element| !element.name.is_empty() && !window.contains_rect(element.rect)) {
                    writeln!(out, "{};{:?};{};{:.0};{:.0};{:.0};{:.0}", screen.name(), element.role, cell(&element.name), element.rect.min.x, element.rect.min.y, element.rect.width(), element.rect.height()).unwrap();
                }
            }
        }
    }
    fs::write(folder.join("zoom-200-hors-fenetre.csv"), out).unwrap();
    let (mut harness, _fake) = open(Screen::Line, Theme::Light, WINDOW, 1.0, false);
    let allowed = harness.ctx.options(|options| options.zoom_with_keyboard);
    let before = harness.ctx.zoom_factor();
    harness.key_press_modifiers(Modifiers::COMMAND, Key::Plus);
    harness.run();
    let after = harness.ctx.zoom_factor();
    let text = format!("zoom_with_keyboard : {allowed}\nfacteur de zoom avant Ctrl et + : {before}\naprès : {after}\n");
    print!("{text}");
    fs::write(folder.join("zoom-clavier.txt"), text).unwrap();
}

/// The screens at 100 %, light and dark, then the focus ring at the keyboard on a tab's cross, a
/// choice in segments and the terminal's screen, into `captures/`: the look before and after a
/// change.
#[test]
#[ignore = "captures of the window, run by hand on a machine with a GPU"]
fn looks() {
    let captures = folder().join("captures");
    fs::create_dir_all(&captures).unwrap();
    let screens = [
        Screen::Line,
        Screen::Terminal,
        Screen::Curves,
        Screen::CurvesMenu,
        Screen::Pill,
        Screen::Settings(Section::Link),
        Screen::Settings(Section::Console),
        Screen::Settings(Section::Profiles),
        Screen::Settings(Section::Devices),
        Screen::Settings(Section::Look),
    ];
    for theme in [Theme::Light, Theme::Dark] {
        for screen in screens {
            let (mut harness, _fake) = open(screen, theme, WINDOW, 1.0, true);
            harness.render().expect("drawn").save(captures.join(format!("{}-{}.png", screen.name(), theme_name(theme)))).unwrap();
        }
        for (screen, name, file) in [(Screen::Line, "Fermer l'onglet", "focus-croix"), (Screen::Settings(Section::Link), "8", "focus-choix"), (Screen::Terminal, "Écran du terminal", "focus-terminal")] {
            let (mut harness, _fake) = open(screen, theme, WINDOW, 1.0, true);
            unfocus(&mut harness);
            for _ in 0..MOST_TABS {
                if focused(&harness).is_some_and(|element| element.name == name) {
                    break;
                }
                harness.key_press(Key::Tab);
                harness.run();
            }
            harness.render().expect("drawn").save(captures.join(format!("{file}-{}.png", theme_name(theme)))).unwrap();
        }
    }
}

/// The state of the link is whole, and over no switch.
#[test]
fn the_state_line_is_whole_at_the_least_size() {
    let (harness, _fake) = open(Screen::Line, Theme::Light, LEAST, 1.0, false);
    let all = elements(&harness);
    let state = label(&all, "Connecté à");
    assert!(whole(&harness, state), "« {} », cut short: {:?}", state.value, state.rect);
    for switch in all.iter().filter(|element| element.role == Role::CheckBox) {
        assert!(!state.rect.shrink(0.5).intersects(switch.rect), "« {} » over the switch « {} »", state.value, switch.name);
    }
}

/// At the least size, the state goes up to the row of the tabs, whole, and the switches hold on one
/// row.
#[test]
fn the_state_goes_up_to_the_tabs_at_the_least_size() {
    let (harness, _fake) = open(Screen::Line, Theme::Light, LEAST, 1.0, false);
    let all = elements(&harness);
    let state = label(&all, "Connecté à");
    assert!(state_on_tabs_row(&all), "the state not on the row of the tabs: {:?}", state.rect);
    assert!(whole(&harness, state), "« {} », cut short", state.value);
    let switches: Vec<&Element> = all.iter().filter(|element| element.role == Role::CheckBox && ["Horodatage", "Défilement auto", "Affichage hex", "Courbes", "Terminal"].contains(&element.name.as_str())).collect();
    assert_eq!(switches.len(), 5, "the five switches");
    assert!(switches.iter().all(|switch| (switch.rect.center().y - switches[0].rect.center().y).abs() < 2.0), "the switches on one row");
}

/// With room, the state stays with the switches; when the tabs fill their row, it keeps a row of
/// its own under the connection bar, whole.
#[test]
fn the_state_stays_under_the_connection_bar_otherwise() {
    let (harness, _fake) = open(Screen::Line, Theme::Light, WINDOW, 1.0, false);
    assert!(!state_on_tabs_row(&elements(&harness)), "with room, the state goes up");
    let (mut harness, _fake) = open(Screen::Line, Theme::Light, LEAST, 1.0, false);
    let ctx = harness.ctx.clone();
    for _ in 0..4 {
        harness.state_mut().add_tab(&ctx);
    }
    harness.state_mut().settings.active = 0;
    harness.run();
    let all = elements(&harness);
    let state = label(&all, "Connecté à");
    assert!(!state_on_tabs_row(&all), "six tabs fill their row, yet the state goes up: {:?}", state.rect);
    assert!(whole(&harness, state), "« {} », cut short", state.value);
}

/// Each section of the settings, brought into view, shows above their bottom bar.
#[test]
fn every_settings_section_can_be_reached_at_the_least_size() {
    let (mut harness, _fake) = open(Screen::Settings(Section::Link), Theme::Light, LEAST, 1.0, false);
    for section in Section::ALL {
        let title = section.title();
        harness.get_by(|node| node.role() == Role::Button && node.label().as_deref() == Some(title)).scroll_to_me();
        harness.run();
        let all = elements(&harness);
        let close = all.iter().find(|element| element.role == Role::Button && element.name == "Fermer").expect("« Fermer »");
        let item = all.iter().find(|element| element.role == Role::Button && element.name == title).expect("the section");
        assert!(item.rect.bottom() <= close.rect.top() - 4.0, "« {title} » under the settings' bottom bar: {:?}, « Fermer » at {:?}", item.rect, close.rect);
    }
}

/// At the window's size by default, the list of the sections holds whole, without scrolling: with
/// the sections "Profils" and "Appareils", « À propos » went under its bottom (20/11/2026).
#[test]
fn the_list_of_the_sections_holds_at_the_size_by_default() {
    let (harness, _fake) = open(Screen::Settings(Section::Link), Theme::Light, WINDOW, 1.0, false);
    let all = elements(&harness);
    let list = all.iter().find(|element| element.role == Role::Button && element.name == "Liaison").expect("the list of the sections").rect;
    let bars: Vec<Rect> = all.iter().filter(|element| element.role == Role::ScrollBar && element.rect.center().x < list.right() + 20.0).map(|element| element.rect).collect();
    let last = all.iter().find(|element| element.role == Role::Button && element.name == "À propos").expect("« À propos »").rect;
    assert!(bars.is_empty(), "the list scrolls: {bars:?}; « À propos » at {last:?}");
}

/// The terminal's line of shortcuts is whole.
#[test]
fn the_terminal_shortcuts_are_whole_at_the_least_size() {
    let (harness, _fake) = open(Screen::Terminal, Theme::Light, LEAST, 1.0, false);
    let all = elements(&harness);
    let line = label(&all, "Le clavier va à l'appareil");
    assert!(whole(&harness, line), "cut short: {:?}", line.rect);
}

/// The words of a profile's card and of a device's card, in the settings, are whole.
#[test]
fn the_cards_of_a_profile_and_of_a_device_are_whole_at_the_least_size() {
    let (harness, _fake) = open(Screen::Settings(Section::Profiles), Theme::Light, LEAST, 1.0, false);
    let all = elements(&harness);
    for words in ["115200 8N1 · LF", "Utilisé par Carte Nano n°3"] {
        let text = label(&all, words);
        assert!(whole(&harness, text), "« {} », cut short: {:?}", text.value, text.rect);
    }
    let (harness, _fake) = open(Screen::Settings(Section::Devices), Theme::Light, LEAST, 1.0, false);
    let all = elements(&harness);
    for words in ["branché sur COM23", "10C4:EA60"] {
        let text = label(&all, words);
        assert!(whole(&harness, text), "« {} », cut short: {:?}", text.value, text.rect);
    }
}

/// The curves at the least size, two charts: at a small size, a single view is imposed. « Les deux » is not offered and nothing leaves the window; the console alone keeps 3
/// rows; the curves alone keep each chart's height and legend, and scroll each whole into view
/// above the send line.
#[test]
fn the_curves_keep_their_room_at_the_least_size() {
    let window = Rect::from_min_size(Pos2::ZERO, LEAST);
    let (mut harness, _fake) = open(Screen::Curves, Theme::Light, LEAST, 1.0, false);
    // One chart, the curves alone, scrolled to their end by the wheel: the text of the marks of the
    // horizontal axis goes a little below the axis; 4 points of it stay above the curves' frame's
    // bottom (its margin of 10 and the space of 8 above the send line), not cut (seen at 200 %,
    // 18/11/2026).
    display(&mut harness, "Courbes");
    let chart = elements(&harness).into_iter().find(|element| element.name.starts_with("Courbes :")).expect("the chart");
    harness.hover_at(chart.rect.center());
    harness.event(egui::Event::MouseWheel { unit: egui::MouseWheelUnit::Point, delta: egui::vec2(0.0, -1000.0), phase: egui::TouchPhase::Move, modifiers: Modifiers::NONE });
    harness.run();
    let all = elements(&harness);
    let chart = all.iter().find(|element| element.name.starts_with("Courbes :")).expect("the chart");
    let axis = all.iter().find(|element| element.role == Role::Unknown && element.name.is_empty() && (element.rect.top() - chart.rect.bottom()).abs() < 2.0 && (element.rect.width() - chart.rect.width()).abs() < 1.0).expect("the marks under the chart");
    let frame = all.iter().find(|element| element.role == Role::TextInput).expect("the send line").rect.top() - 18.0;
    let bar = all.iter().find(|element| element.name == "Exporter en CSV").expect("the curves' bar").rect.bottom();
    assert!(axis.rect.bottom() + 4.0 <= frame, "the marks cut: they end at {}, the curves' frame at {frame}; the bar ends at {bar}, the chart is {:?}", axis.rect.bottom(), chart.rect);
    harness.state_mut().settings.edit_tab_profile(0, |profile| profile.plot_below = vec!["hum".into()]);
    harness.run();
    let both = elements(&harness).into_iter().find(|element| element.role == Role::RadioButton && element.name == "Les deux").expect("the choice « Les deux »");
    assert!(both.disabled, "« Les deux » offered at the least size");
    for choice in ["Console", "Courbes"] {
        display(&mut harness, choice);
        let all = elements(&harness);
        let find = |role: Role, name: &str| all.iter().find(|element| element.role == role && element.name == name).unwrap_or_else(|| panic!("« {name} »"));
        for name in ["Réglages", "Copier", "Effacer", "Envoyer"] {
            assert!(window.contains_rect(find(Role::Button, name).rect), "{choice}: « {name} » out of the window");
        }
        let send = find(Role::TextInput, "Ligne à envoyer").rect;
        if choice == "Console" {
            // From the row of the switches to the send line: the console's frame and 3 rows.
            let row = harness.ctx.fonts_mut(|fonts| fonts.row_height(&egui::FontId::monospace(13.0)));
            let console = send.top() - find(Role::CheckBox, "Terminal").rect.bottom();
            assert!(console + 1.0 >= 24.0 + 3.0 * row, "the console keeps {console} points, not 3 rows");
            continue;
        }
        let charts: Vec<&Element> = all.iter().filter(|element| element.name.starts_with("Courbes :")).collect();
        assert_eq!(charts.len(), 2, "two charts");
        for chart in &charts {
            assert!(chart.rect.height() + 0.5 >= 60.0, "« {} » squeezed to {}", chart.name, chart.rect.height());
        }
        for legend in all.iter().filter(|element| element.role == Role::CheckBox && ["temp", "hum", "wind"].contains(&element.name.as_str())) {
            assert!(charts.iter().any(|chart| chart.rect.expand(1.0).contains_rect(legend.rect)), "the legend « {} » out of its chart", legend.name);
        }
        let names: Vec<String> = charts.iter().map(|chart| chart.name.clone()).collect();
        for name in names {
            harness.get_by(|node| node.label().as_deref() == Some(name.as_str())).scroll_to_me();
            harness.run();
            let all = elements(&harness);
            let chart = all.iter().find(|element| element.name == name).expect("the chart");
            let send = all.iter().find(|element| element.role == Role::TextInput).expect("the send line");
            let bar = all.iter().find(|element| element.name == "Exporter en CSV").expect("the curves' bar");
            assert!(chart.rect.top() >= bar.rect.bottom() && chart.rect.bottom() <= send.rect.top(), "« {name} » not brought whole into view: {:?}, the curves' bar ends at {}, the send line begins at {}", chart.rect, bar.rect.bottom(), send.rect.top());
        }
    }
}

/// With room, « Les deux » is offered and shown, with the separation; « Console » and « Courbes »
/// show each alone.
#[test]
fn both_are_shown_when_the_window_has_room() {
    let (mut harness, _fake) = open(Screen::Curves, Theme::Light, WINDOW, 1.0, false);
    // The console's area, not the choice « Console » that bears its name.
    let shown = |harness: &Harness<'static, App>| {
        let all = elements(harness);
        let console = all.iter().any(|element| element.role == Role::Log && element.name == "Console");
        let has = |name: &str| all.iter().any(|element| element.name.starts_with(name));
        (console, has("Séparation console et courbes"), has("Courbes :"))
    };
    let both = elements(&harness).into_iter().find(|element| element.role == Role::RadioButton && element.name == "Les deux").expect("the choice « Les deux »");
    assert!(!both.disabled, "« Les deux » offered with room");
    assert_eq!(shown(&harness), (true, true, true), "both at first");
    display(&mut harness, "Console");
    assert_eq!(shown(&harness), (true, false, false), "the console alone");
    display(&mut harness, "Courbes");
    assert_eq!(shown(&harness), (false, false, true), "the curves alone");
    display(&mut harness, "Les deux");
    assert_eq!(shown(&harness), (true, true, true), "both again");
}

/// The least window holds whole on a 1920 × 1080 screen at 200 %: 960 × 540 points, less the
/// taskbar (48 points, Windows 11's), the title bar of a window that is not maximized (29 points: 45
/// pixels of title and 13 of frame at 192 dpi, `GetSystemMetricsForDpi`) and a point of border
/// (measured 19/11/2026; 24 and 16 were written here before, guessed).
#[test]
fn the_window_fits_a_full_hd_screen_at_200_percent() {
    let least = std::hint::black_box(super::super::MIN_SIZE);
    assert!(least.x <= 960.0 - 1.0 && least.y <= 540.0 - 48.0 - 29.0 - 1.0, "{least:?}");
}

/// The least window holds whole on a 1366 × 768 screen at 150 %, the usual setting of such a laptop:
/// 910 × 512 points, less the taskbar (48 points, Windows 11's: 60 pixels at 125 %), the title bar
/// of a window that is not maximized (30 points: 34 pixels of title and 11 of frame at 144 dpi,
/// `GetSystemMetricsForDpi`) and a point of border (measured 19/11/2026).
#[test]
fn the_window_fits_a_1366_768_screen_at_150_percent() {
    let least = std::hint::black_box(super::super::MIN_SIZE);
    assert!(least.x <= 1366.0 / 1.5 - 1.0 && least.y <= 768.0 / 1.5 - 48.0 - 30.0 - 1.0, "{least:?}");
}

/// On a screen too low for the window with its title bar and the taskbar, RSC asks at its start to
/// be maximized: the system then gives it the room the taskbar leaves. eframe brings the size of the
/// start (1100 × 720) down to the whole screen only, 910 × 512 on a 1366 × 768 at 150 %: 78 points
/// too high (19/11/2026). On a screen with room, the window is left as it is.
#[test]
fn the_first_start_fits_a_small_screen() {
    let cases = [
        // 1366 × 768 at 150 %, the start brought down by eframe to the screen.
        (egui::vec2(910.0, 512.0), egui::vec2(910.0, 512.0), true),
        // 1920 × 1080 at 200 %.
        (egui::vec2(960.0, 540.0), egui::vec2(960.0, 540.0), true),
        // 1920 × 1200 at 125 %, the machine of the tests in the real window.
        (egui::vec2(1536.0, 960.0), egui::vec2(1100.0, 720.0), false),
    ];
    for (screen, window, maximized) in cases {
        let (mut harness, _fake) = open(Screen::Line, Theme::Light, WINDOW, 1.0, false);
        // The screen as the system tells it, from the next frame on.
        let viewport = harness.input_mut().viewports.get_mut(&egui::ViewportId::ROOT).expect("the window");
        viewport.monitor_size = Some(screen);
        viewport.inner_rect = Some(Rect::from_min_size(Pos2::ZERO, window));
        let mut asked = false;
        for _ in 0..3 {
            harness.step();
            asked |= harness.output().viewport_output.get(&egui::ViewportId::ROOT).is_some_and(|output| output.commands.contains(&egui::ViewportCommand::Maximized(true)));
        }
        assert_eq!(asked, maximized, "a screen of {screen:?}, a window of {window:?}: maximized");
    }
}

/// Every chart shows two marks at least on its vertical axis, in full colour, whatever the range of
/// its values: at the least size (the curves alone) as with room (both). Seen lost on 1014 to 1018
/// in the real window (18/11/2026): egui_plot writes a mark only 20 points from the next, by steps
/// of 1, 10, 100; there, marks of 1 stood 19 points apart, and none of 10 was in view.
#[test]
fn every_chart_shows_its_vertical_marks() {
    const RANGES: [(f64, f64); 7] = [(1014.0, 1018.0), (0.0, 1.0), (0.0, 42.0), (0.001, 0.004), (990.0, 1090.0), (-5.0, 5.0), (20.9, 21.1)];
    for size in [LEAST, WINDOW] {
        let (mut harness, fake) = open(Screen::Curves, Theme::Light, size, 1.0, false);
        harness.state_mut().settings.edit_tab_profile(0, |profile| profile.plot_below = vec!["v".into()]);
        for (low, high) in RANGES {
            harness.state_mut().sessions[0].plot.clear();
            for n in 0..40_usize {
                let t = n as f64 / 39.0;
                let v = low + (high - low) * (0.5 - 0.5 * (t * std::f64::consts::PI).cos());
                fake.device_sends(format!("u={:.1} v={v}\r\n", 10.0 * t).as_bytes());
                until(&mut harness, "measure", |app| app.sessions[0].plot.curves().iter().any(|curve| curve.name == "v" && curve.points().len() == n + 1));
            }
            harness.run();
            let names: Vec<String> = elements(&harness).into_iter().filter(|element| element.name.starts_with("Courbes :")).map(|element| element.name).collect();
            assert_eq!(names.len(), 2, "two charts");
            for name in names {
                // Brought into view: at the least size, the curves scroll.
                harness.get_by(|node| node.label().as_deref() == Some(name.as_str())).scroll_to_me();
                harness.run();
                let chart = elements(&harness).into_iter().find(|element| element.name == name).expect("the chart").rect;
                let marks: Vec<(Rect, String, Color32)> = texts(&harness)
                    .into_iter()
                    .filter(|(rect, text, _)| !text.trim().is_empty() && rect.right() <= chart.left() + 1.0 && rect.left() >= chart.left() - 80.0 && chart.y_range().contains(rect.center().y))
                    .collect();
                let said: Vec<&str> = marks.iter().map(|(_, text, _)| text.as_str()).collect();
                assert!(marks.len() >= 2, "{size:?}, values from {low} to {high}: « {name} » ({:.0} points high) shows {said:?} on its vertical axis", chart.height());
                assert!(marks.iter().all(|(_, _, color)| color.a() == 255), "{size:?}, values from {low} to {high}: « {name} » fades its marks {said:?}");
            }
        }
    }
}

/// Every vertical mark is written whole and apart from the others: neither cut by the top of the
/// curves' area, nor written over a mark of the other chart. Seen cut in the real window, « 40 » over
/// a counter from 0 to 41 (19/11/2026): egui_plot writes a mark's text centred on its line, and the
/// area of the curves cut the half of it above the higher chart.
#[test]
fn every_vertical_mark_is_written_whole() {
    // The values of the higher chart (u, from 0 to the first) and of the lower one (v): those of the
    // test above, then the capture's, a counter over a pressure.
    const CASES: [(f64, f64, f64); 8] = [
        (10.0, 1014.0, 1018.0),
        (10.0, 0.0, 1.0),
        (10.0, 0.0, 42.0),
        (10.0, 0.001, 0.004),
        (10.0, 990.0, 1090.0),
        (10.0, -5.0, 5.0),
        (10.0, 20.9, 21.1),
        (41.0, 1014.0, 1018.0),
    ];
    let mut faults = Vec::new();
    for size in [LEAST, WINDOW] {
        let (mut harness, fake) = open(Screen::Curves, Theme::Light, size, 1.0, false);
        harness.state_mut().settings.edit_tab_profile(0, |profile| profile.plot_below = vec!["v".into()]);
        for (most, low, high) in CASES {
            harness.state_mut().sessions[0].plot.clear();
            for n in 0..40_usize {
                let t = n as f64 / 39.0;
                let v = low + (high - low) * (0.5 - 0.5 * (t * std::f64::consts::PI).cos());
                fake.device_sends(format!("u={:.1} v={v}\r\n", most * t).as_bytes());
                until(&mut harness, "measure", |app| app.sessions[0].plot.curves().iter().any(|curve| curve.name == "v" && curve.points().len() == n + 1));
            }
            harness.run();
            let names: Vec<String> = elements(&harness).into_iter().filter(|element| element.name.starts_with("Courbes :")).map(|element| element.name).collect();
            assert_eq!(names.len(), 2, "two charts");
            for name in &names {
                // Brought into view: at the least size, the curves scroll.
                harness.get_by(|node| node.label().as_deref() == Some(name.as_str())).scroll_to_me();
                harness.run();
                let all = elements(&harness);
                let chart = |name: &str| all.iter().find(|element| element.name == name).expect("the chart").rect;
                let charts: Vec<Rect> = names.iter().map(|name| chart(name)).collect();
                let shown = chart(name);
                let case = format!("{size:?}, u from 0 to {most}, v from {low} to {high}");
                let marks: Vec<(Rect, Rect, String)> = clipped_texts(&harness)
                    .into_iter()
                    .filter(|(rect, _, text)| {
                        !text.trim().is_empty() && charts.iter().any(|chart| rect.right() <= chart.left() + 1.0 && rect.left() >= chart.left() - 80.0 && chart.y_range().contains(rect.center().y))
                    })
                    .collect();
                for (rect, clip, text) in marks.iter().filter(|(rect, _, _)| shown.y_range().contains(rect.center().y)) {
                    if !clip.expand(0.5).contains_rect(*rect) {
                        faults.push(format!("{case}: « {text} » of « {name} » cut, written in {rect:?}, shown in {clip:?}"));
                    }
                }
                // What shows of each text, cut to where it is drawn: a text out of view covers
                // nothing (at the least size, a mark of the lower chart under the area's end was
                // taken over « Ligne », the send line's label, 19/11/2026).
                let seen: Vec<(Rect, &str)> = marks.iter().map(|(rect, clip, text)| (rect.intersect(*clip), text.as_str())).filter(|(seen, _)| seen.width() > 0.5 && seen.height() > 0.5).collect();
                for (i, (first, one)) in seen.iter().enumerate() {
                    for (second, other) in &seen[i + 1..] {
                        let both = first.intersect(*second);
                        if both.width() > 0.5 && both.height() > 0.5 {
                            faults.push(format!("{case}: « {one} » written over « {other} », seen in {first:?} and {second:?}"));
                        }
                    }
                }
            }
        }
    }
    faults.sort();
    faults.dedup();
    assert!(faults.is_empty(), "{} mark(s) not written whole:\n{}", faults.len(), faults.join("\n"));
}

/// The settings opened on a section low in their list show its row in the list's view, at the
/// least size; brought there once, the list scrolls again by the wheel.
#[test]
fn the_chosen_section_is_in_view_at_the_least_size() {
    for section in [Section::Devices, Section::About] {
        let (mut harness, _fake) = open(Screen::Settings(section), Theme::Light, LEAST, 1.0, false);
        let title = section.title();
        let row = |harness: &Harness<'static, App>| {
            let all = elements(harness);
            let item = all.iter().find(|element| element.role == Role::Button && element.name == title).expect("the section").rect;
            let top = label(&all, "Le profil général").rect.bottom();
            let close = all.iter().find(|element| element.role == Role::Button && element.name == "Fermer").expect("« Fermer »").rect.top();
            (item, top, close)
        };
        let (item, top, close) = row(&harness);
        assert!(item.top() >= top && item.bottom() <= close - 4.0, "« {title} » out of the list's view: {item:?}, the list between {top} and {close}");
        harness.hover_at(item.center());
        harness.event(egui::Event::MouseWheel { unit: egui::MouseWheelUnit::Point, delta: egui::vec2(0.0, 1000.0), phase: egui::TouchPhase::Move, modifiers: Modifiers::NONE });
        harness.run();
        let (moved, _, _) = row(&harness);
        assert!((moved.top() - item.top()).abs() > 1.0, "the list holds « {title} »: the wheel does not scroll it");
    }
}
