//! The scripts that are run straight (`./build.sh`) keep the right to be run. A script edited from
//! Windows, on `\\wsl.localhost`, loses it, and then answers "Permission denied": it happened to
//! `build.sh` and to `tests/terminal/record.sh`. A script is a file named `*.sh`, or whose first
//! line begins with `#!/`; its owner must be able to run it. Linux and macOS only: Windows has no
//! such right. The shell each script is written for reads it without running it (`bash -n`,
//! `sh -n`): a script written on one system and run on the other shows its syntax error here.

#![cfg(unix)]

use std::fs;
use std::io::Read;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

/// The folders of the tool that hold none of its scripts: what is built, what is measured.
const LEFT_OUT: [&str; 2] = ["build", "etat"];

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Whether a file is run straight: named `*.sh`, or beginning with `#!/`. Rust's `#![...]` is not.
fn is_script(path: &Path) -> bool {
    if path.extension().is_some_and(|extension| extension == "sh") {
        return true;
    }
    let mut start = [0_u8; 3];
    fs::File::open(path).and_then(|mut file| file.read_exact(&mut start)).is_ok() && start == *b"#!/"
}

/// The scripts under a folder, at any depth, hidden folders and links left out; at the top, the
/// folders of `LEFT_OUT` too.
fn scripts(folder: &Path, top: bool, found: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(folder).unwrap_or_else(|error| panic!("{}: {error}", folder.display())) {
        let entry = entry.expect("an entry of the folder");
        let kind = entry.file_type().expect("the kind of the entry");
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if kind.is_dir() {
            if !name.starts_with('.') && !(top && LEFT_OUT.contains(&name.as_ref())) {
                scripts(&entry.path(), false, found);
            }
        } else if kind.is_file() && is_script(&entry.path()) {
            found.push(entry.path());
        }
    }
}

/// The scripts of the tool, by their path from its folder.
fn the_scripts() -> Vec<(PathBuf, String)> {
    let root = root();
    let mut found = Vec::new();
    scripts(&root, true, &mut found);
    found.into_iter().map(|path| {
        let name = path.strip_prefix(&root).unwrap_or(&path).to_string_lossy().into_owned();
        (path, name)
    }).collect()
}

#[test]
fn every_script_can_be_run_by_its_owner() {
    let denied: Vec<String> = the_scripts()
        .into_iter()
        .filter(|(path, _)| fs::metadata(path).expect("the metadata of the script").permissions().mode() & 0o100 == 0)
        .map(|(_, name)| format!("chmod +x {name}"))
        .collect();
    assert!(denied.is_empty(), "scripts that cannot be run (one edited from Windows loses the right); to mend them:\n{}", denied.join("\n"));
}

/// A walk that found nothing would pass the test above for nothing.
#[test]
fn the_walk_finds_the_scripts_and_only_them() {
    let names: Vec<String> = the_scripts().into_iter().map(|(_, name)| name).collect();
    for script in ["build.sh", "tests/terminal/record.sh", "assets/linux/install.sh", "tests/arduino/flash.sh", "measure.sh"] {
        assert!(names.iter().any(|name| name == script), "{script} not found among {names:?}");
    }
    // Rust's `#![...]` is no script, nor what is run through `python3 <script>`.
    let not_run_straight = ["src/lib.rs", "src/main.rs", "assets/make_icons.py", "tests/terminal/couleurs.py"];
    assert!(names.iter().all(|name| !not_run_straight.contains(&name.as_str())), "{names:?}");
}

/// The shell a script is written for, from its first line: `bash`, or `sh` (also for a `*.sh`
/// without one); none for a script of another language.
fn shell_of(path: &Path) -> Option<&'static str> {
    let text = fs::read_to_string(path).ok()?;
    let first = text.lines().next().unwrap_or_default();
    let shebang = first.starts_with("#!");
    if shebang && first.ends_with("bash") {
        Some("bash")
    } else if (shebang && (first.ends_with("/sh") || first.ends_with(" sh"))) || (!shebang && path.extension().is_some_and(|extension| extension == "sh")) {
        Some("sh")
    } else {
        None
    }
}

/// The scripts their shell cannot read, each with what the shell said.
fn unreadable(scripts: &[(PathBuf, String)]) -> Vec<String> {
    scripts
        .iter()
        .filter_map(|(path, name)| {
            let shell = shell_of(path)?;
            let read = Command::new(shell).arg("-n").arg(path).output().unwrap_or_else(|error| panic!("{shell}: {error}"));
            (!read.status.success()).then(|| format!("{name}: {}", String::from_utf8_lossy(&read.stderr).trim()))
        })
        .collect()
}

#[test]
fn every_shell_script_parses() {
    let scripts = the_scripts();
    let read = scripts.iter().filter(|(path, _)| shell_of(path).is_some()).count();
    assert!(read >= 4, "only {read} shell scripts among {scripts:?}");
    let broken = unreadable(&scripts);
    assert!(broken.is_empty(), "scripts their shell cannot read:\n{}", broken.join("\n"));

    // A script spoilt on purpose is found: the check does not pass for nothing.
    let folder = std::env::temp_dir().join("rsc-tests").join("scripts");
    fs::create_dir_all(&folder).expect("a temporary folder");
    let spoilt = folder.join("spoilt.sh");
    fs::write(&spoilt, "#!/usr/bin/env bash\nif true; then\n    echo never closed\n").expect("the spoilt script");
    let found = unreadable(&[(spoilt, "spoilt.sh".to_owned())]);
    assert_eq!(found.len(), 1, "{found:?}");
}
