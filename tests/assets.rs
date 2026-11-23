//! RSC's icon on each system, checked in its form: rsc.ico for Windows (build.rs puts it in rsc.exe),
//! the PNGs and the .desktop file for Linux (build.sh --install), rsc.icns for macOS (RSC.app), and the
//! margin a macOS icon keeps around its tile. Each check is shown to fail on a file spoilt on purpose.

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

/// The images of an .icns file, by their type.
fn icns_images(icns: &[u8]) -> Result<BTreeMap<String, Vec<u8>>, String> {
    let length = |at: usize| u32::from_be_bytes(icns[at..at + 4].try_into().unwrap()) as usize;
    if icns.len() < 8 || &icns[..4] != b"icns" || length(4) != icns.len() {
        return Err("not an icns, or its length is wrong".into());
    }
    let mut images = BTreeMap::new();
    let mut at = 8;
    while at < icns.len() {
        let kind = String::from_utf8_lossy(&icns[at..at + 4]).into_owned();
        let size = length(at + 4);
        let data = icns.get(at + 8..at + size).ok_or_else(|| format!("{kind}: past the end"))?;
        images.insert(kind, data.to_vec());
        at += size;
    }
    Ok(images)
}

/// The entries of an .icns file, by their type, each the size of its PNG.
fn icns_entries(icns: &[u8]) -> Result<BTreeMap<String, u32>, String> {
    icns_images(icns)?.into_iter().map(|(kind, png)| Ok((kind, png_size(&png)?.0))).collect()
}

/// An .icns file holding these PNG images, by their type.
#[cfg(target_os = "macos")]
fn icns_of(images: &BTreeMap<String, Vec<u8>>) -> Vec<u8> {
    let entries: Vec<u8> = images.iter().flat_map(|(kind, png)| [kind.as_bytes(), &(8 + png.len() as u32).to_be_bytes(), png].concat()).collect();
    [b"icns".as_slice(), &(8 + entries.len() as u32).to_be_bytes(), &entries].concat()
}

/// The images Apple's iconutil takes out of an .icns file, by their name (`icon_48x48.png`), each the
/// width of its PNG. iconutil gives each image the size its type has for Apple.
#[cfg(target_os = "macos")]
fn iconutil_widths(icns: &[u8], name: &str) -> BTreeMap<String, u32> {
    let folder = std::env::temp_dir().join("rsc-tests").join("iconutil");
    std::fs::create_dir_all(&folder).expect("a folder for iconutil");
    let (file, iconset) = (folder.join(format!("{name}.icns")), folder.join(format!("{name}.iconset")));
    std::fs::write(&file, icns).expect("the .icns written");
    let _ = std::fs::remove_dir_all(&iconset);
    let status = std::process::Command::new("iconutil").arg("-c").arg("iconset").arg(&file).arg("-o").arg(&iconset).status().expect("iconutil runs");
    assert!(status.success(), "iconutil reads {name}.icns");
    let images = std::fs::read_dir(&iconset).expect("iconutil's images").map(|entry| {
        let path = entry.expect("an image").path();
        (path.file_name().unwrap().to_string_lossy().into_owned(), png_size(&std::fs::read(&path).expect("an image read")).expect("a PNG").0)
    });
    images.collect()
}

/// The width of an icon's tile across its middle row (the pixels more opaque than not, so not a soft
/// shadow), and the icon's own width.
fn tile_width(png: &[u8]) -> Result<(u32, u32), String> {
    let icon = eframe::icon_data::from_png_bytes(png).map_err(|error| error.to_string())?;
    let row = (icon.height / 2 * icon.width * 4) as usize;
    let opaque: Vec<u32> = (0..icon.width).filter(|x| icon.rgba[row + *x as usize * 4 + 3] > 128).collect();
    match (opaque.first(), opaque.last()) {
        (Some(first), Some(last)) => Ok((last - first + 1, icon.width)),
        _ => Err("no tile on the middle row".into()),
    }
}

/// The width of a tile on Apple's grid, 824 pixels in a square of 1024, as Calendar's or Calculator's.
fn apple_tile(side: u32) -> u32 {
    (side * 824 + 512) / 1024
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
    let expected = [
        ("ic07", 128),
        ("ic08", 256),
        ("ic09", 512),
        ("ic10", 1024),
        ("ic11", 32),
        ("ic12", 64),
        ("ic13", 256),
        ("ic14", 512),
        ("icp4", 16),
        ("icp5", 32),
        ("icp6", 48),
    ];
    assert_eq!(entries, expected.iter().map(|(kind, size)| ((*kind).to_owned(), *size)).collect());
    let mut spoilt = asset("rsc.icns");
    spoilt.truncate(spoilt.len() - 10);
    assert!(icns_entries(&spoilt).is_err(), "a file cut short is seen");
}

#[test]
#[cfg(target_os = "macos")]
fn apple_s_iconutil_finds_each_image_of_the_macos_icon_at_its_size() {
    let names = [
        ("icp4", "icon_16x16.png"),
        ("icp5", "icon_32x32.png"),
        ("ic11", "icon_16x16@2x.png"),
        ("icp6", "icon_48x48.png"),
        ("ic12", "icon_32x32@2x.png"),
        ("ic07", "icon_128x128.png"),
        ("ic08", "icon_256x256.png"),
        ("ic13", "icon_128x128@2x.png"),
        ("ic09", "icon_512x512.png"),
        ("ic14", "icon_256x256@2x.png"),
        ("ic10", "icon_512x512@2x.png"),
    ];
    let ours = icns_entries(&asset("rsc.icns")).expect("rsc.icns reads");
    let apple = iconutil_widths(&asset("rsc.icns"), "rsc");
    assert_eq!(apple.len(), names.len(), "{apple:?}");
    for (kind, name) in names {
        assert_eq!(apple.get(name), ours.get(kind), "{kind}, which iconutil names {name}");
    }
    // A 64-pixel image in icp6 comes out at 48: an image at the wrong size is seen.
    let mut images = icns_images(&asset("rsc.icns")).expect("rsc.icns reads");
    let bigger = images["ic12"].clone();
    images.insert("icp6".into(), bigger);
    assert_eq!(iconutil_widths(&icns_of(&images), "spoilt").get("icon_48x48.png"), Some(&48), "brought down to 48");
    assert_eq!(icns_entries(&icns_of(&images)).expect("the spoilt icns reads").get("icp6"), Some(&64));
}

#[test]
fn the_macos_icon_leaves_apple_s_margin_around_its_tile() {
    for (kind, png) in icns_images(&asset("rsc.icns")).expect("rsc.icns reads") {
        let (tile, side) = tile_width(&png).unwrap_or_else(|error| panic!("{kind}: {error}"));
        assert!(tile.abs_diff(apple_tile(side)) <= 1, "{kind}: a tile of {tile} pixels in {side}, {} on Apple's grid", apple_tile(side));
    }
    // The window's icon under macOS, which the Dock shows while RSC runs.
    let (tile, side) = tile_width(&asset("rsc-macos-512.png")).expect("rsc-macos-512.png reads");
    assert_eq!((tile.abs_diff(apple_tile(side)) <= 1, side), (true, 512), "a tile of {tile} pixels in {side}");
    // The icon of Windows and Linux keeps its whole tile, which is refused here.
    let (tile, side) = tile_width(&asset("linux/rsc-256.png")).expect("linux/rsc-256.png reads");
    assert_eq!((tile, side), (240, 256), "the icon of Windows and Linux fills its square");
    assert!(tile.abs_diff(apple_tile(side)) > 1, "a tile without Apple's margin is seen");
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
