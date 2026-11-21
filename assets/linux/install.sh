#!/bin/sh
# Installs RSC for this user from the folder this script is in, as a release archive lays it out:
# the program in ~/.local/bin, its entry in the menus, its icon in every size. No root needed.
#
#   ./install.sh               install, or install again over an earlier version
#   ./install.sh --uninstall   take those files away again
set -eu

here=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
bin="$HOME/.local/bin"
apps="$HOME/.local/share/applications"
icons="$HOME/.local/share/icons/hicolor"
sizes="16 24 32 48 64 128 256"

case "${1:-}" in
    "") ;;
    --uninstall)
        rm -f "$bin/rsc" "$apps/rsc.desktop"
        for size in $sizes; do rm -f "$icons/${size}x${size}/apps/rsc.png"; done
        echo "RSC removed from $bin, $apps and $icons"
        exit 0
        ;;
    *)
        echo "usage: $0 [--uninstall]" >&2
        exit 2
        ;;
esac

mkdir -p "$bin" "$apps"
cp "$here/rsc" "$bin/rsc"
chmod 755 "$bin/rsc"
# Its full path: ~/.local/bin is not always on the menus' PATH.
sed "s|^Exec=.*|Exec=$bin/rsc|" "$here/rsc.desktop" > "$apps/rsc.desktop"
for size in $sizes; do
    mkdir -p "$icons/${size}x${size}/apps"
    cp "$here/icons/rsc-$size.png" "$icons/${size}x${size}/apps/rsc.png"
done
# The menus see it at once where these tools are installed; at the next session otherwise.
if command -v update-desktop-database >/dev/null 2>&1; then update-desktop-database "$apps" || true; fi
if command -v gtk-update-icon-cache >/dev/null 2>&1; then gtk-update-icon-cache -q -t "$icons" || true; fi

echo "RSC installed: $bin/rsc, in the menus as RSC."
echo "To open serial ports, be in their group, then log out and in again:"
echo "    sudo usermod -aG dialout \"\$USER\"    (uucp on Arch Linux)"
