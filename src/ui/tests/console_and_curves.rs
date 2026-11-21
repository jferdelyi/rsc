//! The console and the curves: the auto scroll, a selection copied, an icon button's tooltip;
//! the curves made, paused, cleared, their axis and chart kept in the profile, a curve hidden.

use super::*;

#[test]
fn the_auto_scroll_switch_follows_the_console_and_brings_it_down() {
    let fake = FakePorts::with(&["COM7"]);
    let mut harness = harness(&fake, on("COM7"), "follow");
    harness.get_by_label("Connecter").click();
    until(&mut harness, "connected", |app| app.sessions[0].is_open());
    let lines: String = (0..300).map(|n| format!("line {n}\n")).collect();
    fake.device_sends(lines.as_bytes());
    until(&mut harness, "received", |app| console_text(app, 0).contains("line 299"));
    harness.run();
    let bottom = harness.state().sessions[0].view.offset();
    assert!(bottom > 0.0 && harness.state().sessions[0].view.follows());
    // Up in the console, as the minimap takes it: the following stops, and a new line does not move the view.
    harness.state_mut().sessions[0].view.jump_to(0.0);
    harness.run();
    assert!(!harness.state().sessions[0].view.follows());
    fake.device_sends(b"one more\n");
    until(&mut harness, "one more", |app| console_text(app, 0).contains("one more"));
    harness.run();
    assert_eq!(harness.state().sessions[0].view.offset(), 0.0, "the view stays where it was read");
    // Switched on again: down to the last line.
    harness.get_by_label("Défilement auto").click();
    harness.run();
    let view = &harness.state().sessions[0].view;
    assert!(view.follows() && view.offset() >= bottom, "{} from {bottom}", view.offset());
}

#[test]
fn an_icon_button_says_what_it_does_on_hover() {
    let fake = FakePorts::with(&["COM7"]);
    let mut harness = harness(&fake, on("COM7"), "tooltip");
    harness.run();
    let tip = "Copier la sélection, ou toute la console s'il n'y en a pas";
    assert!(harness.query_by_label(tip).is_none());
    harness.get_by_label("Copier").hover();
    harness.run();
    assert!(harness.query_by_label(tip).is_some(), "even greyed out, it says what it does");
}

#[test]
fn a_drag_selects_and_copy_takes_the_selection() {
    let fake = FakePorts::with(&["COM7"]);
    let mut harness = harness(&fake, on("COM7"), "select");
    harness.get_by_label("Connecter").click();
    until(&mut harness, "connected", |app| app.sessions[0].is_open());
    harness.state_mut().sessions[0].console.clear();
    fake.device_sends(b"first line\nsecond line\n");
    until(&mut harness, "received", |app| console_text(app, 0).contains("second line"));
    harness.run();
    let (from, to) = {
        let view = &harness.state().sessions[0].view;
        (view.gap(0, 6), view.gap(1, 6))
    };
    harness.hover_at(from);
    harness.drag_at(from);
    harness.step();
    harness.hover_at(to);
    harness.step();
    harness.drop_at(to);
    harness.step();
    let app = harness.state();
    assert_eq!(app.sessions[0].view.selected_text(&app.sessions[0].console, Shown::default()).as_deref(), Some("line\nsecond"));
    harness.event(egui::Event::Copy);
    harness.step();
    let copied = harness.output().platform_output.commands.iter().any(|command| matches!(command, egui::OutputCommand::CopyText(text) if text == "line\nsecond"));
    assert!(copied, "{:?}", harness.output().platform_output.commands);
}

#[test]
fn lines_of_measures_make_curves_only_when_asked() {
    let fake = FakePorts::with(&["COM7"]);
    let mut harness = harness(&fake, on("COM7"), "curves");
    harness.get_by_label("Connecter").click();
    until(&mut harness, "connected", |app| app.sessions[0].is_open());
    fake.device_sends(b"temp=21.5 hum=40\r\n");
    until(&mut harness, "received", |app| console_text(app, 0).contains("hum=40"));
    harness.run();
    assert!(harness.state().sessions[0].plot.is_empty(), "no curve while the box is not ticked");
    harness.get_by_label("Courbes").click();
    harness.run();
    assert!(harness.state().settings.general.plot, "a bare port: the general profile");
    fake.device_sends(b"temp=22 hum=41\r\nI (5140) devices: Init\r\n");
    until(&mut harness, "log line", |app| console_text(app, 0).contains("devices: Init"));
    harness.run();
    let plot = &harness.state().sessions[0].plot;
    let names: Vec<&str> = plot.curves().iter().map(|curve| curve.name.as_str()).collect();
    assert_eq!(names, ["temp", "hum"]);
    assert_eq!(plot.curves()[0].points().len(), 1, "the log line made no point");
}

#[test]
fn a_pause_stops_the_curves_and_clearing_empties_them() {
    let fake = FakePorts::with(&["COM7"]);
    let mut settings = on("COM7");
    settings.general.plot = true;
    let mut harness = harness(&fake, settings, "pause");
    harness.get_by_label("Connecter").click();
    until(&mut harness, "connected", |app| app.sessions[0].is_open());
    fake.device_sends(b"1 2\r\n");
    until(&mut harness, "curves", |app| app.sessions[0].plot.curves().len() == 2);
    harness.get_by_label("Pause").click();
    harness.run();
    assert!(harness.state().sessions[0].plot_paused);
    fake.device_sends(b"3 4\r\n");
    until(&mut harness, "received", |app| console_text(app, 0).contains("3 4"));
    harness.run();
    assert_eq!(harness.state().sessions[0].plot.curves()[0].points().len(), 1, "paused: no point added");
    harness.get_by_label("Reprendre").click();
    harness.run();
    fake.device_sends(b"5 6\r\n");
    until(&mut harness, "resumed", |app| app.sessions[0].plot.curves()[0].points().len() == 2);
    harness.get_by_label("Effacer les courbes").click();
    harness.run();
    assert!(harness.state().sessions[0].plot.is_empty());
}

/// Curves of measures on COM7, the curves shown: temp from 21 to 22, pression from 1018 to 1016.
fn on_measures(name: &str) -> (FakePorts, Harness<'static, App>) {
    let fake = FakePorts::with(&["COM7"]);
    let mut settings = on("COM7");
    settings.general.plot = true;
    let mut harness = harness(&fake, settings, name);
    harness.get_by_label("Connecter").click();
    until(&mut harness, "connected", |app| app.sessions[0].is_open());
    fake.device_sends(b"temp=21 pression=1018\r\ntemp=22 pression=1016\r\n");
    until(&mut harness, "curves", |app| app.sessions[0].plot.curves().iter().all(|curve| curve.points().len() == 2) && app.sessions[0].plot.curves().len() == 2);
    harness.run();
    (fake, harness)
}

#[test]
fn the_axis_and_the_chart_of_each_curve_are_chosen_in_the_curves_and_kept_in_the_profile() {
    let (_fake, mut harness) = on_measures("places");
    harness.get_by_label("N°").click();
    harness.run();
    assert_eq!(harness.state().settings.general.plot_x, XAxis::Number, "a bare port: the general profile");
    assert_eq!(harness.state().sessions[0].plot_view.charts, 1);
    harness.get_by_label("Haut ou bas").click();
    harness.run();
    // A row a curve, in their order: pression's is the second.
    harness.get_all_by_label("Bas").nth(1).expect("pression's").click();
    harness.run();
    assert_eq!(harness.state().settings.general.plot_below, ["pression"]);
    assert_eq!(harness.state().sessions[0].plot_view.charts, 2);
    harness.get_all_by_label("Haut").nth(1).expect("pression's").click();
    harness.run();
    assert!(harness.state().settings.general.plot_below.is_empty());
    assert_eq!(harness.state().sessions[0].plot_view.charts, 1);
}

#[test]
fn a_curve_hidden_by_the_legend_counts_no_more_in_the_scale() {
    let (_fake, mut harness) = on_measures("hidden");
    let scale = |app: &App| app.sessions[0].plot_view.shown_y[0].expect("drawn");
    let before = scale(harness.state());
    assert!(before.0 < 21.0 && before.1 > 1018.0, "both curves: {before:?}");
    // Its entry in the legend.
    harness.get_by_label("pression").click();
    harness.run();
    let after = scale(harness.state());
    assert!(after.0 > 20.0 && after.1 < 23.0, "temp alone, 21 to 22: {after:?}");
}
