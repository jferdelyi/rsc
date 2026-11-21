//! What is run by hand (ignored): the frames measured, and the captures of the window.

use super::*;

/// The median, the quartiles, the least and the most of the values (quartiles between the
/// closest ranks, as Excel's QUARTILE.INC).
fn figures(values: &[f64]) -> String {
    if values.is_empty() {
        return "no run".to_owned();
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let at = |share: f64| {
        let place = share * (sorted.len() - 1) as f64;
        let (low, high) = (place.floor() as usize, place.ceil() as usize);
        sorted[low] + (sorted[high] - sorted[low]) * (place - low as f64)
    };
    format!("median {:.3}, quartiles {:.3} to {:.3}, least {:.3}, most {:.3} ({} runs)", at(0.5), at(0.25), at(0.75), sorted[0], sorted[sorted.len() - 1], sorted.len())
}

/// The time of a frame of the window, measured in the harness: egui's pass of the application,
/// its accessibility tree included, without tessellation or graphics card. The console keeps
/// 10 000, 100 000, then 1 000 000 lines of an ESP-IDF journal; nothing comes, or 1 536 bytes come
/// a frame (921 600 bauds at 60 frames a second), received in the timed part as the window
/// receives them. Run by hand, optimized:
///
///     cargo test --release --lib measure_frames -- --ignored --nocapture --test-threads=1
///
/// 120 frames a run, a warm-up run, then RSC_RUNS (15 by default); the raw values go to
/// frames.csv in RSC_MEASURES (build/measures by default). Nothing is asserted of the times.
/// Runs of 120 frames, nothing coming ("idle") or a piece of `more` a frame ("921600"), given to
/// the window by `feed`; a warm-up run, then `runs`. Their times go to `csv`, under `label`.
fn timed_frames(harness: &mut Harness<'static, App>, label: &str, runs: usize, more: &[u8], feed: fn(&mut App, &[u8]), csv: &mut String) {
    const FRAMES: usize = 120;
    const RUN_BOUND: Duration = Duration::from_secs(60);
    const PIECE: usize = 1536;
    let mut pieces = more.chunks(PIECE).cycle();
    for case in ["idle", "921600"] {
        let mut per_run = Vec::new();
        for run in 0..=runs {
            let start = Instant::now();
            for _ in 0..FRAMES {
                if case == "921600" {
                    feed(harness.state_mut(), pieces.next().expect("pieces without end"));
                }
                harness.step();
            }
            let took = start.elapsed();
            let ms = took.as_secs_f64() * 1e3 / FRAMES as f64;
            *csv += &format!("{label};{case};{run};{};{FRAMES};{}\n", u8::from(run == 0), format!("{ms:.3}").replace('.', ","));
            if run > 0 {
                per_run.push(ms);
            }
            if took > RUN_BOUND {
                println!("{label}, {case}: a run took {took:?}, past {RUN_BOUND:?}: the series ends there");
                break;
            }
        }
        println!("frames, {label}, {case}: ms a frame {}", figures(&per_run));
    }
}

#[test]
#[ignore = "a measure: run by hand in an optimized build"]
fn measure_frames() {
    let runs: usize = std::env::var("RSC_RUNS").ok().and_then(|runs| runs.parse().ok()).unwrap_or(15);
    let line = |n: usize| {
        let ms = 14_964 + n * 7;
        match n % 4 {
            0 => format!("\x1b[0;32mI ({ms}) lte: Checking lte link\x1b[0m\r\n"),
            1 => format!("\x1b[0;32mI ({ms}) lte: Network registration: NET=2,GPRS=4,LTE=2\x1b[0m\r\n"),
            2 => format!("\x1b[0;32mI ({ms}) sampling: Readings in buffer {}/0\x1b[0m\r\n", n % 10),
            _ => format!("\x1b[0;33mW ({ms}) upload: Queue is full\x1b[0m\r\n"),
        }
    };
    let folder = std::env::var_os("RSC_MEASURES").map_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("build").join("measures"), PathBuf::from);
    fs::create_dir_all(&folder).unwrap();
    let mut csv = String::from("lines;case;run;warmup;frames;ms_per_frame\n");
    for lines in [10_000, 100_000, 1_000_000] {
        let fake = FakePorts::with(&["COM7"]);
        let mut settings = on("COM7");
        settings.general.scrollback = lines;
        let mut harness = harness(&fake, settings, &format!("frames-{lines}"));
        // Filled as the device would, without the thread of the port: what is timed is the frame.
        let journal: Vec<u8> = (0..lines).flat_map(|n| line(n).into_bytes()).collect();
        {
            let console = &mut harness.state_mut().sessions[0].console;
            for piece in journal.chunks(64 * 1024) {
                console.receive(piece, clock());
            }
            assert_eq!(console.lines().len(), lines, "the check: every line of the journal is kept");
        }
        harness.run();
        let more: Vec<u8> = (lines..lines + 60_000).flat_map(|n| line(n).into_bytes()).collect();
        timed_frames(&mut harness, &lines.to_string(), runs, &more, |app, piece| app.sessions[0].console.receive(piece, clock()), &mut csv);
    }
    // The terminal mode: a screen full of the journal's colours, then its pieces as they come.
    let fake = FakePorts::with(&["COM7"]);
    let mut settings = on("COM7");
    settings.general.terminal = true;
    let mut harness = harness(&fake, settings, "frames-terminal");
    harness.run();
    let journal: Vec<u8> = (0..10_000).flat_map(|n| line(n).into_bytes()).collect();
    harness.state_mut().sessions[0].terminal.as_mut().expect("the screen").receive(&journal);
    harness.run();
    let (rows, cols) = harness.state().sessions[0].terminal.as_ref().expect("the screen").size();
    let written = screen(harness.state()).lines().filter(|row| !row.is_empty()).count();
    assert!(written + 1 >= usize::from(rows), "the check: a full screen, {written} rows written of {rows}");
    timed_frames(&mut harness, &format!("terminal {cols}x{rows}"), runs, &journal, |app, piece| drop(app.sessions[0].terminal.as_mut().expect("the screen").receive(piece)), &mut csv);
    fs::write(folder.join("frames.csv"), csv).unwrap();
}

/// The settings of the captures: a board on COM23 with a profile of its own, a GPS beacon beside it
/// with another, the general profile with the timestamps on.
fn capture_settings(theme: settings::Theme) -> Settings {
    let mut settings = on("COM23");
    settings.theme = theme;
    settings.general.timestamps = true;
    settings.name_device(0, "Carte Nano n°3", board()).unwrap();
    settings.add_profile("ESP32 921600", None).unwrap();
    settings.choose_profile(0, Some("ESP32 921600"));
    settings.edit_tab_profile(0, |profile| {
        profile.baud_rate = 921_600;
        profile.line_ending = LineEnding::CrLf;
        profile.font_size = 14.0;
    });
    settings.add_profile("GPS 9600", None).unwrap();
    settings.set_profile(Some("GPS 9600"), Profile { baud_rate: 9600, ..settings.general.clone() });
    settings.devices.push(settings::Device { name: "Balise GPS".into(), usb: UsbId { vid: 0x067b, pid: 0x2303, serial: None }, last_port: "COM7".into(), profile: Some("GPS 9600".into()) });
    settings
}

/// The window drawn, light and dark, into build/captures when RSC_CAPTURES is set: a check by
/// eye, not a test. Each section of the settings on the general profile, and those of the
/// profile on the board's too.
#[test]
fn captures() {
    if std::env::var_os("RSC_CAPTURES").is_none() {
        return;
    }
    let folder = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("build").join("captures");
    fs::create_dir_all(&folder).unwrap();
    for (theme, name) in [(settings::Theme::Light, "clair"), (settings::Theme::Dark, "sombre")] {
        let fake = FakePorts::default();
        fake.0.lock().unwrap().listed = vec![
            PortInfo { name: "COM23".into(), description: "CP2102N USB to UART Bridge Controller".into(), usb: Some(board()) },
            PortInfo { name: "COM4".into(), description: "USB Serial Device".into(), usb: Some(UsbId { vid: 0x0403, pid: 0x6001, serial: Some("FT77".into()) }) },
        ];
        let path = test_settings(&format!("capture-{name}"));
        capture_settings(theme).save(&path).unwrap();
        let fake_link = fake.clone();
        let mut harness = Harness::builder().wgpu().with_size(egui::vec2(1100.0, 720.0)).build_eframe(move |cc| {
            let fake = fake_link.clone();
            App::new(&cc.egui_ctx, Some(path.clone()), Machine::of(fake.clone()), clock)
        });
        harness.get_by_label("Connecter").click();
        until(&mut harness, "connected", |app| app.sessions[0].is_open());
        fake.device_sends(b"\x1b[0;32mI (312) boot: ESP-IDF v5.3 2nd stage bootloader\x1b[0m\r\n");
        fake.device_sends(b"\x1b[0;32mI (598) main: firmware 0.4.2, carte rev D\x1b[0m\r\n");
        fake.device_sends(b"\x1b[0;33mW (847) wifi: aucun point d'acces enregistre\x1b[0m\r\n");
        fake.device_sends("prêt, tapez help\r\n".as_bytes());
        until(&mut harness, "boot lines", |app| console_text(app, 0).contains("tapez help"));
        harness.state_mut().sessions[0].input = "read sensors".into();
        harness.get_by_label("Envoyer").click();
        fake.device_sends("temp 21.6 °C · hum 44 % · 1013.2 hPa\r\n".as_bytes());
        fake.device_sends(b"\x1b[0;31mE (22410) i2c: delai depasse sur 0x44\x1b[0m\r\n");
        until(&mut harness, "answer", |app| console_text(app, 0).contains("0x44"));
        harness.run();
        harness.render().unwrap().save(folder.join(format!("rsc-{name}.png"))).unwrap();
        for (n, section) in Section::ALL.into_iter().enumerate() {
            let profiles: &[(Option<&str>, &str)] = if n < 6 { &[(None, "general"), (Some("ESP32 921600"), "profil")] } else { &[(None, "general")] };
            for &(profile, which) in profiles {
                harness.state_mut().window = Some(settings_window::Window::at(section, profile));
                harness.run();
                harness.render().unwrap().save(folder.join(format!("rsc-{name}-reglages-{}-{which}.png", n + 1))).unwrap();
            }
        }
    }
    // The curves, with a clock that runs.
    for (theme, name) in [(settings::Theme::Light, "clair"), (settings::Theme::Dark, "sombre")] {
        let fake = FakePorts::with(&["COM23"]);
        let mut settings = on("COM23");
        settings.theme = theme;
        settings.general.plot = true;
        let path = test_settings(&format!("capture-courbes-{name}"));
        settings.save(&path).unwrap();
        let fake_link = fake.clone();
        let mut harness = Harness::builder().wgpu().with_size(egui::vec2(1100.0, 720.0)).build_eframe(move |cc| {
            let fake = fake_link.clone();
            App::new(&cc.egui_ctx, Some(path.clone()), Machine::of(fake.clone()), running_clock)
        });
        harness.get_by_label("Connecter").click();
        until(&mut harness, "connected", |app| app.sessions[0].is_open());
        for n in 0..120_usize {
            let x = n as f64 / 8.0;
            let line = format!("temp={:.2} hum={:.2} wind={:.2}\r\n", 21.0 + x.sin(), 40.0 + 3.0 * (x / 2.0).cos(), 12.0 + 4.0 * (x / 5.0).sin());
            fake.device_sends(line.as_bytes());
            until(&mut harness, "measure", |app| app.sessions[0].plot.curves().first().is_some_and(|curve| curve.points().len() == n + 1));
        }
        harness.run();
        harness.render().unwrap().save(folder.join(format!("rsc-{name}-courbes.png"))).unwrap();
        // Two charts, hum below on its own scale, along the numbers of the measures, all of them.
        harness.state_mut().settings.edit_tab_profile(0, |profile| {
            profile.plot_below = vec!["hum".into()];
            profile.plot_x = XAxis::Number;
        });
        harness.state_mut().sessions[0].plot_view.span = plot_view::Span::All;
        harness.run();
        harness.render().unwrap().save(folder.join(format!("rsc-{name}-courbes-empilees.png"))).unwrap();
    }
    // The terminal mode, on what real programs wrote (tests/terminal/record.sh, at 80 by 24).
    for (theme, name) in [(settings::Theme::Light, "clair"), (settings::Theme::Dark, "sombre")] {
        for program in ["menu", "couleurs", "vi"] {
            let fake = FakePorts::with(&["COM23"]);
            let mut settings = on("COM23");
            settings.theme = theme;
            settings.general.terminal = true;
            let path = test_settings(&format!("capture-terminal-{program}-{name}"));
            settings.save(&path).unwrap();
            let fake_link = fake.clone();
            let mut harness = Harness::builder().wgpu().with_size(egui::vec2(1100.0, 720.0)).build_eframe(move |cc| {
                let fake = fake_link.clone();
                App::new(&cc.egui_ctx, Some(path.clone()), Machine::of(fake.clone()), clock)
            });
            harness.get_by_label("Connecter").click();
            until(&mut harness, "connected", |app| app.sessions[0].is_open());
            let recorded = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests").join("terminal").join(format!("{program}.out"));
            fake.device_sends(&fs::read(recorded).unwrap());
            until(&mut harness, "drawn", |app| !screen(app).trim().is_empty());
            harness.run();
            harness.render().unwrap().save(folder.join(format!("rsc-{name}-terminal-{program}.png"))).unwrap();
        }
    }
}

/// The pill's menus drawn, light and dark, into build/captures when RSC_CAPTURES is set: on the
/// board, then on a bare USB port, each with its copy menu open too.
#[test]
fn captures_of_the_pill() {
    if std::env::var_os("RSC_CAPTURES").is_none() {
        return;
    }
    let folder = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("build").join("captures");
    fs::create_dir_all(&folder).unwrap();
    let ftdi = UsbId { vid: 0x0403, pid: 0x6001, serial: Some("FT77".into()) };
    for (theme, name) in [(settings::Theme::Light, "clair"), (settings::Theme::Dark, "sombre")] {
        let fake = FakePorts::default();
        fake.0.lock().unwrap().listed = vec![
            PortInfo { name: "COM23".into(), description: "CP2102N USB to UART Bridge Controller".into(), usb: Some(board()) },
            PortInfo { name: "COM4".into(), description: "USB Serial Device".into(), usb: Some(ftdi.clone()) },
        ];
        let mut settings = capture_settings(theme);
        let bare = settings.add_tab();
        settings.choose_port(bare, "COM4", Some(&ftdi));
        settings.active = 0;
        let path = test_settings(&format!("capture-pastille-{name}"));
        settings.save(&path).unwrap();
        let mut harness = Harness::builder().wgpu().with_size(egui::vec2(1100.0, 720.0)).build_eframe(move |cc| {
            let fake = fake.clone();
            App::new(&cc.egui_ctx, Some(path.clone()), Machine::of(fake.clone()), clock)
        });
        until(&mut harness, "ports listed", |app| !app.ports.is_empty());
        for (pill, which) in [("Profil Carte Nano n°3 · 3 propres", "profil-appareil"), ("Profil général", "port")] {
            harness.get_by_label(pill).click();
            harness.run();
            harness.render().unwrap().save(folder.join(format!("rsc-{name}-pastille-{which}.png"))).unwrap();
            harness.get_by_label("Copier depuis un existant").click();
            harness.run();
            harness.render().unwrap().save(folder.join(format!("rsc-{name}-pastille-{which}-copie.png"))).unwrap();
            harness.key_press(Key::Escape);
            harness.run();
            harness.state_mut().settings.active = bare;
            harness.run();
        }
    }
}

/// The "About" section drawn, light and dark, into build/captures when RSC_CAPTURES is set: the
/// list, then serialport's licence shown.
#[test]
fn captures_of_about() {
    if std::env::var_os("RSC_CAPTURES").is_none() {
        return;
    }
    let folder = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("build").join("captures");
    fs::create_dir_all(&folder).unwrap();
    let serialport = crate::licences::bundled().components.iter().find(|component| component.name == "serialport").expect("serialport");
    for (theme, name) in [(settings::Theme::Light, "clair"), (settings::Theme::Dark, "sombre")] {
        let fake = FakePorts::default();
        let settings = Settings { theme, ..Settings::default() };
        let path = test_settings(&format!("capture-a-propos-{name}"));
        settings.save(&path).unwrap();
        let mut harness = Harness::builder().wgpu().with_size(egui::vec2(1100.0, 720.0)).build_eframe(move |cc| {
            let fake = fake.clone();
            App::new(&cc.egui_ctx, Some(path.clone()), Machine::of(fake.clone()), clock)
        });
        harness.state_mut().window = Some(settings_window::Window::at(Section::About, None));
        harness.run();
        harness.render().unwrap().save(folder.join(format!("rsc-{name}-a-propos.png"))).unwrap();
        if let Some(window) = &mut harness.state_mut().window {
            window.about_search = "serialport".into();
        }
        harness.run();
        harness.get_by_label(&format!("{} · {}", serialport.label(), serialport.licence)).click();
        harness.run();
        harness.render().unwrap().save(folder.join(format!("rsc-{name}-a-propos-licence.png"))).unwrap();
    }
}

/// The README's screenshots, in English, into docs/screenshots when RSC_SCREENSHOTS is set: the
/// console dark and light, the curves in two charts, the terminal mode, the profile pill's menu.
/// Drawn at 2 pixels a point, for screens that have as many.
#[test]
fn readme_screenshots() {
    if std::env::var_os("RSC_SCREENSHOTS").is_none() {
        return;
    }
    let folder = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("docs").join("screenshots");
    fs::create_dir_all(&folder).unwrap();
    let gps = UsbId { vid: 0x067b, pid: 0x2303, serial: None };
    // A board on COM23 named, with a profile of its own; a GPS beacon on COM7 with another.
    let settings = |theme: settings::Theme, terminal: bool, plot: bool| {
        let mut settings = on("COM23");
        settings.theme = theme;
        settings.language = settings::Language::English;
        settings.general.timestamps = true;
        settings.name_device(0, "Board 3", board()).unwrap();
        settings.add_profile("ESP32 921600", None).unwrap();
        settings.choose_profile(0, Some("ESP32 921600"));
        settings.edit_tab_profile(0, |profile| {
            profile.baud_rate = 921_600;
            profile.line_ending = LineEnding::CrLf;
            profile.font_size = 14.0;
            profile.terminal = terminal;
            profile.plot = plot;
        });
        settings.add_profile("GPS 9600", None).unwrap();
        settings.set_profile(Some("GPS 9600"), Profile { baud_rate: 9600, ..settings.general.clone() });
        settings.devices.push(settings::Device { name: "GPS beacon".into(), usb: gps.clone(), last_port: "COM7".into(), profile: Some("GPS 9600".into()) });
        let beacon = settings.add_tab();
        settings.choose_port(beacon, "COM7", Some(&gps));
        settings.active = 0;
        settings
    };
    let window = |settings: Settings, name: &str, height: f32, clock: fn() -> Time| {
        let fake = FakePorts::default();
        fake.0.lock().unwrap().listed = vec![
            PortInfo { name: "COM23".into(), description: "CP2102N USB to UART Bridge Controller".into(), usb: Some(board()) },
            PortInfo { name: "COM7".into(), description: "USB-Serial Controller".into(), usb: Some(gps.clone()) },
        ];
        let path = test_settings(&format!("screenshot-{name}"));
        settings.save(&path).unwrap();
        let link = fake.clone();
        let mut harness = Harness::builder().wgpu().with_size(egui::vec2(1100.0, height)).with_pixels_per_point(2.0).build_eframe(move |cc| {
            App::new(&cc.egui_ctx, Some(path.clone()), Machine::of(link.clone()), clock)
        });
        harness.get_by_label("Connect").click();
        until(&mut harness, "connected", |app| app.sessions[0].is_open());
        (harness, fake)
    };
    // The pointer out of the window, so that no arrow stands over a button, then the picture.
    let shot = |harness: &mut Harness<'static, App>, file: &str| {
        harness.event(egui::Event::PointerGone);
        harness.run();
        harness.render().unwrap().save(folder.join(file)).unwrap();
    };
    // A board starting.
    let boot = |harness: &mut Harness<'static, App>, fake: &FakePorts| {
        for line in [
            "\x1b[0;32mI (312) boot: ESP-IDF v5.3 2nd stage bootloader\x1b[0m",
            "\x1b[0;32mI (598) main: firmware 0.4.2, board rev D\x1b[0m",
            "\x1b[0;32mI (640) sensor: SHT40 found at 0x44\x1b[0m",
            "\x1b[0;32mI (721) wifi: connecting to the access point\x1b[0m",
            "\x1b[0;33mW (847) wifi: no access point saved, retrying in 5 s\x1b[0m",
            "\x1b[0;32mI (1204) mqtt: connected to the broker\x1b[0m",
            "ready, type help",
        ] {
            fake.device_sends(format!("{line}\r\n").as_bytes());
        }
        until(harness, "boot lines", |app| console_text(app, 0).contains("type help"));
    };
    // The console: a board starting, then a line sent and its answer.
    for (theme, name) in [(settings::Theme::Dark, "dark"), (settings::Theme::Light, "light")] {
        let (mut harness, fake) = window(settings(theme, false, false), &format!("console-{name}"), 640.0, clock);
        boot(&mut harness, &fake);
        harness.state_mut().sessions[0].input = "read sensors".into();
        harness.get_by_label("Send").click();
        until(&mut harness, "the line sent", |app| console_text(app, 0).contains("read sensors"));
        fake.device_sends("temp 21.6 °C · hum 44 % · 1013.2 hPa\r\n".as_bytes());
        fake.device_sends(b"\x1b[0;31mE (22410) i2c: timeout on 0x44\x1b[0m\r\n");
        until(&mut harness, "answer", |app| console_text(app, 0).contains("timeout on 0x44"));
        shot(&mut harness, &format!("console-{name}.png"));
    }
    // The curves: two charts, hum and wind below on their own scale, along the numbers of the
    // measures, the curves taking most of a taller window: each chart has the room of its whole
    // scale.
    let (mut harness, fake) = window(settings(settings::Theme::Dark, false, true), "curves", 820.0, running_clock);
    for n in 0..160_usize {
        let x = n as f64 / 8.0;
        let line = format!("temp={:.2} hum={:.2} wind={:.2}\r\n", 21.0 + x.sin(), 45.0 + 12.0 * (x / 2.0).cos(), 20.0 + 10.0 * (x / 5.0 + 1.0).sin());
        fake.device_sends(line.as_bytes());
        until(&mut harness, "measure", |app| app.sessions[0].plot.curves().first().is_some_and(|curve| curve.points().len() == n + 1));
    }
    harness.state_mut().settings.edit_tab_profile(0, |profile| {
        profile.plot_below = vec!["hum".into(), "wind".into()];
        profile.plot_x = XAxis::Number;
    });
    harness.state_mut().sessions[0].plot_view.span = plot_view::Span::All;
    harness.state_mut().sessions[0].plot_share = 0.75;
    shot(&mut harness, "curves.png");
    // The terminal mode: a device's menu on the alternate screen, in a frame of DEC's set, the line
    // chosen in reverse, and a strip of direct colours.
    let (mut harness, fake) = window(settings(settings::Theme::Dark, true, false), "terminal", 640.0, clock);
    let mut drawing = String::from("\x1b[?1049h\x1b[2J\x1b[1;1H\x1b[44;97m Board 3 · setup \x1b[K\x1b[0m");
    let items = ["Link        921600 8N1", "Wi-Fi       office-2g", "Sensors     SHT40, BMP280", "Log level   info", "Firmware    0.4.2", "Restart"];
    let (top, left, width) = (4, 6, 40);
    drawing += &format!("\x1b[{top};{left}H\x1b(0l{}k\x1b(B", "q".repeat(width - 2));
    for (row, item) in items.iter().enumerate() {
        let chosen = if row == 2 { "\x1b[7m" } else { "" };
        drawing += &format!("\x1b[{};{left}H\x1b(0x\x1b(B {chosen}{item:<w$}\x1b[0m \x1b(0x\x1b(B", top + 1 + row, w = width - 4);
    }
    drawing += &format!("\x1b[{};{left}H\x1b(0m{}j\x1b(B", top + 1 + items.len(), "q".repeat(width - 2));
    drawing += &format!("\x1b[{};{left}H\x1b[2mUp and down to move, Enter to choose, Esc to leave\x1b[0m", top + 3 + items.len());
    drawing += &format!("\x1b[{};{left}H", top + 5 + items.len());
    for column in 0..60_u32 {
        let (r, g, b) = (255 - column * 4, 60 + column * 3, column * 4);
        drawing += &format!("\x1b[48;2;{r};{g};{b}m ");
    }
    // The cursor hidden, as a menu does.
    drawing += "\x1b[0m\x1b[?25l";
    fake.device_sends(drawing.as_bytes());
    until(&mut harness, "drawn", |app| screen(app).contains("Log level"));
    shot(&mut harness, "terminal.png");
    // The profile pill's menu, on the named board.
    let (mut harness, fake) = window(settings(settings::Theme::Dark, false, false), "pill", 640.0, clock);
    boot(&mut harness, &fake);
    harness.get_by_label("Profile ESP32 921600").click();
    harness.run();
    shot(&mut harness, "profile-menu.png");
}

/// A clock that runs, 25 ms at each look: the curves of the captures spread over time.
fn running_clock() -> Time {
    static MILLIS: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(0);
    crate::console::at(0, 0) + chrono::TimeDelta::milliseconds(MILLIS.fetch_add(25, std::sync::atomic::Ordering::Relaxed))
}
