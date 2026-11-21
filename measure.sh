#!/usr/bin/env bash
# Measures RSC's start and rest on the release program built by ./build.sh. macOS and Linux; the
# Windows counterpart is measure.ps1.
#
#   ./measure.sh                     writes start.csv and rest.csv into build/measures
#   ./measure.sh --out <folder>      elsewhere
#   ./measure.sh --tabs 4            RSC with 4 tabs on no port (1 by default)
#   ./measure.sh --exe <rsc>         another program than build.sh's, an earlier one say
#   ./measure.sh --terminal          its tabs in the terminal mode
#   ./measure.sh --starts 15 --rests 10 --settle 10 --window 30    the counts and seconds, these by default
#
# Start: from RSC's launch to its window shown (eframe shows it once its first frame is drawn).
# Under macOS, a small Swift program written below launches RSC, then asks the list of the windows
# on screen every millisecond until one is RSC's; it reads neither their names nor their pictures,
# so it needs no permission. Under Linux, X11 only: xdotool is asked as fast as it answers (a few
# milliseconds a time), and RSC runs on X11 (without WAYLAND_DISPLAY), in a Wayland session too.
# Without xdotool or X11, the start is not measured and RSC runs as it would; the rest is measured.
# Rest: once the window is shown (or RSC launched, under Linux without xdotool) and Settle seconds
# have passed, the processor time RSC uses over Window seconds, and its memory at the end: private,
# the memory that is RSC's alone (macOS: its footprint, as the Activity Monitor shows it; Linux:
# Private_Clean + Private_Dirty of /proc/<pid>/smaps_rollup), and resident, all its pages in memory
# (RSS). The processor time is read to the hundredth of a second: over the 30 s of Window, a third
# of a millisecond per second. Windows counts memory otherwise: its CSV and these do not compare.
# The first run of each series is a warm-up, kept in the CSV with warmup=1 and left out of the
# statistics.
#
# Each run uses a settings file of its own (RSC_SETTINGS) in a temporary folder: the user's settings
# are not read. eframe writes the window's size and place to app.ron in the user's data folder
# every 30 s: it is copied first and put back at the end if a run changed it.
#
# CSV: semicolons and decimal commas, as Excel reads them in French.
set -euo pipefail

here=$(cd "$(dirname "$0")" && pwd)
out="$here/build/measures"
starts=15
rests=10
settle=10
window=30
tabs=1
terminal=0
exe=""

usage() {
    echo "usage: $0 [--out <folder>] [--starts <n>] [--rests <n>] [--settle <s>] [--window <s>] [--tabs <n>] [--exe <rsc>] [--terminal]" >&2
    exit 2
}
# A count or a number of seconds: digits only.
count() { case "$1" in "" | *[!0-9]*) usage ;; esac; }

while [ $# -gt 0 ]; do
    case "$1" in
        --out) [ $# -ge 2 ] || usage; out=$2; shift 2 ;;
        --starts) [ $# -ge 2 ] || usage; count "$2"; starts=$2; shift 2 ;;
        --rests) [ $# -ge 2 ] || usage; count "$2"; rests=$2; shift 2 ;;
        --settle) [ $# -ge 2 ] || usage; count "$2"; settle=$2; shift 2 ;;
        --window) [ $# -ge 2 ] || usage; count "$2"; window=$2; shift 2 ;;
        --tabs) [ $# -ge 2 ] || usage; count "$2"; tabs=$2; shift 2 ;;
        --exe) [ $# -ge 2 ] || usage; exe=$2; shift 2 ;;
        --terminal) terminal=1; shift ;;
        *) usage ;;
    esac
done
[ "$tabs" -ge 1 ] && [ "$window" -ge 1 ] || usage

system=$(uname -s)
case "$system-$(uname -m)" in
    Darwin-*) built="$here/build/macos/RSC.app/Contents/MacOS/rsc" ;;
    Linux-aarch64 | Linux-arm64) built="$here/build/linux-arm64/rsc" ;;
    Linux-*) built="$here/build/linux-x86/rsc" ;;
    *) echo "macOS and Linux only; under Windows, measure.ps1." >&2; exit 1 ;;
esac
exe=${exe:-$built}
[ -x "$exe" ] || { echo "No $exe : run ./build.sh first." >&2; exit 1; }
if pgrep -x rsc >/dev/null; then
    echo "RSC is running: close it first, it would be measured too." >&2
    exit 1
fi

# Where eframe keeps app.ron (eframe::storage_dir).
case "$system" in
    Darwin) data="$HOME/Library/Application Support/rsc" ;;
    *)
        case "${XDG_DATA_HOME:-}" in
            /*) data="$XDG_DATA_HOME/rsc" ;;
            *) data="$HOME/.local/share/rsc" ;;
        esac
        ;;
esac
app_ron="$data/app.ron"

mkdir -p "$out"
work=$(mktemp -d "${TMPDIR:-/tmp}/rsc-measure.XXXXXX")
had_app_ron=0
if [ -f "$app_ron" ]; then
    had_app_ron=1
    cp -p "$app_ron" "$work/app.ron"
fi
pid=""

# At the end, even on an error: RSC stopped, app.ron as it was, the temporary folder gone.
finish() {
    [ -z "$pid" ] || kill -KILL "$pid" 2>/dev/null || true
    if [ "$had_app_ron" = 1 ] && ! cmp -s "$app_ron" "$work/app.ron"; then
        cp -p "$work/app.ron" "$app_ron"
        echo "app.ron changed by the runs: put back as it was."
    elif [ "$had_app_ron" = 0 ] && [ -f "$app_ron" ]; then
        rm -f "$app_ron"
        echo "app.ron made by the runs: taken away."
    fi
    rm -rf "$work"
}
trap finish EXIT

# A number as Excel reads it in French: a decimal comma.
french() { LC_ALL=C awk -v value="$1" -v format="$2" 'BEGIN { printf format, value }' | tr . ,; }

# A clock in seconds, the same for every process of the machine.
clock() { python3 -I -c 'import time; print("%.6f" % time.monotonic())'; }

# The ways to see RSC's window: Swift's under macOS, xdotool's under Linux on X11, or none.
watch=none
if [ "$system" = Darwin ]; then
    watch=swift
    cat > "$work/window_wait.swift" <<'SWIFT'
// Launches RSC, waits for one of its windows on screen, then prints RSC's process id and the
// milliseconds from its launch. RSC's own output goes to the file named second: this program's
// output is read until it ends, and RSC outlives it.
import CoreGraphics
import Foundation

let rsc = Process()
rsc.executableURL = URL(fileURLWithPath: CommandLine.arguments[1])
rsc.standardInput = FileHandle.nullDevice
rsc.standardOutput = FileHandle.nullDevice
rsc.standardError = FileHandle(forWritingAtPath: CommandLine.arguments[2])
let start = DispatchTime.now().uptimeNanoseconds
try rsc.run()
let pid = rsc.processIdentifier

func shown() -> Bool {
    let options: CGWindowListOption = [.optionOnScreenOnly, .excludeDesktopElements]
    guard let windows = CGWindowListCopyWindowInfo(options, kCGNullWindowID) as? [[String: Any]] else { return false }
    return windows.contains { ($0[kCGWindowOwnerPID as String] as? Int32) == pid && ($0[kCGWindowLayer as String] as? Int) == 0 }
}

while !shown() {
    if !rsc.isRunning { exit(3) }
    if DispatchTime.now().uptimeNanoseconds - start > 30_000_000_000 { rsc.terminate(); exit(4) }
    usleep(1000)
}
print(pid, Double(DispatchTime.now().uptimeNanoseconds - start) / 1e6)
SWIFT
    swiftc -O -o "$work/window-wait" "$work/window_wait.swift" 2> "$work/swiftc.err" || {
        cat "$work/swiftc.err" >&2
        echo "swiftc failed: the Command Line Tools are needed (xcode-select --install)." >&2
        exit 1
    }
elif command -v xdotool >/dev/null 2>&1 && [ -n "${DISPLAY:-}" ]; then
    watch=xdotool
    echo "RSC runs on X11 (without WAYLAND_DISPLAY), so that xdotool sees its window."
else
    echo "No xdotool or no X11 display: the start is not measured, the rest is (counted from RSC's launch)."
fi

# Starts RSC on a settings file of its own; sets pid, and ms to its window ("" when not seen).
start_rsc() {
    local name=$1
    export RSC_SETTINGS="$work/$name.json"
    rm -f "$RSC_SETTINGS"
    # Several tabs, each on no port, in the terminal mode or not: what the settings file holds for them.
    if [ "$tabs" -gt 1 ] || [ "$terminal" = 1 ]; then
        local general='{}' list='{}' tab=1
        [ "$terminal" = 0 ] || general='{"terminal":true}'
        while [ "$tab" -lt "$tabs" ]; do list="$list,{}"; tab=$((tab + 1)); done
        printf '{"general":%s,"tabs":[%s]}\n' "$general" "$list" > "$RSC_SETTINGS"
    fi
    : > "$work/$name.err"
    ms=""
    case "$watch" in
        swift)
            local seen
            seen=$("$work/window-wait" "$exe" "$work/$name.err") || {
                echo "RSC ended before showing its window, or showed none within 30 s (run $name)." >&2
                exit 1
            }
            pid=${seen% *}
            ms=${seen#* }
            ;;
        xdotool)
            local begin now
            begin=$(date +%s%N)
            env -u WAYLAND_DISPLAY "$exe" < /dev/null > /dev/null 2> "$work/$name.err" &
            pid=$!
            until xdotool search --onlyvisible --pid "$pid" > /dev/null 2>&1; do
                kill -0 "$pid" 2>/dev/null || { echo "RSC ended before showing its window (run $name)." >&2; exit 1; }
                now=$(date +%s%N)
                [ $((now - begin)) -lt 30000000000 ] || { echo "No window within 30 s (run $name)." >&2; exit 1; }
            done
            now=$(date +%s%N)
            ms=$(LC_ALL=C awk -v ns=$((now - begin)) 'BEGIN { print ns / 1e6 }')
            ;;
        none)
            "$exe" < /dev/null > /dev/null 2> "$work/$name.err" &
            pid=$!
            ;;
    esac
}

stop_rsc() {
    kill -KILL "$pid" 2>/dev/null || true
    wait "$pid" 2>/dev/null || true
    while kill -0 "$pid" 2>/dev/null; do sleep 0.1; done
    pid=""
    sleep 0.5
}

# The processor time RSC has used so far, in milliseconds.
cpu_ms() {
    if [ "$system" = Darwin ]; then
        # [[hours:]minutes:]seconds.hundredths
        ps -o time= -p "$pid" | LC_ALL=C awk -F: '{ ms = 0; for (i = 1; i <= NF; i++) ms = ms * 60 + $i; print ms * 1000 }'
    else
        # utime and stime, the 14th and 15th fields, counted after the name in parentheses.
        LC_ALL=C awk -v hz="$(getconf CLK_TCK)" '{ sub(/^.*\) /, ""); print ($12 + $13) * 1000 / hz }' "/proc/$pid/stat"
    fi
}

# RSC's private and resident memory, in megabytes, on one line.
memory_mb() {
    local resident
    if [ "$system" = Darwin ]; then
        resident=$(ps -o rss= -p "$pid")
        footprint -p "$pid" | LC_ALL=C awk -v rss="$resident" '
            /Footprint:/ {
                for (i = 1; i < NF; i++) if ($i == "Footprint:") { value = $(i + 1); unit = $(i + 2) }
                if (unit == "B") value /= 1048576; else if (unit == "KB") value /= 1024; else if (unit == "GB") value *= 1024
                print value, rss / 1024
                exit
            }'
    else
        resident=$(LC_ALL=C awk '/^VmRSS:/ { print $2 }' "/proc/$pid/status")
        LC_ALL=C awk -v rss="$resident" '/^Private_(Clean|Dirty):/ { kb += $2 } END { print kb / 1024, rss / 1024 }' "/proc/$pid/smaps_rollup"
    fi
}

if [ "$watch" != none ]; then
    lines="run;warmup;ms"
    for run in $(seq 0 "$starts"); do
        start_rsc "start-$run"
        stop_rsc
        warmup=0; [ "$run" != 0 ] || warmup=1
        lines="$lines"$'\n'"$run;$warmup;$(french "$ms" '%.1f')"
        echo "start $run/$starts: $(LC_ALL=C awk -v ms="$ms" 'BEGIN { printf "%.1f", ms }') ms"
    done
    printf '%s\n' "$lines" > "$out/start.csv"
fi

lines="run;warmup;cpu_ms_per_s;private_mb;resident_mb"
for run in $(seq 0 "$rests"); do
    start_rsc "rest-$run"
    sleep "$settle"
    before=$(cpu_ms)
    from=$(clock)
    sleep "$window"
    after=$(cpu_ms)
    to=$(clock)
    read -r private resident <<< "$(memory_mb)"
    stop_rsc
    cpu=$(LC_ALL=C awk -v a="$after" -v b="$before" -v t="$from" -v u="$to" 'BEGIN { print (a - b) / (u - t) }')
    warmup=0; [ "$run" != 0 ] || warmup=1
    lines="$lines"$'\n'"$run;$warmup;$(french "$cpu" '%.2f');$(french "$private" '%.1f');$(french "$resident" '%.1f')"
    LC_ALL=C awk -v r="$run" -v n="$rests" -v c="$cpu" -v p="$private" -v s="$resident" \
        'BEGIN { printf "rest %d/%d: %.2f ms of processor per s, %.1f MB private, %.1f MB resident\n", r, n, c, p, s }'
done
printf '%s\n' "$lines" > "$out/rest.csv"
echo "-> $out"
