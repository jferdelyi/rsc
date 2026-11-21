//! The licences of what RSC is made of: each component the program carries, the licence it
//! declares, where its source is, and the texts its licence asks to go with every copy. The record
//! is made by `tests/licences.rs` from cargo's own list of each system's dependencies, kept in
//! assets/licences, and carried in the program: it travels with a copied executable.

use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

/// The systems RSC is built for, by the names the record gives them.
pub const SYSTEMS: [&str; 4] = ["windows", "linux-x86", "linux-arm64", "macos-arm64"];

/// What RSC is made of, and the texts of their licences.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Notices {
    pub components: Vec<Component>,
    /// Each text once, however many components it is for.
    pub texts: Vec<Text>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Component {
    pub name: String,
    /// Empty for what has no version of its own, as Rust's standard library.
    pub version: String,
    /// The licence as the component declares it: "MIT OR Apache-2.0".
    pub licence: String,
    /// Where its source is, at this version.
    pub source: String,
    /// Where it is developed, when it says.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repository: Option<String>,
    /// The systems whose program carries it, among `SYSTEMS`.
    pub systems: Vec<String>,
    /// Its licence's texts, by their place in `Notices::texts`.
    pub texts: Vec<usize>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Text {
    /// The file it comes from, "LICENSE-MIT", or the licence's own text with the authors: "MIT".
    pub title: String,
    pub body: String,
}

impl Component {
    /// Its name and version: "egui 0.36.2".
    pub fn label(&self) -> String {
        if self.version.is_empty() { self.name.clone() } else { format!("{} {}", self.name, self.version) }
    }
}

impl Notices {
    /// The texts of a component's licence.
    pub fn texts_of<'a>(&'a self, component: &'a Component) -> impl Iterator<Item = &'a Text> {
        component.texts.iter().filter_map(|&index| self.texts.get(index))
    }

    /// The whole record as a text file: each component, then each licence text with those it is for.
    pub fn text(&self) -> String {
        let mut out = format!(
            "RSC {}: third-party licences\n\n\
             RSC is made with the components below. Their licences ask that these notices go with\n\
             every copy of the program. For each component: its licence as it declares it, where its\n\
             source is, the systems whose program carries it, and its licence texts below.\n\n\
             COMPONENTS ({})\n\n",
            env!("CARGO_PKG_VERSION"),
            self.components.len()
        );
        for component in &self.components {
            out += &format!("{}\n  Licence: {}\n  Source: {}\n", component.label(), component.licence, component.source);
            if let Some(repository) = &component.repository {
                out += &format!("  Repository: {repository}\n");
            }
            let texts: Vec<String> = component.texts.iter().map(|index| (index + 1).to_string()).collect();
            out += &format!("  Systems: {}\n  Texts: {}\n\n", component.systems.join(", "), texts.join(", "));
        }
        out += &format!("LICENCE TEXTS ({})\n", self.texts.len());
        for (index, text) in self.texts.iter().enumerate() {
            let users: Vec<String> = self.components.iter().filter(|component| component.texts.contains(&index)).map(Component::label).collect();
            out += &format!("\n======== {}. {} ========\nFor: {}\n\n{}\n", index + 1, text.title, users.join(", "), text.body);
        }
        out
    }
}

/// Who holds the rights on RSC itself, and since when.
pub const HOLDER: &str = "Jean-François Erdelyi";
pub const YEAR: &str = "2026";

/// Where RSC's source code is: the repository `Cargo.toml` names.
pub const SOURCE: &str = env!("CARGO_PKG_REPOSITORY");

/// RSC's own licence, the Eclipse Public License 2.0 with no Secondary License, as `LICENSE` holds
/// it: the text the packages uhlc 0.8.2 and ringbuffer-spsc 0.1.15 carry, the same.
pub fn own_licence() -> &'static str {
    include_str!("../LICENSE")
}

/// RSC's own notice: who holds its rights, its licence, where its source is.
pub fn own_notice() -> String {
    format!(
        "RSC (Rust Serial Console) {}\n\
         Copyright (c) {YEAR} {HOLDER}\n\n\
         This program and the accompanying materials are made available under the terms of the\n\
         Eclipse Public License 2.0, below, also at https://www.eclipse.org/legal/epl-2.0, with no\n\
         Secondary License. SPDX-License-Identifier: EPL-2.0\n\n\
         The source code of RSC is at {SOURCE}.\n",
        env!("CARGO_PKG_VERSION")
    )
}

/// Every licence that goes with a copy of RSC, as a text file: its own notice and licence, then
/// the notices of what it is made of.
pub fn everything() -> String {
    format!("{}\n======== RSC's licence: Eclipse Public License 2.0 ========\n\n{}\n\n{}", own_notice(), own_licence(), bundled().text())
}

/// The record carried in the program, read the first time it is asked for.
pub fn bundled() -> &'static Notices {
    static NOTICES: OnceLock<Notices> = OnceLock::new();
    // A test reads the same file: it cannot be unreadable here.
    NOTICES.get_or_init(|| serde_json::from_str(include_str!("../assets/licences/licences.json")).expect("the licences carried are read"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn find<'a>(notices: &'a Notices, name: &str) -> &'a Component {
        notices.components.iter().find(|component| component.name == name).unwrap_or_else(|| panic!("{name} is not in the record"))
    }

    fn texts_of(notices: &Notices, name: &str) -> String {
        notices.texts_of(find(notices, name)).map(|text| text.body.as_str()).collect::<Vec<_>>().join("\n")
    }

    #[test]
    fn every_component_carried_has_its_licence_texts_and_its_systems() {
        let notices = bundled();
        assert!(notices.components.len() > 100, "{} components", notices.components.len());
        for component in &notices.components {
            assert!(!component.texts.is_empty(), "{}: no text", component.label());
            assert!(component.texts.iter().all(|&index| index < notices.texts.len()), "{}: a text past the end", component.label());
            assert!(!component.systems.is_empty() && component.systems.iter().all(|system| SYSTEMS.contains(&system.as_str())), "{}: {:?}", component.label(), component.systems);
            assert!(component.source.starts_with("https://"), "{}: {}", component.label(), component.source);
        }
        for system in SYSTEMS {
            assert!(notices.components.iter().any(|component| component.systems.iter().any(|known| known == system)), "nothing for {system}");
        }
    }

    #[test]
    fn what_asks_for_something_is_in_the_record() {
        let notices = bundled();
        // MPL-2.0: where its source is.
        let serialport = find(notices, "serialport");
        assert_eq!(serialport.licence, "MPL-2.0");
        assert!(serialport.source.starts_with("https://crates.io/crates/serialport/"));
        // Its package brings the short notice only: the licence's whole text goes with it.
        let mpl = texts_of(notices, "serialport");
        assert!(mpl.contains("This Source Code Form is subject to the terms of the Mozilla Public License") && mpl.contains("Mozilla Public License Version 2.0"));
        // The fonts of the window: OFL and Ubuntu Font.
        let fonts = texts_of(notices, "epaint_default_fonts");
        assert!(fonts.contains("SIL OPEN FONT LICENSE") && fonts.contains("UBUNTU FONT LICENCE"), "the fonts' licences");
        // The icons, and Rust's standard library, carried in every program.
        assert!(texts_of(notices, "Lucide").contains("ISC License"));
        assert!(texts_of(notices, "Rust standard library").contains("Permission is hereby granted"));
        // A package that brings no file of its own: the licence's text, with its authors.
        assert!(texts_of(notices, "egui").starts_with("Copyright (c) Emil Ernerfeldt\n"));
    }

    #[test]
    fn rsc_s_own_licence_is_the_eclipse_public_license_2_0_and_cargo_says_so() {
        let licence = own_licence();
        assert!(licence.starts_with("Eclipse Public License - v 2.0\n"), "{}", &licence[..60]);
        assert!(licence.contains("Exhibit A - Form of Secondary Licenses Notice"), "the whole text, its Exhibit A included");
        assert!(!licence.contains("pur_pose"), "not the text with misprints some packages carry");
        assert!(include_str!("../Cargo.toml").contains("\nlicense = \"EPL-2.0\"\n"), "Cargo.toml says the same licence");
        let notice = own_notice();
        assert!(notice.contains(&format!("Copyright (c) {YEAR} {HOLDER}")) && notice.contains("SPDX-License-Identifier: EPL-2.0"), "{notice}");
        assert!(notice.contains("The source code of RSC is at https://github.com/jferdelyi/rsc.\n"), "where its source is: {notice}");
    }

    #[test]
    fn rsc_s_source_is_the_repository_cargo_names() {
        assert!(include_str!("../Cargo.toml").contains(&format!("\nrepository = \"{SOURCE}\"\n")), "Cargo.toml names {SOURCE}");
        assert!(SOURCE.starts_with("https://github.com/"), "a public repository: {SOURCE}");
    }

    #[test]
    fn the_text_to_save_gives_rsc_s_licence_first_then_the_components() {
        let text = everything();
        assert!(text.starts_with(&own_notice()), "RSC's own notice first");
        let licence_at = text.find(own_licence()).expect("RSC's licence, whole");
        let components_at = text.find(&bundled().text()).expect("the components' notices, unchanged");
        assert!(licence_at < components_at, "RSC's licence before its components'");
    }

    #[test]
    fn the_text_file_names_every_component_and_gives_every_text() {
        let notices = bundled();
        let text = notices.text();
        for component in &notices.components {
            assert!(text.contains(&format!("{}\n  Licence: {}\n", component.label(), component.licence)), "{}", component.label());
        }
        for (index, licence) in notices.texts.iter().enumerate() {
            assert!(text.contains(&format!("======== {}. {} ========", index + 1, licence.title)));
            assert!(text.contains(&licence.body));
        }
    }
}
