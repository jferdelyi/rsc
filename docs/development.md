# Developing RSC

How RSC is built, tested, measured and published. For what RSC does, see the [user guide](guide.md).

- [Build and install](#build-and-install)
- [Releases](#releases)
- [Development](#development)
- [Performance](#performance)
- [The files](#the-files)
- [Dependencies and licences](#dependencies-and-licences)

## Build and install

RSC is built from its sources with Rust (1.100 or later).

| System | Command | Result | Build folder |
|---|---|---|---|
| Windows (Rust and the Visual Studio Build Tools) | `.\build.ps1`, `.\build.ps1 -Run` to start it, `.\build.ps1 -Install` to install it | `build\rsc.exe`, with its icon | `%LOCALAPPDATA%\rsc\target` |
| Linux x86 | `./build.sh`, or `./build.sh --run` | `build/linux-x86/rsc`, for the glibc of the machine that built it | `~/.cache/rsc/target` |
| Linux x86, to hand out | `./build.sh --linux-x86` | `build/linux-x86/rsc`, for glibc 2.31 and later (Ubuntu 20.04, Debian 11 and later) | same |
| Linux arm64, from Linux x86 | `./build.sh --linux-arm64` | `build/linux-arm64/rsc`, for glibc 2.31 and later | same |
| macOS | `./build.sh`, or `./build.sh --run` | `build/macos/RSC.app`, with its icon, signed ad hoc | same |

- **The builds with zig** (`--linux-x86`, `--linux-arm64`) need `cargo-zigbuild` and zig (variable `ZIG`), and for arm64 `rustup target add aarch64-unknown-linux-gnu`.
- **The release build** optimises the whole program at once (LTO): a smaller executable, for a rebuild of about two minutes.
- **`build.ps1`** refuses to replace `build\rsc.exe`, to install or to uninstall while RSC is open.

**Installing from source**

- **Windows**: `.\build.ps1 -Install` installs RSC for the user, `%LOCALAPPDATA%\Programs\RSC\rsc.exe`, with its shortcut in the Start menu. `.\build.ps1 -Uninstall` removes them.
- **Linux**: `./build.sh --install` installs RSC for the user, with its entry and icon in the menus: `~/.local/bin/rsc`, `~/.local/share/applications/rsc.desktop`, `~/.local/share/icons/hicolor/<size>/apps/rsc.png`. `./build.sh --uninstall` removes them. A release archive does the same with its own `install.sh`.
- **macOS**: `RSC.app` is the application, its icon included; move it to Applications.

## Releases

A tag `v<version>`, the version of `Cargo.toml`, publishes a release: `.github/workflows/release.yml` builds RSC on each system, archives it with `LICENSE` and the components' notices, then publishes the archives with their SHA-256 (`SHA256SUMS.txt`) and an attestation of where they were built (`gh attestation verify <archive> --repo jferdelyi/rsc`). `RSC.app` is signed ad hoc, which costs nothing; the executables are not signed with a paid certificate. Run by hand (Run workflow, in the Actions tab), the same workflow builds the archives and keeps them with the run, without publishing anything.

```bash
git tag v0.1.0
git push origin v0.1.0
```

## Development

```bash
cargo clippy --all-targets      # no warning tolerated: lints in deny
cargo test                      # the whole suite
RSC_CAPTURES=1 cargo test captures                                    # the window, light and dark, in build/captures
RSC_SCREENSHOTS=1 cargo test --lib readme_screenshots                 # the README's screenshots, in English, in docs/screenshots
RSC_LICENCES=write cargo test --test licences write_the_record       # make the licence record again (under Linux)
RSC_PORT=COM3 cargo test --test real_port -- --ignored --nocapture    # a real port: opens it, reads 3 s, closes it
RSC_PORT=COM3 cargo test --lib real_device -- --ignored --nocapture   # the window on a copy of your settings: the current tab connects with its profile, reads 5 s, disconnects
RSC_ARDUINO=COM3 cargo test --test arduino -- --ignored --nocapture --test-threads=1  # an Arduino running the test program: console, size, menu, measures, throughput
tests/terminal/record.sh                                              # record real programs again in tmux (under Linux)
cargo test --release --test measures -- --ignored --nocapture --test-threads=1       # measures: console, curves, terminal, memory, list of the ports
RSC_RUNS=5 cargo test --release --lib measure_frames -- --ignored --nocapture --test-threads=1  # measures: time of a frame (15 runs without RSC_RUNS)
cargo test --release --lib frame_time_of_the_curves -- --ignored --nocapture                   # measures: one frame of the curves, 5 curves of 100 000 points
cargo test --lib audit -- --ignored --nocapture --test-threads 1                                # accessibility (WCAG 2.2 AA): contrast, accessibility tree, keyboard, 200 %; on a machine with a GPU
```

**Where the tests are.** Rust keeps two kinds of tests, and RSC follows its rule:
- **Unit tests, in `src/`**, next to the code they test, in modules marked `#[cfg(test)]`: `cargo test` builds them, the program you run never holds them. They see inside the library, which the window's tests and the accessibility audit need: they build the window on ports played in memory, `FakePorts` (`src/link/fake.rs`, compiled for the tests only), where a test puts what the device sends and reads what RSC wrote. `src/ui/tests/window.rs`, for one, starts with `FakePorts::with(&["COM7"])`.
- **Integration tests, in `tests/`**: each file is a program of its own that sees only RSC's public interface, as another program using the library would, or the files of the repository: the licences, the icons, the vocabulary, the scripts, the terminal against tmux ([`tests/terminal/README.md`](../tests/terminal/README.md)), real pseudo-terminals, a real port, the Arduino, the measures.

- **Measures in Rust** write their CSV into `RSC_MEASURES`, otherwise into `build/measures`.
- **The accessibility audit** (`src/ui/audit/`, one file per criterion: contrast, names and targets, keyboard, 200 %, colour and drags, screen readers) measures the window against WCAG 2.2 AA in the test harness, without judging. The contrast of each pair of colours drawn, light and dark; the accessibility tree of each screen (role, name, size of the targets); Tab from end to end, the focused image compared with the same one without focus; the keys that could leave the terminal's screen; each screen at 200 %. Its tables and captures go into `RSC_AUDIT`, otherwise into `build/audit`. `looks` captures the screens at 100 %, light and dark, and the focus outline. Ordinary tests of the same files keep what it had corrected:
  - the focus outline, the order of Tab, leaving the terminal's screen, the separation with the keyboard, the settings that keep the keyboard, a name for each control, the pill's whole name, the contrast of each pair of colours;
  - at the size by default, the list of the settings' sections whole, without scrolling;
  - at the least size: the whole state, right of the tabs if need be, each section of the settings reached, the section chosen in view, the line of shortcuts and the cards of a profile and of a device whole, the curves and the **Display** choice, the window that holds on a Full HD screen at 200 % and on a 1366 × 768 laptop at 150 %, the first start maximised on a screen too low;
  - the vertical marks of each chart, over seven ranges of values, at both sizes, each written whole and apart from the others;
  - without colour or drag: a tab's state by its mark and in words, the name at the end of each curve, apart from the legend, the separation at a click, the pause's buttons; the visible labels of the send line and of the search;
  - for screen readers: the console's lines in view and its frame in the window, the rows of the terminal's screen and its cursor, the values of each chart, the state line announced, once per change; the console with the keyboard (Page Up, Page Down, Home, End).

  What the audit does not see: what a real screen reader says (Narrator, NVDA). `device-check.ps1 -Curves` reads the real window's tree through UI Automation, as Narrator does, and listens to Windows' announcements, saying nothing aloud.
- **The vocabulary**: `tests/vocabulary.rs` checks that the window's texts, in both languages, the README and these docs call a device a device. It also reads, when the machine has one, a local list of words that must never be written, `tests/banned.local`, which git ignores: one word per line, `=word` for a whole word only, `#` for a comment.
- **A script edited from Windows** (on `\\wsl.localhost`) loses its right to be run, and `./build.sh` then answers "Permission denied". `tests/scripts.rs` sees it under Linux and macOS and gives the command that repairs it (`chmod +x <script>`). The same test has each shell read its scripts without running them (`bash -n`, `sh -n`).
- **`.\measure.ps1`** (Windows) measures the start and the rest of `build\rsc.exe`, with temporary settings (27 starts by default; `-Starts` and `-Rests` for fewer), and writes its CSV into `build\measures` or into the folder of `-Out`. `-Tabs 4` opens RSC on 4 tabs without a port; `-Terminal` puts them in terminal mode; `-Exe` measures another executable, an earlier version for instance.
- **`./measure.sh`** (macOS and Linux) measures the same on the program `./build.sh` made, with the same options written `--starts`, `--rests`, `--settle`, `--window`, `--tabs`, `--exe`, `--terminal`, `--out`. Under macOS, a small Swift program, compiled on the spot (the Command Line Tools are needed), watches the list of the windows for RSC's, without any permission. Under Linux, the start is measured on X11 only, with `xdotool` (RSC then runs without `WAYLAND_DISPLAY`); otherwise only the rest is. Its memory columns are `private_mb` (macOS: the footprint; Linux: the private pages) and `resident_mb` (RSS): they do not compare with Windows' `private_mb` and `working_set_mb`.
- **`.\window-check.ps1 -Port COM3`** (Windows) goes through the real window of `build\rsc.exe` as a person would, through Windows accessibility, without moving the mouse: the least size, the Profiles and Devices sections, a profile of the library chosen through the pill on an unnamed USB port, the device named, About, a licence, the window that saves the notices (its type checked as `*.txt`, then cancelled), and the terminal's line of shortcuts. It captures the window at each stage in `build\captures\vraie-<theme>-*.png`, except the save window, which shows your folders. Temporary settings; the port is listed, never opened; `-Theme dark` for a single theme.
- **The test program for Arduino** (`tests/arduino/rsc_essai/rsc_essai.ino`, written for a Leonardo): a small console `essai>` that handles its own echo, Backspace, the up arrow and Ctrl+C. Its commands exercise RSC on a real device:
  - `aide` and `version`: the console and line editing;
  - `taille`: the device asks the screen's size (`ESC [ 6 n`, `ESC [ 18 t`) and shows RSC's answers; in line mode, it says no terminal answered;
  - `menu`: a menu on the alternate screen, in a frame of DEC's set, the chosen line in reverse, led with the arrows, Enter and Escape;
  - `couleurs`: the 16 colours, the cube of 256, direct colours; `journal`: coloured lines like ESP-IDF's;
  - `mesures`: `temp=21.50 pression=1013.2 compteur=42` every 100 ms, until Ctrl+C, Enter or `q`;
  - `flux`: 3 s of numbered lines as fast as USB takes them, then their count and the throughput.
- **`tests\arduino\flash.ps1 -Port COM3`** (Windows) builds and flashes this program with arduino-cli, kept with its `arduino:avr` core in `%LOCALAPPDATA%\rsc-arduino` (about 460 MB; `-Tools` for another folder). `-Keep <file.hex>` first reads the program the board holds, through its bootloader, to put it back later. It refuses any port that does not carry an Arduino (USB vendor 2341).
- **`tests/arduino/flash.sh --port /dev/cu.usbmodem1101`** (macOS and Linux; `/dev/ttyACM0` or so under Linux) does the same: the same `--keep <file.hex>`, the same refusal. It takes arduino-cli from `~/.cache/rsc-arduino/bin`, the `PATH` or, under macOS, the Arduino IDE, and never downloads it; the first run installs the core `arduino:avr` 1.8.8 into `~/.cache/rsc-arduino` (about 50 MB downloaded, 380 MB installed; `--tools` for another folder), never into your own Arduino folders. `--build-only` compiles the program without a board, to check the tools first. On a Mac with Apple silicon, the Arduino tools need Rosetta 2; under Linux, opening the port needs the group `dialout`.
- **`.\device-check.ps1 -Port COM3`** (Windows): the real window of `build\rsc.exe` on an Arduino running the test program, with Windows' real keyboard. Temporary settings (one tab on this port, in terminal mode); it clicks **Connect** through accessibility, brings the window to the front, then types `taille`, `menu`, down arrow, Enter, `couleurs`, `mesures` and Ctrl+C. Before each key, it checks RSC's window is in front, and stops otherwise. Type nothing during its 20 s or so. Captures in `build\captures\appareil-*.png`. With `-Curves`, the curves on and `pression` in the lower chart, then what a screen reader receives, read through UI Automation: the terminal screen's rows, the state line, the console's lines; last, it closes the port while listening to Windows' announcements.
- **The logo and the icons**: how they are drawn is in [`assets/README.md`](../assets/README.md).

## Performance

Measured on 19/11/2026 on a single build, on an AMD Ryzen AI 7 350 under Windows 11, on mains power, each series started after a minute of calm (medians, `release` build).

| Measure | Result |
|---|---|
| Executable | 11.6 MB under Windows, 17.0 MB under Linux x86, 15.0 MB under Linux arm64, 0.7 MB of which for the licence notices |
| Start to RSC's window | 111.1 ms (107.5 to 115.2) |
| Rest, one tab without a port, line mode | 8.6 ms of processor per second (5.5 to 8.6); 102.7 MB of private memory |
| Rest, one tab without a port, terminal mode | 7.8 ms of processor per second (7.8 to 12.5); 102.7 MB of private memory |
| Rest, 4 tabs without a port (17/11/2026, not measured again) | 15.6 ms of processor per second, 127 MB of private memory |
| Console | 52 MB of log per second (49 to 54), about 570 times what 921 600 bauds carry |
| Curves | 1 050 000 lines of measures per second (944 000 to 1 074 000), about 370 times 921 600 bauds |
| Memory of a console line | 307 to 349 bytes: 3.5 MB for 10 000 lines, 307 MB for 1 000 000 |
| Memory of a curve point | 52 to 73 bytes, its measure's number included: 18.4 MB for 100 000 lines of 3 values |
| One frame of the console, data at 921 600 bauds | 0.47 ms with 10 000 lines kept, 0.56 ms with 100 000, 0.83 ms with 1 000 000 (egui's part only, the tree for screen readers included); 0.26 to 0.32 ms when nothing comes |
| One frame of the curves, 5 curves of 100 000 points, all shown | 3.3 ms; 0.6 ms over 1 min |
| List of the ports | 6.0 ms for 1 port, made by a single thread for all tabs: every second while a tab has not its port open, every 2 s when all have |
| Terminal mode: the screen's throughput | 73 MB/s on an ESP-IDF log (about 800 times 921 600 bauds), 106 MB/s on a whiptail menu drawn again |
| Terminal mode: one frame of a full screen, 128×27 | 0.28 ms, 0.44 ms while data comes at 921 600 bauds |
| Terminal mode: the history | about 1.3 KB a row, 13.4 MB for 10 000 rows |

From one campaign to the next, on unchanged code, the throughput and frame figures moved by up to 30 % on this machine. A smaller gap does not read as a change: to judge a change, measure before and after in the same session.

The drawing handles only the visible lines, and the folding of long lines follows the console as it goes: the cost of a frame hardly depends on the number of lines kept any more.

## The files

| File | Role |
|---|---|
| `Cargo.toml`, `Cargo.lock` | the package, its own root; versions pinned, lints in deny, LTO in release |
| `LICENSE` | RSC's licence: the Eclipse Public License 2.0 |
| `build.rs` | under Windows, the icon and the properties of `rsc.exe` |
| `build.ps1`, `build.sh` | release build and copy into `build/`; under Windows and Linux, install and uninstall; under macOS, `RSC.app`; Linux x86 and arm64 for glibc 2.31 (`--linux-x86`, `--linux-arm64`) |
| `measure.ps1`, `measure.sh` | RSC's start and rest, on one or several tabs, as CSV: `measure.ps1` under Windows, `measure.sh` under macOS and Linux |
| `window-check.ps1` | under Windows, the real window walked through accessibility, with its captures |
| `device-check.ps1` | under Windows, the real window on an Arduino running the test program, with the real keyboard, with its captures |
| `src/main.rs` | the executable: the window |
| `src/lib.rs` | the modules |
| `src/i18n.rs` | `t!` and `tr!`: each text in French and in English |
| `src/settings/` | general profile, library of profiles, named devices, tabs; safe reading and writing of the file (`mod.rs`); the values of a setting (`values.rs`), said plainly (`describe.rs`); the list of a profile's settings (`profile.rs`); the files of earlier versions taken in (`legacy.rs`); its tests by theme (`tests/`) |
| `src/ports.rs` | the system's ports, their product and USB identity; finding a device again |
| `src/link/` | the ports' threads: what the window uses, commands, events and a tab's end of the thread (`mod.rs`); the single thread that lists the ports for all tabs (`watch.rs`); the thread that owns a tab's port: open, read, write, close (`worker.rs`); the machine's real ports (`system.rs`); ports played in memory for the tests (`fake.rs`); its tests (`tests.rs`) |
| `src/console/` | the text received (`mod.rs`); a line and what it shows (`line.rs`); the bytes received made into lines, UTF-8 and sequences cut between two reads, ANSI colours (`decode.rs`); the folding of long lines as it goes (`wrap.rs`); its tests by theme (`tests/`) |
| `src/terminal/` | the terminal mode: the screen (vt100) and its history (`mod.rs`); the answers to the device's questions (`answers.rs`); DEC's line drawing set (`line_drawing.rs`); keys and paste as bytes (`keys.rs`); its tests (`tests.rs`) |
| `src/plot.rs` | the measures: reading the lines, curves, reduction to the width shown, CSV export |
| `src/send.rs` | the line to send: line ending, escapes, hexadecimal, history |
| `src/journal.rs` | the journal as it goes |
| `src/licences.rs` | the licence record the program carries, and its whole text |
| `src/ui/mod.rs` | the window: its public parts, a tab's state (`Session`), `App`, its making, its step (`tick`) and its drawing from top to bottom, the link with eframe |
| `src/ui/session.rs` | a tab's link: its port's events, open, close, reconnect, send a line, the lines given to the curves and the journal; add and close a tab; save the settings |
| `src/ui/files.rs` | the files: the save window and its types, export or copy the console, the journal, the curves' CSV, send a file |
| `src/ui/top.rs` | the top of the window: tabs, connection bar, baud rate, state line and switches |
| `src/ui/pill.rs` | the profile pill and its menu: choose a profile of the library, make one from the current one, name the device |
| `src/ui/middle.rs` | the middle: console or terminal, curves, separation, the terminal's line of shortcuts |
| `src/ui/bottom.rs` | the bottom: send line and actions |
| `src/ui/tests/` | the window's tests, one file per theme (link, profiles, files, terminal, console and curves, window, what is run by hand), and their helpers in `mod.rs` |
| `src/ui/console_view/` | the console on screen, visible lines only, scrolling followed (`mod.rs`); a frame's scene and the lines in view for screen readers (`scene.rs`); selection, mouse and keyboard (`input.rs`); drawing the rows and the minimap (`paint.rs`); its tests (`tests.rs`) |
| `src/ui/terminal_view.rs` | the terminal's screen: cells and colours, cursor, selection, history at the wheel, keyboard |
| `src/ui/plot_view/` | the curves under the console, their heights and what the window gives them (`mod.rs`); their bar (`bar.rs`); one or two charts and the vertical scale over the part chosen (`charts.rs`); the marks, the values written short, what the pointer shows (`marks.rs`); their tests (`tests.rs`) |
| `src/ui/icons.rs` | the Lucide icons used, by name |
| `src/ui/widgets.rs` | icon button, switch, segmented choice, label with its explanation; the painted text areas and their lines for screen readers |
| `src/ui/settings_window/` | the settings window: profile changed at the top, sections at the left (`mod.rs`); a section's rows (`rows.rs`); a profile's sections (`profile.rs`); the library of profiles (`profiles.rs`); the named devices (`devices.rs`); Look and About (`application.rs`) |
| `src/ui/theme.rs` | the colours of the light and dark themes, the filled buttons and their states, the console, the logo |
| `src/ui/audit/` | the window's accessibility audit (WCAG 2.2 AA), one file per criterion, as ignored tests run by hand, and the tests that keep what it had corrected; the screens and common helpers in `mod.rs` |
| `tests/pty.rs` | a real serial terminal of the system (a pair of pseudo-terminals), Linux and macOS; a real bash and vi on the terminal's screen, from end to end |
| `tests/terminal.rs`, `tests/terminal/` | the terminal's screen compared with tmux's, cell by cell, on real programs (bash, vi, less, nano, a whiptail menu, curses in 256 colours); `record.sh` records them, under Linux |
| `tests/real_port.rs` | a real port named by `RSC_PORT`, run by hand |
| `tests/arduino.rs`, `tests/arduino/` | an Arduino named by `RSC_ARDUINO`, run by hand: the link's thread, the terminal's screen and RSC's console on the test program; `rsc_essai/` is that program, `flash.ps1` (Windows) and `flash.sh` (macOS, Linux) flash it |
| `tests/measures.rs` | the measures of the console, the curves and the list of the ports, with an allocator that counts memory, run by hand |
| `tests/assets.rs` | the form of each system's icons and of the `.desktop` file |
| `tests/licences.rs` | the licence record: making it, and checking it follows the machine's dependencies |
| `tests/scripts.rs` | under Linux and macOS, each script run straight (`*.sh`, or a first line `#!/`) keeps its right to be run, and its shell reads it without error (`bash -n`, `sh -n`) |
| `tests/vocabulary.rs` | the window's texts (French and English), the README and `docs/`: a generic tool, which says "device" and names no company |
| `assets/fonts/` | the icons' font (Lucide 1.74.0), its licence and where it comes from |
| `assets/licences/` | the licence record (`licences.json`, which the program carries), its text (`THIRD-PARTY-LICENSES.txt`), the standard texts and where they come from (`textes/`) |
| `assets/` | the logo and icons, the Linux menu entry and `install.sh`, what draws them: see [`assets/README.md`](../assets/README.md) |
| `docs/guide.md`, `docs/development.md` | the user guide, and this page |
| `docs/screenshots/` | the README's screenshots, drawn by `readme_screenshots` (`src/ui/tests/by_hand.rs`) |
| `.github/workflows/ci.yml` | the CI: clippy with no warning, then the test suite, on Windows, macOS and Linux |
| `.github/workflows/release.yml` | the release: the archives of each system, their SHA-256 and their attestation, on a version tag |

## Dependencies and licences

| Package | Version | Role |
|---|---|---|
| eframe | 0.36.2 | the window, drawn through OpenGL with glow rather than wgpu (141 MB at rest instead of 348 under Windows, measured on 17/11/2026) |
| egui_plot | 0.37.0 | the curves |
| serialport | 4.10.1 | the ports, without libudev |
| vt100, vte | 0.18.1, 0.15.0 | the terminal mode's screen |
| serde, serde_json | 1.0.229, 1.0.151 | the settings |
| rfd | 0.15.4 | the system's file windows |
| chrono | 0.4.45 | the local time |
| winit (Linux) | 0.30.13 | under Wayland, the title bar the window draws itself |
| winresource (Windows, build) | 0.1.31 | the icon of `rsc.exe` |
| egui_kittest, ab_glyph (tests) | 0.36.2, 0.2.32 | the window driven without a screen; the fonts' tables |
| Lucide | 1.74.0 | the icons' font (ISC, and MIT for what comes from Feather) |

**Licences**, on 18/11/2026, from the licence each package declares:

- 283 packages in the executable of at least one target (138 under Windows, 231 under Linux x86 and arm64, 155 under macOS), mostly under MIT or Apache-2.0 at choice; with them, in each executable, the Lucide font and Rust's standard library;
- ask for something:
  - `serialport`, under MPL-2.0: its files stay under that licence, and its notice says where its source code is;
  - the window's fonts (`epaint_default_fonts`), under OFL-1.1 and Ubuntu Font 1.0;
  - 16 packages under Unicode-3.0, a permissive licence with a notice to keep.

The components' notices go with each copy:

- **RSC carries them in itself**: Settings, About. Each component, its licence, where its source code is, and its texts. **Save the notices…** writes, into a file to join to a copy (`RSC-LICENSES.txt`), RSC's licence then all these notices. The components' alone are also in `assets/licences/THIRD-PARTY-LICENSES.txt`.
- **The record** is `assets/licences/licences.json`, made by `tests/licences.rs`:
  - the list of each target comes from `cargo tree`, along the program's dependencies only (not `cargo metadata`, which would count `wgpu`, turned on by a dependency of the tests);
  - the texts come from each package's licence files, in cargo's cache;
  - a package that joins none gets the standard text of its licence, after its authors (`assets/licences/textes/`, with where they come from).
- **After an update of the packages**, the test suite fails until the record is made again. Make it again on a machine whose cargo cache has the packages of the four targets (`cargo fetch --locked` if some are missing):

```bash
RSC_LICENCES=write cargo test --test licences write_the_record
```
