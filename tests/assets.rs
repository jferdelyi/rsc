//! RSC's icon on each system, checked in its form: rsc.ico for Windows (build.rs puts it in rsc.exe),
//! the PNGs and the .desktop file for Linux (build.sh --install), rsc.icns for macOS (RSC.app). Each
//! check is shown to fail on a file spoilt on purpose.

use std::collections::BTreeMap;

const PNG: &[u8] = b"\x89PNG\r\n\x1a\n";
const SIZES: [u32; 7] = [16, 24, 32, 48, 64, 128, 256];

fn asset(name: &str) -> Vec<u8> {
    std::fs::read(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets").join(name)).unwrap_or_else(|error| panic!("assets/{name}: {error}"))
}

/// The width and height a PNG says it has.
fn png_size(png: &[u8]) -> Result<(u32, u32), String> {
    if !png.starts_with(PNG) || png.len() < 24 || &png[12..16] != b"IHDR" {
        return Err("not a PNG".into());
    }
    let number = |at: usize| u32::from_be_bytes([png[at], png[at + 1], png[at + 2], png[at + 3]]);
    Ok((number(16), number(20)))
}

/// The images of an .ico file by their size, each a PNG of that size.
fn ico_sizes(ico: &[u8]) -> Result<Vec<u32>, String> {
    let word = |at: usize| u16::from_le_bytes([ico[at], ico[at + 1]]);
    if ico.len() < 6 || word(0) != 0 || word(2) != 1 {
        return Err("not an icon".into());
    }
    let mut sizes = Vec::new();
    for index in 0..usize::from(word(4)) {
        let entry = 6 + 16 * index;
        let size = u32::from_le_bytes(ico[entry + 8..entry + 12].try_into().unwrap()) as usize;
        let offset = u32::from_le_bytes(ico[entry + 12..entry + 16].try_into().unwrap()) as usize;
        let image = ico.get(offset..offset + size).ok_or("an image past the end")?;
        let (width, height) = png_size(image)?;
        let stated = if ico[entry] == 0 { 256 } else { u32::from(ico[entry]) };
        if (width, height) != (stated, stated) {
            return Err(format!("image {index}: {width}x{height}, stated {stated}"));
        }
        sizes.push(stated);
    }
    Ok(sizes)
}

/// The entries of an .icns file, by their type, each the size of its PNG.
fn icns_entries(icns: &[u8]) -> Result<BTreeMap<String, u32>, String> {
    let length = |at: usize| u32::from_be_bytes(icns[at..at + 4].try_into().unwrap()) as usize;
    if icns.len() < 8 || &icns[..4] != b"icns" || length(4) != icns.len() {
        return Err("not an icns, or its length is wrong".into());
    }
    let mut entries = BTreeMap::new();
    let mut at = 8;
    while at < icns.len() {
        let kind = String::from_utf8_lossy(&icns[at..at + 4]).into_owned();
        let size = length(at + 4);
        let data = icns.get(at + 8..at + size).ok_or_else(|| format!("{kind}: past the end"))?;
        entries.insert(kind, png_size(data)?.0);
        at += size;
    }
    Ok(entries)
}

/// The keys of a .desktop file's [Desktop Entry], once each.
fn desktop_keys(text: &str) -> Result<BTreeMap<String, String>, String> {
    let mut lines = text.lines();
    if lines.next() != Some("[Desktop Entry]") {
        return Err("no [Desktop Entry] first".into());
    }
    let mut keys = BTreeMap::new();
    for line in lines.filter(|line| !line.is_empty() && !line.starts_with('#')) {
        let (key, value) = line.split_once('=').ok_or_else(|| format!("not a key: {line}"))?;
        if keys.insert(key.to_owned(), value.to_owned()).is_some() {
            return Err(format!("{key} twice"));
        }
    }
    for key in ["Type", "Name", "Exec", "Icon", "StartupWMClass", "Categories"] {
        if !keys.contains_key(key) {
            return Err(format!("no {key}"));
        }
    }
    Ok(keys)
}

#[test]
fn the_windows_icon_holds_every_size() {
    assert_eq!(ico_sizes(&asset("rsc.ico")), Ok(SIZES.to_vec()));
    let mut spoilt = asset("rsc.ico");
    spoilt[6] = 20;
    assert!(ico_sizes(&spoilt).is_err(), "a size that is not the image's is seen");
}

#[test]
fn the_linux_icons_are_pngs_of_their_size() {
    for size in SIZES {
        assert_eq!(png_size(&asset(&format!("linux/rsc-{size}.png"))), Ok((size, size)), "rsc-{size}.png");
    }
    assert!(png_size(b"GIF89a, not a PNG at all").is_err());
}

#[test]
fn the_macos_icon_holds_the_sizes_of_its_types() {
    let entries = icns_entries(&asset("rsc.icns")).expect("rsc.icns reads");
    let expected = [("ic07", 128), ("ic08", 256), ("ic11", 32), ("ic12", 64), ("ic13", 256), ("icp4", 16), ("icp5", 32), ("icp6", 64)];
    assert_eq!(entries, expected.iter().map(|(kind, size)| ((*kind).to_owned(), *size)).collect());
    let mut spoilt = asset("rsc.icns");
    spoilt.truncate(spoilt.len() - 10);
    assert!(icns_entries(&spoilt).is_err(), "a file cut short is seen");
}

#[test]
fn the_desktop_file_names_the_icon_and_the_window() {
    let keys = desktop_keys(&String::from_utf8(asset("rsc.desktop")).unwrap()).expect("rsc.desktop reads");
    assert_eq!((keys["Type"].as_str(), keys["Name"].as_str(), keys["Icon"].as_str()), ("Application", "RSC", "rsc"));
    // The window's app id (ui::APP_ID): the desktop finds its icon by it.
    assert_eq!(keys["StartupWMClass"], rsc::ui::APP_ID);
    assert!(keys["Categories"].ends_with(';'));
    assert!(desktop_keys("[Desktop Entry]\nType=Application\nName=RSC\nExec=rsc\nStartupWMClass=rsc\nCategories=Development;\n").is_err(), "no Icon is seen");
}
