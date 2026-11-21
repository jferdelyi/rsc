//! RSC's record of licences, made from cargo's own list of what each system's program carries:
//!
//!   RSC_LICENCES=write cargo test --test licences write_the_record
//!
//! writes assets/licences, under WSL, where cargo's cache holds the packages of the four systems.
//! With the rest of the suite, a check that the record the program carries is that of this
//! system's dependencies today: a package updated without the record made again fails it. The
//! check is shown to fail on records spoilt on purpose.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;
use rsc::licences::{self, Component, Notices, Text};

/// Each system: its target, and its name in the record.
const TARGETS: [(&str, &str); 4] = [
    ("x86_64-pc-windows-msvc", "windows"),
    ("x86_64-unknown-linux-gnu", "linux-x86"),
    ("aarch64-unknown-linux-gnu", "linux-arm64"),
    ("aarch64-apple-darwin", "macos-arm64"),
];

/// The licence files of a package, by the start of their name: "LICENSE-MIT", "COPYING".
const LICENCE_FILES: [&str; 6] = ["license", "licence", "copying", "copyright", "unlicense", "notice"];

/// Licence files a package brings elsewhere than at its root: the fonts of egui's window.
const MORE_FILES: [(&str, &[&str]); 1] = [("epaint_default_fonts", &["fonts/OFL.txt", "fonts/UFL.txt", "fonts/Hack-Regular.txt", "fonts/emoji-icon-font-mit-license.txt"])];

/// The licences whose own text goes with a package that brings none: the first the package allows.
const OWN_TEXTS: [(&str, &str); 3] = [("MIT", "MIT.txt"), ("Apache-2.0", "Apache-2.0.txt"), ("BSL-1.0", "BSL-1.0.txt")];

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// A component as cargo's list gives it: its texts themselves rather than their place.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Found {
    name: String,
    version: String,
    licence: String,
    source: String,
    repository: Option<String>,
    texts: Vec<Text>,
}

/// The target this test runs on, and its name in the record.
fn this_system() -> (&'static str, &'static str) {
    let target = if cfg!(windows) {
        "x86_64-pc-windows-msvc"
    } else if cfg!(target_os = "macos") {
        "aarch64-apple-darwin"
    } else if cfg!(target_arch = "aarch64") {
        "aarch64-unknown-linux-gnu"
    } else {
        "x86_64-unknown-linux-gnu"
    };
    TARGETS.into_iter().find(|(known, _)| *known == target).expect("a target of the record")
}

/// A text file, its line ends and trailing spaces made even.
fn read_text(path: &Path) -> Result<String, String> {
    let bytes = fs::read(path).map_err(|error| format!("{}: {error}", path.display()))?;
    let text = String::from_utf8_lossy(&bytes).replace("\r\n", "\n");
    Ok(text.lines().map(str::trim_end).collect::<Vec<_>>().join("\n").trim_matches('\n').to_owned())
}

/// The licence files at a package's root, in the order of their names; a folder of them, its files.
fn licence_files(folder: &Path) -> Result<Vec<Text>, String> {
    let mut entries: Vec<PathBuf> = fs::read_dir(folder).map_err(|error| format!("{}: {error}", folder.display()))?.filter_map(|entry| entry.ok().map(|entry| entry.path())).collect();
    entries.sort();
    let mut texts = Vec::new();
    for entry in entries {
        let name = entry.file_name().map(|name| name.to_string_lossy().into_owned()).unwrap_or_default();
        if !LICENCE_FILES.iter().any(|start| name.to_lowercase().starts_with(start)) {
            continue;
        }
        if entry.is_dir() {
            let mut inside: Vec<PathBuf> = fs::read_dir(&entry).map_err(|error| error.to_string())?.filter_map(|file| file.ok().map(|file| file.path())).filter(|file| file.is_file()).collect();
            inside.sort();
            for file in inside {
                let title = format!("{name}/{}", file.file_name().map(|file| file.to_string_lossy().into_owned()).unwrap_or_default());
                texts.push(Text { title, body: read_text(&file)? });
            }
        } else {
            texts.push(Text { title: name, body: read_text(&entry)? });
        }
    }
    Ok(texts)
}

/// The text of a licence the package allows, after "Copyright (c)" and its authors.
fn own_text(licence: &str, authors: &[String]) -> Result<Text, String> {
    let words: Vec<&str> = licence.split(|c: char| !(c.is_alphanumeric() || c == '-' || c == '.')).collect();
    let (spdx, file) = OWN_TEXTS.into_iter().find(|(spdx, _)| words.contains(spdx)).ok_or_else(|| format!("no text of its own for \"{licence}\""))?;
    let body = read_text(&root().join("assets").join("licences").join("textes").join(file))?;
    let who = if authors.is_empty() { "its authors".to_owned() } else { authors.join(", ") };
    Ok(Text { title: format!("{spdx}, with the authors the package declares"), body: format!("Copyright (c) {who}\n\n{body}") })
}

/// A package of cargo's list, with its licence texts: those it brings, the licence's own text when
/// it brings none, and the whole MPL when it brings only its short notice.
fn found(package: &Value) -> Result<Found, String> {
    let field = |key: &str| package[key].as_str().map(str::to_owned);
    let name = field("name").ok_or("a package without a name")?;
    let version = field("version").unwrap_or_default();
    let licence = field("license").ok_or_else(|| format!("{name} {version}: no licence declared"))?;
    let manifest = field("manifest_path").ok_or_else(|| format!("{name}: no manifest"))?;
    let folder = Path::new(&manifest).parent().ok_or_else(|| format!("{name}: no folder"))?.to_owned();
    let mut texts = licence_files(&folder)?;
    if let Some(file) = field("license_file") {
        let text = Text { title: file.clone(), body: read_text(&folder.join(&file))? };
        if !texts.iter().any(|known| known.body == text.body) {
            texts.push(text);
        }
    }
    if texts.is_empty() {
        // An author as "Emil Ernerfeldt <emil.ernerfeldt@gmail.com>": the name.
        let authors: Vec<String> = package["authors"].as_array().into_iter().flatten().filter_map(Value::as_str).map(|author| author.split(" <").next().unwrap_or(author).trim().to_owned()).collect();
        texts.push(own_text(&licence, &authors)?);
    }
    for (known, files) in MORE_FILES {
        if known == name {
            for file in files {
                texts.push(Text { title: (*file).to_owned(), body: read_text(&folder.join(file))? });
            }
        }
    }
    if licence.contains("MPL-2.0") && !texts.iter().any(|text| text.body.contains("Mozilla Public License Version 2.0")) {
        texts.push(Text { title: "MPL-2.0".to_owned(), body: read_text(&root().join("assets").join("licences").join("textes").join("MPL-2.0.txt"))? });
    }
    let repository = field("repository");
    Ok(Found { source: format!("https://crates.io/crates/{name}/{version}"), name, version, licence, repository, texts })
}

/// What cargo says, run on RSC for a target.
fn cargo(target: &str, command: &[&str]) -> Result<String, String> {
    let output = Command::new(option_env!("CARGO").unwrap_or("cargo"))
        .args(command)
        .args(["--locked", "--offline", "--manifest-path"])
        .arg(root().join("Cargo.toml"))
        .output()
        .map_err(|error| format!("cargo {}: {error}", command[0]))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    } else {
        Err(format!("cargo {} for {target}: {}", command[0], String::from_utf8_lossy(&output.stderr)))
    }
}

/// The packages a target's program is built with, by name and version: `cargo tree` along the
/// dependencies of the program only, not those that build it or test it, nor the macros of the
/// compiler and what they need. Not `cargo metadata`, which counts what a feature asked by a test's
/// dependency brings: wgpu and 40 other packages, through egui_kittest, that rsc.exe does not hold.
fn carried(target: &str) -> Result<BTreeSet<(String, String)>, String> {
    let tree = cargo(target, &["tree", "--target", target, "--edges", "normal,no-proc-macro", "--prefix", "none", "--format", "{p}"])?;
    // "egui v0.36.2", "num-traits v0.2.19 (*)" for one already shown, RSC itself first.
    Ok(tree
        .lines()
        .skip(1)
        .filter_map(|line| {
            let mut words = line.split_whitespace();
            Some((words.next()?.to_owned(), words.next()?.trim_start_matches('v').to_owned()))
        })
        .collect())
}

/// What the program carries for a target: the packages RSC is built with there, each with what
/// cargo knows of it, then the icons' font and Rust's standard library.
fn made_for(target: &str) -> Result<Vec<Found>, String> {
    let names = carried(target)?;
    let metadata: Value = serde_json::from_str(&cargo(target, &["metadata", "--format-version", "1", "--filter-platform", target])?).map_err(|error| error.to_string())?;
    let packages: BTreeMap<(String, String), &Value> = metadata["packages"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|package| ((package["name"].as_str().unwrap_or_default().to_owned(), package["version"].as_str().unwrap_or_default().to_owned()), package))
        .collect();
    let mut made = names.iter().map(|key| packages.get(key).ok_or_else(|| format!("{} {}: not in cargo's list", key.0, key.1)).and_then(|package| found(package))).collect::<Result<Vec<_>, _>>()?;
    made.push(Found {
        name: "Lucide".to_owned(),
        version: "1.74.0".to_owned(),
        licence: "ISC AND MIT".to_owned(),
        source: "https://www.npmjs.com/package/lucide-static/v/1.74.0".to_owned(),
        repository: Some("https://github.com/lucide-icons/lucide".to_owned()),
        texts: vec![Text { title: "LICENSE".to_owned(), body: read_text(&root().join("assets").join("fonts").join("LICENSE-lucide.txt"))? }],
    });
    made.push(Found {
        name: "Rust standard library".to_owned(),
        version: String::new(),
        licence: "MIT OR Apache-2.0".to_owned(),
        source: "https://github.com/rust-lang/rust".to_owned(),
        repository: None,
        texts: vec![own_text("MIT OR Apache-2.0", &["The Rust Project Contributors".to_owned()])?],
    });
    Ok(made)
}

/// The record of what each system carries: a component once, with the systems that carry it, and
/// each text once.
fn record(per_system: &[(&str, Vec<Found>)]) -> Notices {
    let mut merged: BTreeMap<(String, String, String), (Found, Vec<String>)> = BTreeMap::new();
    for (system, made) in per_system {
        for item in made {
            let key = (item.name.to_lowercase(), item.name.clone(), item.version.clone());
            merged.entry(key).or_insert_with(|| (item.clone(), Vec::new())).1.push((*system).to_owned());
        }
    }
    let mut notices = Notices::default();
    for (item, systems) in merged.into_values() {
        let texts = item
            .texts
            .iter()
            .map(|text| match notices.texts.iter().position(|known| known.body == text.body) {
                Some(index) => index,
                None => {
                    notices.texts.push(text.clone());
                    notices.texts.len() - 1
                }
            })
            .collect();
        notices.components.push(Component { name: item.name, version: item.version, licence: item.licence, source: item.source, repository: item.repository, systems, texts });
    }
    notices
}

/// What differs between a record and what cargo's list gives for a system.
fn differences(carried: &Notices, system: &str, made: &[Found]) -> Vec<String> {
    let in_record: BTreeMap<(String, String), Found> = carried
        .components
        .iter()
        .filter(|component| component.systems.iter().any(|known| known == system))
        .map(|component| {
            let texts = carried.texts_of(component).map(|text| Text { title: String::new(), body: text.body.clone() }).collect();
            let found = Found { name: component.name.clone(), version: component.version.clone(), licence: component.licence.clone(), source: component.source.clone(), repository: component.repository.clone(), texts };
            ((component.name.clone(), component.version.clone()), found)
        })
        .collect();
    let mut problems = Vec::new();
    for item in made {
        let key = (item.name.clone(), item.version.clone());
        // The titles of a text shared by several components are those of the first: the bodies count.
        let item = Found { texts: item.texts.iter().map(|text| Text { title: String::new(), body: text.body.clone() }).collect(), ..item.clone() };
        match in_record.get(&key) {
            None => problems.push(format!("{} {}: carried by the program, missing from the record", key.0, key.1)),
            Some(known) if *known != item => problems.push(format!("{} {}: not as in the record", key.0, key.1)),
            Some(_) => {}
        }
    }
    let made: BTreeSet<(&str, &str)> = made.iter().map(|item| (item.name.as_str(), item.version.as_str())).collect();
    for (name, version) in in_record.keys() {
        if !made.contains(&(name.as_str(), version.as_str())) {
            problems.push(format!("{name} {version}: in the record, no longer carried"));
        }
    }
    problems
}

#[test]
fn the_record_carried_is_that_of_this_systems_dependencies() {
    let (target, system) = this_system();
    let made = made_for(target).unwrap_or_else(|error| panic!("{error}"));
    let problems = differences(licences::bundled(), system, &made);
    assert!(problems.is_empty(), "{}\nMake the record again, under WSL: RSC_LICENCES=write cargo test --test licences write_the_record", problems.join("\n"));
    let file = read_text(&root().join("assets").join("licences").join("THIRD-PARTY-LICENSES.txt")).unwrap_or_else(|error| panic!("{error}"));
    assert!(file == licences::bundled().text().trim_matches('\n'), "THIRD-PARTY-LICENSES.txt is not the record's text: make the record again");
}

#[test]
fn a_record_spoilt_on_purpose_is_found_out() {
    let (target, system) = this_system();
    let made = made_for(target).unwrap_or_else(|error| panic!("{error}"));
    let good = record(&[(system, made.clone())]);
    assert_eq!(differences(&good, system, &made), Vec::<String>::new());
    // A component gone.
    let mut gone = good.clone();
    let serde = gone.components.iter().position(|component| component.name == "serde").expect("serde");
    let version = gone.components.remove(serde).version;
    assert_eq!(differences(&gone, system, &made), [format!("serde {version}: carried by the program, missing from the record")]);
    // A version the program no longer carries.
    let mut older = good.clone();
    let serialport = older.components.iter_mut().find(|component| component.name == "serialport").expect("serialport");
    let version = std::mem::replace(&mut serialport.version, "4.9.0".to_owned());
    assert_eq!(
        differences(&older, system, &made),
        [format!("serialport {version}: carried by the program, missing from the record"), "serialport 4.9.0: in the record, no longer carried".to_owned()]
    );
    // A licence text that is not the package's.
    let mut changed = good.clone();
    let text = changed.components.iter().find(|component| component.name == "serialport").expect("serialport").texts[0];
    changed.texts[text].body.push_str("\nchanged");
    assert_eq!(differences(&changed, system, &made), [format!("serialport {version}: not as in the record")]);
}

/// Writes the record, when asked: RSC_LICENCES=write.
#[test]
fn write_the_record() {
    if std::env::var("RSC_LICENCES").as_deref() != Ok("write") {
        return;
    }
    let per_system: Vec<(&str, Vec<Found>)> = TARGETS.iter().map(|(target, system)| (*system, made_for(target).unwrap_or_else(|error| panic!("{error}")))).collect();
    let notices = record(&per_system);
    let folder = root().join("assets").join("licences");
    fs::write(folder.join("licences.json"), serde_json::to_string_pretty(&notices).expect("the record as JSON") + "\n").expect("licences.json written");
    fs::write(folder.join("THIRD-PARTY-LICENSES.txt"), notices.text()).expect("THIRD-PARTY-LICENSES.txt written");
    for (system, made) in &per_system {
        println!("{system}: {} components", made.len());
    }
    println!("record: {} components, {} texts", notices.components.len(), notices.texts.len());
}
