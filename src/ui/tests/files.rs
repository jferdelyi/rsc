//! Files: the kinds a save window offers, the settings file of an earlier version, what is
//! written back, the name of a file, egui's memory left out, « À propos » and the licences.

use super::*;

#[test]
fn each_save_window_offers_the_kind_of_its_file_then_every_file() {
    assert_eq!(save_filters(Saved::Journal), [("Journal (*.log)", "log"), ("Tous les fichiers (*.*)", "*")]);
    assert_eq!(save_filters(Saved::Curves)[0], ("CSV (*.csv)", "csv"));
    assert_eq!(save_filters(Saved::Notices)[0], ("Texte (*.txt)", "txt"));
}

#[test]
fn a_settings_file_of_an_earlier_version_is_taken_in_and_said() {
    let fake = FakePorts::default();
    let path = test_settings("taken-in");
    fs::write(&path, r#"{"tabs":[{"port":"COM4","link":{"baud_rate":4800}}],"presets":[]}"#).unwrap();
    let mut harness = harness_at(&fake, path.clone());
    harness.run();
    let kept = path.with_file_name("settings.json.old");
    let text = console_text(harness.state(), 0);
    assert!(text.contains(&format!("Réglages d'une version précédente repris ; l'ancien fichier est gardé dans {}.", kept.display())), "{text}");
    assert!(kept.exists(), "the old file kept");
    assert_eq!(Settings::load(&path).1, Loaded::Read, "written again in this version's form");
}

#[test]
fn the_about_section_lists_the_components_and_shows_their_licence() {
    let notices = crate::licences::bundled();
    let count = notices.components.len();
    let fake = FakePorts::default();
    let mut harness = harness(&fake, Settings::default(), "about");
    harness.state_mut().window = Some(settings_window::Window::at(Section::About, None));
    harness.run();
    harness.get_by_label(&format!(
        "RSC est fait de {count} composants libres. Leurs licences demandent que leurs mentions accompagnent chaque copie : les voici, avec leurs textes."
    ));
    harness.get_by_label(&format!("{count} sur {count}"));
    let egui = notices.components.iter().find(|component| component.name == "egui").expect("egui");
    harness.get_by_label(&format!("{} · {}", egui.label(), egui.licence));
    // Searched for: serialport alone.
    if let Some(window) = &mut harness.state_mut().window {
        window.about_search = "serialport".into();
    }
    harness.run();
    harness.get_by_label(&format!("1 sur {count}"));
    assert!(harness.query_by_label(&format!("{} · {}", egui.label(), egui.licence)).is_none(), "egui is not shown");
    let serialport = notices.components.iter().find(|component| component.name == "serialport").expect("serialport");
    harness.get_by_label(&format!("{} · {}", serialport.label(), serialport.licence)).click();
    harness.run();
    // Its source, and its licence's texts.
    harness.get_by_label(&format!("Source : {}", serialport.source));
    harness.get_by_label_contains("Mozilla Public License Version 2.0");
}

#[test]
fn the_about_section_gives_rsc_s_own_licence() {
    let fake = FakePorts::default();
    let mut harness = harness(&fake, Settings::default(), "about-own");
    harness.state_mut().window = Some(settings_window::Window::at(Section::About, None));
    harness.run();
    harness.get_by_label(&format!("© {} {} · Eclipse Public License 2.0 (EPL-2.0)", crate::licences::YEAR, crate::licences::HOLDER));
    harness.get_by_label("Source : https://github.com/jferdelyi/rsc");
    assert!(harness.query_by_label_contains("Exhibit A").is_none(), "folded at first");
    harness.get_by_label("Lire la licence de RSC").click();
    harness.run();
    harness.get_by_label_contains("Exhibit A - Form of Secondary Licenses Notice");
}

#[test]
fn a_licence_text_keeps_its_lines_whole() {
    let notices = crate::licences::bundled();
    let serialport = notices.components.iter().find(|component| component.name == "serialport").expect("serialport");
    // Its package's notice: 2 lines, the first of 98 characters, longer than the section is wide.
    let notice = notices.texts_of(serialport).find(|text| text.title == "LICENSE.txt").expect("its notice");
    assert_eq!(notice.body.lines().map(|line| line.chars().count()).collect::<Vec<_>>().len(), 2);
    let fake = FakePorts::default();
    let mut harness = harness(&fake, Settings::default(), "licence-lines");
    let mut window = settings_window::Window::at(Section::About, None);
    // Searched for: in the list's view, where a click reaches it.
    window.about_search = "serialport".into();
    harness.state_mut().window = Some(window);
    harness.run();
    harness.get_by_label(&format!("{} · {}", serialport.label(), serialport.licence)).click();
    harness.run();
    let shown = harness.get_by_label(&notice.body).rect();
    let row = harness.ctx.fonts_mut(|fonts| fonts.row_height(&egui::FontId::monospace(12.0)));
    assert!(shown.height() < 2.5 * row, "{} px high, rows of {row} px: a line is cut", shown.height());
}

#[test]
fn a_file_of_an_earlier_version_is_written_again_in_the_new_form() {
    let path = test_settings("converted");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(
        &path,
        r#"{"tabs":[{"port":"COM23","equipment":"Nano","link":{"baud_rate":115200,"data_bits":8,"parity":"none","stop_bits":"one","flow_control":"none","dtr":true,"rts":true},"line_ending":"lf","preset":"Usine"}],
                "active":0,"presets":[{"name":"Usine","link":{"baud_rate":115200,"data_bits":8,"parity":"none","stop_bits":"one","flow_control":"none","dtr":true,"rts":true},"line_ending":"lf"}],
                "equipments":[{"name":"Nano","usb":{"vid":4292,"pid":60000,"serial":"0A1B"},"last_port":"COM23","preset":"Usine"}],"font_size":9.5,"theme":"system","language":"fr"}"#,
    )
    .unwrap();
    let fake = FakePorts::default();
    let mut harness = harness_at(&fake, path.clone());
    harness.run();
    let text = fs::read_to_string(&path).unwrap();
    assert!(text.contains("\"general\"") && !text.contains("\"presets\""), "{text}");
    let (settings, loaded) = Settings::load(&path);
    assert_eq!((loaded, settings.general.font_size, settings.tabs[0].device.as_deref()), (Loaded::Read, 9.5, Some("Nano")));
    assert!(!path.with_extension("json.bad").exists() && !path.with_file_name("settings.json.bad").exists(), "nothing set aside");
}

#[test]
fn a_general_profile_written_in_full_is_written_again_by_its_differences() {
    let path = test_settings("rewritten");
    let mut full = serde_json::to_value(Profile::default()).unwrap();
    full["font_size"] = serde_json::json!(9.5);
    fs::write(&path, serde_json::json!({ "general": full, "tabs": [{ "port": "COM23" }], "active": 0 }).to_string()).unwrap();
    let fake = FakePorts::default();
    harness_at(&fake, path.clone()).run();
    let text = fs::read_to_string(&path).unwrap();
    let value: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(value["general"], serde_json::json!({ "font_size": 9.5 }), "{value}");
    // In its form now: started again, nothing is written.
    harness_at(&fake, path.clone()).run();
    assert_eq!(fs::read_to_string(&path).unwrap(), text);
}

/// eframe's storage, in memory: what app.ron holds.
#[derive(Default)]
struct Stored(std::collections::BTreeMap<String, String>);

impl eframe::Storage for Stored {
    fn get_string(&self, key: &str) -> Option<String> {
        self.0.get(key).cloned()
    }

    fn set_string(&mut self, key: &str, value: String) {
        self.0.insert(key.to_owned(), value);
    }

    fn remove_string(&mut self, key: &str) {
        self.0.remove(key);
    }

    fn flush(&mut self) {}
}

#[test]
fn eframe_keeps_the_window_place_but_not_egui_memory() {
    let fake = FakePorts::with(&["COM7"]);
    let mut harness = harness(&fake, on("COM7"), "egui-memory");
    assert!(!eframe::App::persist_egui_memory(harness.state()), "app.ron would weigh 18 KB of egui's memory");
    // An app.ron of an earlier version: egui's memory goes, the window's place stays.
    let mut stored = Stored::default();
    stored.0.insert("egui".into(), "(options:(theme_preference:System))".into());
    stored.0.insert("window".into(), "(inner_size_points:Some((1100.0,720.0)))".into());
    eframe::App::save(harness.state_mut(), &mut stored);
    assert_eq!(stored.0.keys().map(String::as_str).collect::<Vec<_>>(), ["window"]);
}

#[test]
fn a_file_name_is_made_of_the_tab_name() {
    assert_eq!(file_stem("/dev/ttyUSB0"), "ttyUSB0");
    assert_eq!(file_stem("COM23"), "COM23");
    assert_eq!(file_stem("Carte Nano / rev D"), "Carte-Nano---rev-D");
    assert_eq!(file_stem(r"\\.\COM23"), "COM23");
    assert_eq!(file_stem("///"), "rsc");
}
