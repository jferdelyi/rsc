//! The window as a whole: its English, and a glyph for every character it shows.

use super::*;

#[test]
fn the_window_speaks_english_when_asked() {
    let fake = FakePorts::with(&["COM7"]);
    let mut settings = on("COM7");
    settings.language = Language::English;
    let mut harness = harness(&fake, settings, "english");
    harness.run();
    harness.get_by_label("Connect");
    harness.get_by_label("Send");
    i18n::set_english(false);
}

/// Read in the fonts' own tables: egui's `has_glyph` says no for every character of a font that
/// also holds the replacement character.
#[test]
fn every_character_of_the_window_has_its_glyph() {
    use ab_glyph::Font;
    /// The Rust files under `folder`, at any depth.
    fn sources(folder: &std::path::Path, found: &mut Vec<std::path::PathBuf>) {
        for entry in std::fs::read_dir(folder).unwrap_or_else(|error| panic!("{}: {error}", folder.display())) {
            let path = entry.expect("an entry").path();
            if path.is_dir() {
                sources(&path, found);
            } else if path.extension().is_some_and(|extension| extension == "rs") {
                found.push(path);
            }
        }
    }
    // Every character beyond ASCII written in the sources, French texts included: the window's, its
    // tests', and the other files whose texts it shows (the ports, what a profile changes, the
    // sending, the licences), read as they are, so that no file is forgotten (the curves' were, until
    // 19/11/2026). Not the terminal's (`terminal/`): its set of lines and corners has its own test, in
    // the console's font, ten of them drawn as another character.
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    sources(&root, &mut files);
    let text: std::collections::BTreeSet<char> = files
        .iter()
        .filter(|path| !path.starts_with(root.join("terminal")))
        .flat_map(|path| std::fs::read_to_string(path).unwrap_or_else(|error| panic!("{}: {error}", path.display())).chars().collect::<Vec<char>>())
        .filter(|c| !c.is_ascii())
        .collect();
    assert!(text.contains(&'é') && text.contains(&'«'), "the French texts are read");
    let fonts = egui::FontDefinitions::default();
    let faces = |family: egui::FontFamily| -> Vec<ab_glyph::FontRef<'_>> {
        fonts.families[&family].iter().map(|name| ab_glyph::FontRef::try_from_slice_and_index(&fonts.font_data[name].font, fonts.font_data[name].index).expect(name)).collect()
    };
    let has = |face: &ab_glyph::FontRef<'_>, c: char| face.glyph_id(c).0 != 0;
    // The window's text: any font of its family.
    let proportional = faces(egui::FontFamily::Proportional);
    let missing: String = text.iter().filter(|&&c| !proportional.iter().any(|face| has(face, c))).collect();
    assert!(missing.is_empty(), "the window's text lacks {missing}");
    // The console's: its first font, so that every character is as wide as the others.
    let monospace = faces(egui::FontFamily::Monospace);
    let missing: String = text.iter().filter(|&&c| !has(&monospace[0], c)).collect();
    assert!(missing.is_empty(), "the console lacks {missing}");
    // The check fails where it must: a kanji is in none of them.
    assert!(!has(&monospace[0], '\u{6817}') && !proportional.iter().take(1).any(|face| has(face, '\u{6817}')));
}
