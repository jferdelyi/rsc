//! On Windows, the executable carries RSC's icon and its file properties (the Explorer, the task
//! manager, the firewall's prompt). Nothing elsewhere.

fn main() {
    println!("cargo:rerun-if-changed=assets/rsc.ico");
    #[cfg(windows)]
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        windows_resources();
    }
}

#[cfg(windows)]
fn windows_resources() {
    use std::{env, fs, path::PathBuf};

    // rc.exe does not take UNC paths (\\wsl.localhost\...), where the tool is often built from:
    // the icon is copied next to the generated resource script.
    let icon = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR")).join("rsc.ico");
    fs::write(&icon, include_bytes!("assets/rsc.ico")).expect("write the icon");
    let mut resource = winresource::WindowsResource::new();
    // ASCII only: rc.exe reads its script in the system's code page.
    resource
        .set_icon(icon.to_str().expect("UTF-8 path"))
        .set("ProductName", "RSC")
        .set("FileDescription", "RSC - Rust Serial Console")
        .set("OriginalFilename", "rsc.exe");
    resource.compile().expect("compile the Windows resources");
}
