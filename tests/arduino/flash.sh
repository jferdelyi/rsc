#!/usr/bin/env bash
# Flashes RSC's test program onto an Arduino Leonardo, or first keeps the program it holds. macOS and
# Linux; the Windows counterpart is flash.ps1.
#
#   tests/arduino/flash.sh --port /dev/cu.usbmodem1101 --keep <file.hex>   reads the program it holds into a file
#   tests/arduino/flash.sh --port /dev/cu.usbmodem1101                     compiles and flashes rsc_essai
#   tests/arduino/flash.sh --build-only                                    compiles rsc_essai, no board needed
#   ... --tools <folder>                                                   arduino-cli's folder, ~/.cache/rsc-arduino by default
#
# The port must carry an Arduino (USB vendor 2341): any other is refused. Under macOS it is a
# /dev/cu.usbmodem port; under Linux a /dev/ttyACM one, which the group dialout may open.
#
# arduino-cli is taken from <tools>/bin, else from the PATH, else under macOS from the Arduino IDE;
# this script never downloads it. Its data are kept in <tools>, with a configuration of their own:
# the core arduino:avr, at a pinned version, is installed there the first time (some 50 MB
# downloaded), and the user's own Arduino folders are neither read nor written. The program is
# copied and compiled in a temporary folder.
set -euo pipefail

here=$(cd "$(dirname "$0")" && pwd)
tools="${XDG_CACHE_HOME:-$HOME/.cache}/rsc-arduino"
core="arduino:avr@1.8.8"
fqbn="arduino:avr:leonardo"
ide_cli="/Applications/Arduino IDE.app/Contents/Resources/app/lib/backend/resources/arduino-cli"
port=""
keep=""
build_only=0

usage() {
    echo "usage: $0 --port <port> [--keep <file.hex>] [--tools <folder>]" >&2
    echo "       $0 --build-only [--tools <folder>]" >&2
    exit 2
}

while [ $# -gt 0 ]; do
    case "$1" in
        --port) [ $# -ge 2 ] || usage; port=$2; shift 2 ;;
        --keep) [ $# -ge 2 ] || usage; keep=$2; shift 2 ;;
        --tools) [ $# -ge 2 ] || usage; tools=$2; shift 2 ;;
        --build-only) build_only=1; shift ;;
        *) usage ;;
    esac
done
if [ "$build_only" = 1 ]; then
    [ -z "$port$keep" ] || usage
else
    [ -n "$port" ] || usage
fi

# arduino-cli: the folder's own, the PATH's, or the Arduino IDE's under macOS.
if [ -x "$tools/bin/arduino-cli" ]; then
    cli="$tools/bin/arduino-cli"
elif command -v arduino-cli >/dev/null 2>&1; then
    cli=$(command -v arduino-cli)
elif [ -x "$ide_cli" ]; then
    cli=$ide_cli
else
    echo "arduino-cli not found: install it (https://arduino.github.io/arduino-cli/latest/installation/)," >&2
    echo "or put it in $tools/bin." >&2
    exit 1
fi

# The Arduino tools for macOS are Intel programs: on Apple silicon, they need Rosetta 2.
if [ "$(uname -s)-$(uname -m)" = Darwin-arm64 ] && ! arch -x86_64 /usr/bin/true 2>/dev/null; then
    echo "The Arduino tools need Rosetta 2 on this Mac: softwareupdate --install-rosetta" >&2
    exit 1
fi

# A configuration of its own: neither the user's arduino-cli.yaml nor the user's folders.
config="$tools/arduino-cli.yaml"
mkdir -p "$tools"
if [ ! -f "$config" ]; then
    printf 'directories:\n  data: "%s"\n  downloads: "%s"\n  user: "%s"\n' \
        "$tools/data" "$tools/staging" "$tools/user" > "$config"
fi
arduino() { "$cli" --config-file "$config" "$@"; }

if [ ! -d "$tools/data/packages/arduino/hardware/avr/${core#*@}" ]; then
    echo "First run: $core and its tools go into $tools (some 50 MB downloaded)."
    arduino core update-index
    arduino core install "$core"
fi

work=$(mktemp -d "${TMPDIR:-/tmp}/rsc-essai.XXXXXX")
trap 'rm -rf "$work"' EXIT

# The machine's serial ports, one a line: "<address> <vid>:<pid>", the USB identity in capitals,
# "none" for a port that is not USB.
ports() {
    arduino board list --json "$@" | python3 -I -c '
import json, sys
for found in json.load(sys.stdin).get("detected_ports", []):
    port = found.get("port", {})
    properties = port.get("properties", {})
    try:
        usb = "%04X:%04X" % (int(properties["vid"], 16), int(properties["pid"], 16))
    except (KeyError, ValueError):
        usb = "none"
    if port.get("address"):
        print(port["address"], usb)
'
}

if [ "$build_only" = 0 ]; then
    usb=$(ports | awk -v port="$port" '$1 == port { print $2 }')
    case "$usb" in
        2341:*) echo "$port : an Arduino, USB $usb" ;;
        *)
            echo "$port carries no Arduino (USB '${usb:-none}'): refused." >&2
            arduinos=$(ports | awk '$2 ~ /^2341:/ { print $1 }' | tr '\n' ' ')
            [ -z "$arduinos" ] || echo "Arduino ports here: $arduinos" >&2
            exit 1
            ;;
    esac
    if [ ! -w "$port" ]; then
        echo "$port cannot be opened for writing: under Linux, join the group dialout" >&2
        echo "(sudo usermod -aG dialout \$USER), then log in again." >&2
        exit 1
    fi
fi

if [ -n "$keep" ]; then
    # The Leonardo's bootloader starts when its port is opened at 1200 bauds and closed, on a port of
    # its own (USB 2341:0036), for 8 seconds.
    case "$(uname -s)" in
        Darwin) stty -f "$port" 1200 ;;
        *) stty -F "$port" 1200 ;;
    esac
    boot=""
    start=$SECONDS
    while [ -z "$boot" ] && [ $((SECONDS - start)) -lt 10 ]; do
        boot=$(ports --discovery-timeout 500ms | awk '$2 == "2341:0036" { print $1; exit }')
    done
    [ -n "$boot" ] || { echo "No bootloader port within 10 s." >&2; exit 1; }
    echo "bootloader on $boot"
    avrdude=$(find "$tools/data/packages/arduino/tools/avrdude" -type f -path '*/bin/avrdude' | head -n 1)
    conf=$(find "$tools/data/packages/arduino/tools/avrdude" -type f -name avrdude.conf | head -n 1)
    "$avrdude" -C "$conf" -p atmega32u4 -c avr109 -P "$boot" -b 57600 -D -U "flash:r:$work/kept.hex:i"
    cp "$work/kept.hex" "$keep"
    echo "kept: $keep ($(wc -c < "$keep" | tr -d ' ') bytes)"
    exit 0
fi

mkdir -p "$work/rsc_essai"
cp "$here/rsc_essai/"* "$work/rsc_essai/"
arduino compile --fqbn "$fqbn" --build-path "$work/build" "$work/rsc_essai"
if [ "$build_only" = 1 ]; then
    echo "rsc_essai compiled: the tools work."
    exit 0
fi
arduino upload -p "$port" --fqbn "$fqbn" --input-dir "$work/build" "$work/rsc_essai"
echo "rsc_essai flashed on $port"
