//! The port and its tabs: a line sent and the answer, nothing sent while closed, bytes in
//! hexadecimal, a port opened again (new baud rate, lost then back, another name), tabs at once.

use super::*;

#[test]
fn a_line_typed_goes_to_the_device_and_what_it_answers_shows() {
    let fake = FakePorts::with(&["COM7"]);
    let mut harness = harness(&fake, on("COM7"), "send");
    harness.get_by_label("Connecter").click();
    until(&mut harness, "connected", |app| app.sessions[0].is_open());
    harness.state_mut().sessions[0].input = "status".into();
    harness.get_by_label("Envoyer").click();
    until(&mut harness, "sent", |_| fake.written() == b"status\n");
    fake.device_sends(b"uptime 9 s\r\n");
    until(&mut harness, "answered", |app| console_text(app, 0).contains("uptime 9 s\n"));
    let text = console_text(harness.state(), 0);
    assert!(text.contains("> status\n"), "{text}");
    assert!(harness.state().sessions[0].input.is_empty(), "the line is emptied once sent");
}

#[test]
fn nothing_is_sent_while_the_port_is_closed() {
    let fake = FakePorts::with(&["COM7"]);
    let mut harness = harness(&fake, on("COM7"), "closed");
    harness.state_mut().sessions[0].input = "status".into();
    harness.get_by_label("Envoyer").click();
    harness.run();
    assert!(fake.written().is_empty());
    assert_eq!(harness.state().sessions[0].input, "status", "the line stays");
}

#[test]
fn hexadecimal_input_sends_its_bytes_and_says_what_is_wrong() {
    let fake = FakePorts::with(&["COM7"]);
    let mut settings = on("COM7");
    settings.general.hex_input = true;
    let mut harness = harness(&fake, settings, "hex");
    harness.get_by_label("Connecter").click();
    until(&mut harness, "connected", |app| app.sessions[0].is_open());
    harness.state_mut().sessions[0].input = "48 69 0x0D".into();
    harness.get_by_label("Envoyer").click();
    until(&mut harness, "sent", |_| fake.written() == b"Hi\r");
    harness.state_mut().sessions[0].input = "48 6".into();
    harness.get_by_label("Envoyer").click();
    harness.run();
    assert!(harness.state().sessions[0].problem.as_deref().is_some_and(|problem| problem.contains("« 6 »")));
    assert_eq!(fake.written(), b"Hi\r", "nothing more");
}

#[test]
fn a_new_baud_rate_opens_the_port_again() {
    let fake = FakePorts::with(&["COM7"]);
    let mut harness = harness(&fake, on("COM7"), "baud");
    harness.get_by_label("Connecter").click();
    until(&mut harness, "connected", |app| app.sessions[0].is_open());
    harness.state_mut().settings.general.baud_rate = 9600;
    until(&mut harness, "reopened", |app| matches!(&app.sessions[0].state, State::Open { link, .. } if link.baud_rate == 9600));
    let opened = fake.0.lock().unwrap().opened.clone();
    assert_eq!(opened.iter().map(|(_, link)| link.baud_rate).collect::<Vec<_>>(), [115_200, 9600]);
}

#[test]
fn a_lost_port_is_opened_again_when_it_is_back() {
    let fake = FakePorts::with(&["COM7"]);
    let mut harness = harness(&fake, on("COM7"), "lost");
    harness.get_by_label("Connecter").click();
    until(&mut harness, "connected", |app| app.sessions[0].is_open());
    {
        // Unplugged: gone from the list, and refusing to open while the window still lists it.
        let mut shared = fake.0.lock().unwrap();
        shared.listed.clear();
        shared.refuse = Some("device gone".into());
        shared.incoming.push_back(Err(io::Error::new(io::ErrorKind::BrokenPipe, "device unplugged")));
    }
    until(&mut harness, "waiting", |app| matches!(app.sessions[0].state, State::Waiting { .. }));
    fake.0.lock().unwrap().listed = FakePorts::with(&["COM7"]).0.lock().unwrap().listed.clone();
    until(&mut harness, "open again", |app| app.sessions[0].is_open());
    assert_eq!(fake.0.lock().unwrap().opened.len(), 2, "opened, then opened again");
    assert!(console_text(harness.state(), 0).contains("COM7 perdu : device unplugged"));
}

#[test]
fn a_named_device_back_under_another_port_is_found() {
    let fake = FakePorts::default();
    fake.0.lock().unwrap().listed = vec![usb_port("COM9")];
    let mut settings = on("COM7");
    settings.name_device(0, "Carte", board()).unwrap();
    let mut harness = harness(&fake, settings, "usb");
    until(&mut harness, "ports listed", |app| !app.ports.is_empty());
    harness.get_by_label("Connecter").click();
    until(&mut harness, "connected", |app| app.sessions[0].is_open());
    assert_eq!(fake.0.lock().unwrap().opened[0].0, "COM9");
    let app = harness.state();
    assert_eq!((app.settings.tabs[0].port.as_str(), app.settings.devices[0].last_port.as_str()), ("COM9", "COM9"));
    assert!(console_text(app, 0).contains("COM7 est devenu COM9"));
}

#[test]
fn a_bare_port_is_not_followed_elsewhere() {
    let fake = FakePorts::default();
    fake.0.lock().unwrap().listed = vec![usb_port("COM9")];
    let mut harness = harness(&fake, on("COM7"), "bare");
    until(&mut harness, "ports listed", |app| !app.ports.is_empty());
    harness.get_by_label("Connecter").click();
    harness.run();
    until(&mut harness, "tried", |app| !fake.0.lock().unwrap().opened.is_empty() || !matches!(app.sessions[0].state, State::Opening { .. }));
    assert_eq!(fake.0.lock().unwrap().opened[0].0, "COM7", "the port asked for, not the device on COM9");
}

#[test]
fn two_tabs_hold_two_ports_at_once() {
    let fake = FakePorts::with(&["COM7", "COM8"]);
    let mut harness = harness(&fake, on("COM7"), "tabs");
    harness.get_by_label("Connecter").click();
    until(&mut harness, "first open", |app| app.sessions[0].is_open());
    harness.get_by_label("Nouvel onglet").click();
    harness.run();
    assert_eq!((harness.state().sessions.len(), harness.state().settings.active), (2, 1));
    harness.state_mut().settings.tabs[1].port = "COM8".into();
    harness.run();
    harness.get_by_label("Connecter").click();
    until(&mut harness, "second open", |app| app.sessions[1].is_open());
    assert!(harness.state().sessions[0].is_open(), "the first stays open");
    let opened: Vec<String> = fake.0.lock().unwrap().opened.iter().map(|(port, _)| port.clone()).collect();
    assert_eq!(opened, ["COM7", "COM8"]);
}

#[test]
fn the_tabs_share_one_list_of_the_ports() {
    let fake = FakePorts::with(&["COM3"]);
    let mut settings = on("COM3");
    settings.add_tab();
    settings.add_tab();
    let start = Instant::now();
    let mut harness = harness(&fake, settings, "one-list");
    // Three tabs, no port open: a list at start, then one a second, for all of them.
    while start.elapsed() < Duration::from_millis(1300) {
        harness.step();
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(harness.state().sessions.len(), 3);
    assert_eq!(harness.state().ports.len(), 1, "the window has the list");
    let lists = fake.0.lock().unwrap().lists;
    assert!(lists <= 2, "{lists} lists in 1.3 s with three tabs");
}

/// RSC on a real device, on a copy of a real settings file, run by hand:
///
///     RSC_PORT=COM23 cargo test --lib real_device -- --ignored --nocapture
///
/// The copy is of RSC_SETTINGS, else of the usual file, which is not touched. The tab in front
/// connects, shows what comes for RSC_SECONDS (5 by default), sends nothing, and disconnects.
#[test]
#[ignore = "needs a device: RSC_PORT=<port>"]
fn a_real_device_opens_with_the_profile_of_its_tab() {
    let Some(port) = std::env::var("RSC_PORT").ok() else {
        eprintln!("RSC_PORT not set: nothing to do");
        return;
    };
    let seconds = std::env::var("RSC_SECONDS").ok().and_then(|value| value.parse().ok()).unwrap_or(5);
    let source = settings_path().expect("a settings file");
    let path = test_settings("real-device");
    fs::copy(&source, &path).unwrap_or_else(|error| panic!("{}: {error}", source.display()));
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1100.0, 720.0))
        .build_eframe(move |cc| App::new(&cc.egui_ctx, Some(path.clone()), Machine::system(), now));
    until(&mut harness, "ports listed", |app| app.ports.iter().any(|known| known.name == port));
    let (i, wanted) = {
        let app = harness.state();
        let i = app.active();
        let tab = &app.settings.tabs[i];
        eprintln!("ports: {:?}", app.ports.iter().map(PortInfo::label).collect::<Vec<_>>());
        eprintln!("tab {i}: port {}, device {:?}, profile {}", tab.port, tab.device, app.settings.profile_of(i).link().summary());
        (i, app.settings.profile_of(i).link())
    };
    harness.get_by_label("Connecter").click();
    until(&mut harness, "opened or refused", |app| matches!(app.sessions[i].state, State::Open { .. }) || app.sessions[i].problem.is_some());
    match &harness.state().sessions[i].state {
        State::Open { port: opened, link } => assert_eq!((opened, link), (&port, &wanted), "the port asked for, with the profile of the tab"),
        state => panic!("{port}: {state:?}, {:?}", harness.state().sessions[i].problem),
    }
    let end = Instant::now() + Duration::from_secs(seconds);
    while Instant::now() < end {
        harness.step();
        std::thread::sleep(Duration::from_millis(20));
    }
    let text = console_text(harness.state(), i);
    eprintln!("{} lines in {seconds} s:\n{}", text.lines().count(), text.lines().take(40).collect::<Vec<_>>().join("\n"));
    harness.get_by_label("Déconnecter").click();
    until(&mut harness, "closed", |app| app.sessions[i].state == State::Closed);
    eprintln!("{port} closed");
}
