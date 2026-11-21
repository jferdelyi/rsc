#!/usr/bin/env bash
# Builds RSC in release and copies it into build/. Linux and macOS; the Windows counterpart is
# build.ps1, whose rsc.exe carries its icon.
#
#   ./build.sh                build for this machine: build/linux-x86/rsc or build/linux-arm64/rsc
#                             under Linux, build/macos/RSC.app under macOS (with its icon)
#   ./build.sh --run          then start it
#   ./build.sh --install      Linux: then install it for this user, its icon in the menus and on its
#                             window: ~/.local/bin/rsc, ~/.local/share/applications/rsc.desktop,
#                             ~/.local/share/icons/hicolor/<size>/apps/rsc.png
#   ./build.sh --uninstall    Linux: take those files away again (nothing is built)
#   ./build.sh --linux-arm64  for a 64-bit ARM Linux (Raspberry Pi OS 64-bit, Debian 11 and later),
#                             into build/linux-arm64/rsc, from a Linux x86-64 machine
#   ./build.sh --linux-x86    for a 64-bit x86 Linux older than this machine's (Ubuntu 20.04, Debian
#                             11 and later), into build/linux-x86/rsc: the one to hand out
#
# These two builds need: rustup target add aarch64-unknown-linux-gnu (ARM only), cargo install
# cargo-zigbuild, and zig (`pip install ziglang` in a venv; ZIG names its binary).
# They ask for glibc 2.31 at most; the build for this machine asks for its own glibc.
# Cargo's target directory stays out of the folder, in ~/.cache/rsc/target.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$HOME/.cache/rsc/target}"

usage() { echo "usage : $0 [--run | --install | --uninstall | --linux-arm64 | --linux-x86]" >&2; exit 2; }
run=false
# The target of a build with zig, for an older glibc; empty for this machine's.
zig_target=""
install=false
uninstall=false
# ${1+"$@"}: no argument is not an unbound variable under bash 3.2 (macOS).
for arg in ${1+"$@"}; do
    case "$arg" in
        --run) run=true ;;
        --linux-arm64) zig_target=aarch64-unknown-linux-gnu ;;
        --linux-x86) zig_target=x86_64-unknown-linux-gnu ;;
        --install) install=true ;;
        --uninstall) uninstall=true ;;
        *) usage ;;
    esac
done
if [ "$#" -gt 1 ]; then usage; fi

system="$(uname -s)"
# Where --install puts RSC, and the sizes of its icons (assets/linux, from make_platform_icons.py).
bin="$HOME/.local/bin"
apps="$HOME/.local/share/applications"
icons="$HOME/.local/share/icons/hicolor"
sizes="16 24 32 48 64 128 256"

if "$install" || "$uninstall"; then
    [ "$system" = Linux ] || { echo "--install et --uninstall : sous Linux seulement" >&2; exit 1; }
fi

if "$uninstall"; then
    rm -f "$bin/rsc" "$apps/rsc.desktop"
    for size in $sizes; do rm -f "$icons/${size}x${size}/apps/rsc.png"; done
    echo "RSC retiré de $bin, $apps et $icons"
    exit 0
fi

if [ -n "$zig_target" ]; then
    target="$zig_target"
    case "$target" in
        aarch64-*) out="$here/build/linux-arm64" ;;
        *) out="$here/build/linux-x86" ;;
    esac
    command -v cargo-zigbuild >/dev/null || { echo "cargo-zigbuild manque : cargo install cargo-zigbuild" >&2; exit 1; }
    rustup target list --installed | grep -qx "$target" || { echo "cible manquante : rustup target add $target" >&2; exit 1; }
    # cargo-zigbuild finds zig on PATH, or through CARGO_ZIGBUILD_ZIG_PATH.
    if [ -n "${ZIG:-}" ]; then export CARGO_ZIGBUILD_ZIG_PATH="$ZIG"; fi
    cargo zigbuild --release --manifest-path "$here/Cargo.toml" --target "$target.2.31"
    mkdir -p "$out"
    cp "$CARGO_TARGET_DIR/$target/release/rsc" "$out/rsc"
    echo "-> $out/rsc"
    exit 0
fi

cargo build --release --manifest-path "$here/Cargo.toml"
case "$system-$(uname -m)" in
    Linux-x86_64) out="$here/build/linux-x86" ;;
    Linux-aarch64 | Linux-arm64) out="$here/build/linux-arm64" ;;
    Darwin-*) out="$here/build/macos" ;;
    *) out="$here/build/$system-$(uname -m)" ;;
esac
mkdir -p "$out"

if [ "$system" = Darwin ]; then
    # An application of its own: its icon in the Finder and the Dock, its name in the menu bar.
    app="$out/RSC.app"
    rm -rf "$app"
    mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
    cp "$CARGO_TARGET_DIR/release/rsc" "$app/Contents/MacOS/rsc"
    cp "$here/assets/rsc.icns" "$app/Contents/Resources/rsc.icns"
    version="$(sed -n 's/^version = "\(.*\)"$/\1/p' "$here/Cargo.toml" | head -n 1)"
    cat > "$app/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key><string>RSC</string>
    <key>CFBundleDisplayName</key><string>RSC</string>
    <key>CFBundleIdentifier</key><string>io.github.jferdelyi.rsc</string>
    <key>CFBundleExecutable</key><string>rsc</string>
    <key>CFBundleIconFile</key><string>rsc</string>
    <key>CFBundlePackageType</key><string>APPL</string>
    <key>CFBundleShortVersionString</key><string>$version</string>
    <key>CFBundleVersion</key><string>$version</string>
    <key>NSHighResolutionCapable</key><true/>
    <key>LSMinimumSystemVersion</key><string>11.0</string>
</dict>
</plist>
PLIST
    # Signed ad hoc, which costs nothing: an application with no signature at all is said
    # "damaged" once downloaded, one signed ad hoc comes from "an unidentified developer".
    codesign --force --deep --sign - "$app"
    echo "-> $app"
    if "$run"; then open "$app"; fi
    exit 0
fi

cp "$CARGO_TARGET_DIR/release/rsc" "$out/rsc"
echo "-> $out/rsc"

if "$install"; then
    install -Dm755 "$out/rsc" "$bin/rsc"
    # Its full path: ~/.local/bin is not always on the menus' PATH.
    sed "s|^Exec=.*|Exec=$bin/rsc|" "$here/assets/rsc.desktop" > "$CARGO_TARGET_DIR/rsc.desktop"
    install -Dm644 "$CARGO_TARGET_DIR/rsc.desktop" "$apps/rsc.desktop"
    for size in $sizes; do install -Dm644 "$here/assets/linux/rsc-$size.png" "$icons/${size}x${size}/apps/rsc.png"; done
    # The menus see it at once where these are installed; at the next session otherwise.
    if command -v update-desktop-database >/dev/null; then update-desktop-database "$apps" || true; fi
    if command -v gtk-update-icon-cache >/dev/null; then gtk-update-icon-cache -q -t "$icons" || true; fi
    echo "RSC installé : $bin/rsc, $apps/rsc.desktop, $icons/<taille>/apps/rsc.png"
fi

if "$run"; then
    "$out/rsc" &
fi
