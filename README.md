# RSC · Rust Serial Console

**The ultimate serial console for boards, sensors and modems.**

- **The link**: pick a port or a named device, set the link, read what the device sends (as coloured text or as hexadecimal) and send it lines or a file.
- **The console**: copy it, export it, write it to a journal as it goes; see the measures as curves.
- **The terminal mode**: the keys go to the device one by one, and its screen is drawn as a terminal draws it (a Linux shell and vi, the ESP-IDF console, U-Boot, menus).
- **Several ports at once**, one per tab.
- **Profiles**: a library of named profiles, each with all its values, and a general profile for the rest. A USB device you name is found again on any port, with the profile you give it; a tab on another port keeps its own.

The window speaks French or English (Settings, Look). The Cargo package and the executable are both named `rsc`. RSC opens no network connection.

## Platforms

| Platform | State on 21/11/2026 |
|---|---|
| Windows 11, x86-64 | **tested**: test suite, the real window driven through Windows UI Automation (`window-check.ps1`), a real ESP-IDF board, an Arduino Leonardo running RSC's test program (`tests/arduino.rs`, `device-check.ps1`), measures, installation. Not yet heard with a real screen reader |
| Linux x86-64 | **tested**: test suite (with a real pseudo-terminal of the system, and a real bash and vi on the terminal screen), real window under WSLg. The title bar the window draws under a Wayland desktop (GNOME) is not checked. The portable build (`./build.sh --linux-x86`) closed at start 2 times out of 3 under WSLg's Wayland, cause not found; once open, it holds |
| Linux arm64 | **built** (`./build.sh --linux-arm64`), never run |
| macOS arm64 | **test suite** run on 21/11/2026 (with a real pseudo-terminal of the system, and a real bash and vi on the terminal screen); `RSC.app` is written in `build.sh`, never built |

## Getting started

1. Plug the device in, pick it in the first list of the bar (a port, or a device already named), set the baud rate, then **Connect**. What the device sends shows in the console.
2. Type a line at the bottom, then Enter; the up and down arrows bring back the lines already sent.
3. For other settings: click the **General profile** pill in the bar, type a name under **New profile, from this one**, then **Make and choose this profile**. For the device to find it again on any port: **Name this device…**, in the same menu. The pill then says which profile the tab uses.

## Build and install

RSC is built from its sources with Rust (1.100 or later).

| System | Command | Result | Build folder |
|---|---|---|---|
| Windows (Rust and the Visual Studio Build Tools) | `.\build.ps1`, `.\build.ps1 -Run` to start it, `.\build.ps1 -Install` to install it | `build\rsc.exe`, with its icon | `%LOCALAPPDATA%\rsc\target` |
| Linux x86 | `./build.sh`, or `./build.sh --run` | `build/linux-x86/rsc`, for the glibc of the machine that built it | `~/.cache/rsc/target` |
| Linux x86, to hand out | `./build.sh --linux-x86` | `build/linux-x86/rsc`, for glibc 2.31 and later (Ubuntu 20.04, Debian 11 and later) | same |
| Linux arm64, from Linux x86 | `./build.sh --linux-arm64` | `build/linux-arm64/rsc`, for glibc 2.31 and later | same |
| macOS (never built) | `./build.sh` | `build/macos/RSC.app`, with its icon | same |

- **The builds with zig** (`--linux-x86`, `--linux-arm64`) need `cargo-zigbuild` and zig (variable `ZIG`), and for arm64 `rustup target add aarch64-unknown-linux-gnu`.
- **The release build** optimises the whole program at once (LTO): a smaller executable, for a rebuild of about two minutes.
- **`build.ps1`** refuses to replace `build\rsc.exe`, to install or to uninstall while RSC is open.

**Running an executable built elsewhere**

- **Windows**: RSC is not signed. Before extracting a zip, right-click it, Properties, tick **Unblock**, then OK; otherwise SmartScreen stops RSC at its first start. If SmartScreen shows anyway: **More info**, then **Run anyway**. `rsc.exe` can be copied alone anywhere.
- **Linux**: it needs glibc 2.31 or later and a graphical desktop with OpenGL; under X11, `libxkbcommon-x11` too (package `libxkbcommon-x11-0` on Debian and Ubuntu). If the window closes at once under Wayland, start it again. Serial ports belong to a group (`dialout` on Debian, Ubuntu and Fedora, `uucp` on Arch): `sudo usermod -aG dialout $USER`, then log out and in again.
- **macOS**: built on the Mac itself, `RSC.app` opens directly. Copied from another Mac, it is not notarised: open it once, then in System Settings, Privacy & Security, click **Open Anyway**.

**RSC's icon on each system**

- **Windows**: it is in `rsc.exe`. Explorer does not draw the large icon of a program on a network share (`\\wsl.localhost`): `.\build.ps1 -Install` installs RSC for the user on the local disk, with its logo in the Start menu, the search, the taskbar and Explorer (`%LOCALAPPDATA%\Programs\RSC\rsc.exe` and the Start menu shortcut `RSC`). `.\build.ps1 -Uninstall` removes them.
- **Linux**: an executable carries no icon. `./build.sh --install` installs RSC for the user, with its icon in the menus and on its window: `~/.local/bin/rsc`, `~/.local/share/applications/rsc.desktop`, `~/.local/share/icons/hicolor/<size>/apps/rsc.png`. `./build.sh --uninstall` removes them.
- **macOS**: in `RSC.app`, from `assets/rsc.icns`.

## The window

- **Tabs**: one per port or named device, all open at once; **+** adds one, **×** closes one. The mark before the name tells the state by its shape as much as by its colour: a green disc, connected; a ring, closed; an orange three-quarter ring, connecting or waiting to reconnect; a red cross, an error. The tab's tooltip and screen readers say it in words. The same mark is used on the state line and on a named device's card (disc: plugged in; ring: absent).
- **Connection bar**: the port or named device, the profile pill, the baud rate, then **Connect**.
  - The first list shows the named devices (plugged in or absent), then the ports with no named device, and **Name this device…** when the tab's port carries an unnamed USB device.
  - Changing the port or the baud rate while the port is open reopens it at once.
- **The tab's profile**:
  - a tab on a named device uses that device's profile; on another port, the one the tab chose; otherwise, the general profile;
  - the pill in the bar says which (**General profile**, or **Profile ESP32 921600**), and a click opens its menu:
    - a sentence says who the choice goes to: the named device, which keeps it on any port, or the tab;
    - the list of profiles, **General** first, the current one marked;
    - **New profile, from this one**: a name, then **Make and choose this profile** (a copy of the current profile, chosen at once);
    - **Change in the settings…**, and on an unnamed USB port, **Name this device…**, which opens the Devices section with the name filled from the USB product;
  - the bar's baud rate, the switches **Timestamps**, **Hex view**, **Curves** and **Terminal**, the hex input, the line ending, and in the curves **Time | No.** and **Above or below**, change this profile. Their tooltip names the profile they change.
- **State**: the port, the link (`115200 8N1`), the flow control, the last problem, the journal running. At the right, switches: **Timestamps**, **Auto scroll**, **Hex view**, **Curves**, **Terminal**. On a narrow window, or with large text, the state goes up at the right of the tabs; if it does not hold there either (many tabs), the switches go on a row under the state. The state always stays whole.
- **Size**: the window does not go below 900 × 430 points; with its title bar and the Windows 11 taskbar, it holds whole on a 1366 × 768 laptop at 150 % and on a Full HD screen (1920 × 1080) at 200 %. On a screen too low for its starting size, it starts maximised.
- **Keyboard**: Tab goes through the window in reading order, Shift+Tab backwards. A blue outline surrounds the control reached; it goes at the first mouse click. Once open, the settings keep the keyboard and the mouse: the window behind does not answer until **Close** or ×. Each control has a name for screen readers; the profile pill gives its whole text there, even when the bar cuts it.
- **Screen readers**: the console is a log whose every line in view is an element, its text as it shows and copies; a line sent says so, and so does a message from RSC. The terminal's screen gives its written rows and the cursor's place. Each chart says, for each curve, its last value, then its minimum and maximum over the part shown (`temp: 22, from 20.75 to 22`), or that it is hidden, and whether it is paused. The state line is announced when it changes (connection, loss, reconnection, problem, journal). The lines received are not announced, not to drown the reader: they are read by going into the console.
- **Auto scroll**: the console follows its last lines. Going up (wheel, bar, minimap, Page Up, Home) stops following, and a line that comes no longer moves the view; going back to the very bottom (End) follows again. Turning it on again brings the console to the bottom.
- **Console**: the device's ANSI colours, long lines folded, a minimap at the right (a click or a drag there moves the console). Dragging selects, a double click takes a word, a triple click a line; Ctrl+A, Ctrl+C, right click. With the keyboard, once the console is reached: Page Up and Page Down scroll it by a page (one line kept), Home and End go to its first and last line. What is typed in the console goes to the send line.
- **Send line**, under its label **Line** (**Bytes** in hexadecimal), which stays while typing: Enter sends. **Hex input** reads the line as bytes (`48 65 6C`, `0x48,0x65`, `48656C`), with no line ending added; otherwise, the line ending of your choice: none, LF, CR, CR LF.
- **Curves**: the **Curves** switch opens, under the console, the curves of the measures the device writes.
  - A line made only of values becomes points, in the Arduino plotter's format: `21.5 40` (curves `1` and `2`), `temp=21.5 hum=40`, `temp: 21.5`. A line that holds an ordinary word, a log line for instance, makes none.
  - **The horizontal axis**, **Time | No.**: the time since the first measure, or the measure's number (1 for the first line of measures, 2 for the next; log lines do not count). The number spreads out the measures a device sends in bursts, all dated the same instant.
  - **The part shown**, up to the last measure: 10 s, 1 min, 5 min or all; along the numbers, 100, 1 000, 10 000 measures or all.
  - **Two charts**: **Above or below** puts each curve in the upper or the lower chart. Each has its own vertical scale; they share the horizontal axis, which only the lower one graduates. Useful when the orders of magnitude differ: `temp=21.5 pressure=1013.2`, `pressure` below.
  - **The vertical scale** covers all the values of the curves shown over the part chosen, with a 5 % margin. A curve hidden by a click on the legend no longer counts. Its marks go by steps of 1, 2 or 5 times a power of ten, at least 24 points apart: each chart shows at least two, at any height and over any range, in full colour.
  - **The names**: each curve has its name at the end of its line, right of its last value when there is room, above it otherwise; it is recognised without its colour. The legend, at the top, on the side the curves' ends leave free, hides or shows a curve at a click.
  - **Pause** stops adding; the wheel zooms and a drag moves, on the horizontal axis only and within the part chosen: the vertical scale does not move, nothing is cut. A double click goes back to the whole part. Without a mouse, four buttons beside **paused** do the same: **Earlier** and **Later** move the view by half its width, **Closer** and **Farther** narrow or widen it by half.
  - **Export as CSV** writes the measures kept for a spreadsheet set to French (semicolon, decimal comma).
  - The separation between the console and the curves can be dragged; reached by Tab, the up and down arrows move it by 5 %; a click on it, or Enter, offers three heights for the curves: a quarter, a half, three quarters.
  - **Display**, right of the actions when the curves are on: **Console** (or **Terminal**), **Curves** or **Both**. On a window too low for both (the console at 3 lines and a whole chart), **Both** is greyed and the middle shows the console, or the curves if chosen; **Both** comes back when the window grows again. When their charts do not hold in the room, the curves scroll.
- **Actions**, as icons whose tooltip says what they do: settings, copy (the selection, or the whole console), export the console to a file, start or stop the journal (a red mark while it runs), send a file (16 MB at most), clear.
- **Save**: the system's window offers the file's type first (**Log (*.log)** for the console and the journal, **CSV (*.csv)** for the curves, **Text (*.txt)** for the licence notices), then **All files**. A name typed without an extension takes the one of the type offered.

The hexadecimal view shows the bytes received as they are, line endings and ANSI sequences included; a long line folds without ever cutting a byte in two.

## The terminal mode

The **Terminal** switch of the state bar (or Settings, Console, **Terminal mode**) replaces the console by the device's screen, as a terminal draws it: cursor, erasing, scrolling regions, the alternate screen of vi and less, 16 and 256 colours, direct colours, frames. It is a setting of the profile: a profile can open straight in the terminal.

- **The keyboard goes to the device**, key by key, as xterm sends it: Enter gives CR, Backspace DEL; the arrows, Home, End, Insert, Delete, Page Up and Down, F1 to F12; Ctrl+letter the control character; Alt an ESC before. Tab, the arrows and Escape stay with the device. RSC does not echo what is typed: the device sends it back.
- **Leaving the screen with the keyboard**: Ctrl+Tab goes to the next control, Ctrl+Shift+Tab to the previous one, sending nothing. A serial device cannot tell Ctrl+Tab from Tab: it loses nothing. The line of shortcuts goes on two rows when the window is narrow.
- **Copy and paste, as in Windows Terminal**, recalled at all times in place of the send line:
  - Ctrl+C copies the selection if there is one, otherwise sends Ctrl+C (interrupt);
  - Ctrl+V pastes;
  - Ctrl+Shift+C and Ctrl+Shift+V always copy and paste;
  - Ctrl+X goes to the device (nano uses it to quit).

  Dragging selects, a double click takes a word, a triple click the line; the right click offers **Copy**, **Copy the whole screen**, **Clear the screen**. What is pasted goes with CR at the end of each line, and between ESC [ 200 ~ and ESC [ 201 ~ when the device asked for it.
- **The size follows the window**, and the bottom line says it (for instance `129×28`). A serial link cannot tell it to the device; under Linux, `resize` finds it, because RSC answers the device's questions (cursor position, size, identity). Otherwise, `stty rows 28 cols 129`.
- **The history**: the wheel goes up through the lines scrolled off the screen (the profile's **Lines kept**, 10 000 at most); typing brings it back to the bottom.
- **The line console goes on underneath**: the journal, the curves and the console's export work as before, and turning **Terminal** off finds everything again. Timestamps, Hex view and Auto scroll, which belong to it, are greyed.
- **The screen starts empty** when **Terminal** is turned on: a key (Enter, Ctrl+L) has the device draw it again.
- **Limits**:
  - the mouse is not sent to the device (the menus of htop or mc are led with the keyboard);
  - no bell, no window title;
  - ten characters of DEC's line drawing set have a glyph in none of the console's fonts: its four lines at different heights are drawn as the nearest line, and its control symbols as a dot. The lines and the corners themselves are the real ones.

The screen is the one of the `vt100` package. RSC adds DEC's line drawing set (`ESC ( 0`), which vt100 lacks: a filter before it turns the letters into lines and corners, without touching the inside of sequences.

## Profiles and settings

A window of sections, chosen at its left; everything applies at once and is kept. On a low window, the list of sections scrolls, and the section chosen is brought into view. Each label has its explanation in a tooltip, and so does each control. Its foot gives the version and the path of the settings file.

**The profile, at the top, always.** The **You are changing** selector says the profile that the sections Link to Journal change, and a sentence under it says it plainly:

- **General**: the choices by default, for the ports and the devices that have no profile of the library.
- **A profile of the library** (`ESP32 921600`): the sentence says who uses it (`used by Board 3 and 1 tab`). A profile has all its values: changing the general one does not change it.

The window opens on the current tab's profile. The profile's sections:

- **Link**: baud rate (a list, or a value typed), data bits (5 to 8), parity, stop bits, flow control (none, XON/XOFF, RTS/CTS), DTR and RTS on opening, line ending. Some boards (ESP32, Arduino) restart when DTR or RTS changes: their profile can turn them off.
- **Console**: timestamps, ANSI colours, hexadecimal view, minimap, terminal mode, echo of what is sent, lines kept (10 000 by default), text size.
- **Curves**: show them or not, a prefix (only the lines that begin with it make curves), the points kept per curve (100 000 by default), the horizontal axis (time or the measure's number), the curves of the lower chart (chosen among the curves; **All back above** brings them back up).
- **Sending**: hexadecimal input; read `\n`, `\r`, `\t`, `\0`, `\e`, `\\` and `\xNN` in the line (`\x03` sends a Ctrl+C).
- **Connection**: reopen a lost port as soon as it is back; a device that has a profile is found again even if it comes back on another port.
- **Journal**: a folder (otherwise, the file is asked each time), a start at each connection, the time of each line. The journal writes everything that comes, beyond the lines kept on screen.

The application's sections, the same whatever the profile:

- **Profiles**: the library.
  - At the top, the general one, with its link in plain words and **Change**.
  - **New profile**: a name, then **Make the profile**, a copy of the general one.
  - A card per profile: its link in plain words (`921600 8N1 · CR LF`), who uses it, and **Change** (chooses it at the top and opens the Link section), **Duplicate** (a copy, to rename at once), **Rename** (followed by the devices and the tabs), **Delete** (those that used it take the general profile; when it is in use, the deletion is confirmed in its card).
- **Devices**: a USB device you name, which RSC recognises by its maker, its product and its serial number when it has one, on any port.
  - It is named from the tab on its port, here or through the profile pill: it keeps the profile the tab used.
  - Each card says whether the device is plugged in, its USB identity, and gives its profile in a list (**General** or a profile of the library).
  - It can be renamed, or forgotten (its tabs stay on its last port, with the general profile).
- **Look**: the system's theme (followed when it changes), light or dark; French or English. Common to all profiles.
- **About**: RSC's version, where its source is, and the licences of the components it is made of (see "Dependencies and licences"), which **Find** filters.

The general profile by default: 115200 bauds, 8 bits, no parity, 1 stop bit, no flow control, DTR and RTS on, line ending LF, echo, colours, minimap, reconnection on. A named device is always followed by its USB identity: it is not a setting.

## The settings file

- **Where**: a readable JSON file, `%APPDATA%\rsc\data\settings.json` under Windows, `~/.local/share/rsc/settings.json` under Linux, `~/Library/Application Support/rsc/settings.json` under macOS, or the file of `RSC_SETTINGS=<path>`.
- **What it keeps**: only what differs from the values by default, for the general profile as for each profile of the library (`profiles`, each with its `values`); the named devices (`devices`); the tabs. The USB serial number of each named device, a long hexadecimal string, is in it whole: the device is recognised by it.
- **An unreadable file** is set aside (`settings.json.bad`), never overwritten. The file is written beside, then renamed: never cut.
- **Beside it**, `app.ron` is eframe's file: it keeps only the window's size and place.

The files of earlier versions are recognised by the keys only they wrote; a file with none of them is read in the current form, even written by hand without `general`. They are read as they are and written again at once, after being copied beside (`settings.json.old`, then `.old-2`…: nothing is overwritten); the console says so. Three earlier forms are taken in:

- **Device profiles** (`device_profiles`, and `device_profile` in a tab): a device and its profile at once, the profile keeping only what it changed of the general one. Each becomes a profile of the library under its name, with the values it had (the general one and what it changed), and a named device of the same name, given that profile. Each tab keeps exactly its values; but a profile taken in no longer follows the changes of the general profile.
- **The same form under an older name**: the device profiles, their list and each tab's were named otherwise. They are taken in the same way.
- **Presets** (`presets`, `local_echo`, `link` in a tab…): presets and a link per tab.
  - the link of the current tab, if it is on a bare port, and the former common settings become the general profile;
  - each device gets a profile of the library of its name: the general one, and what the link of its preset (otherwise of its tab) changes of it;
  - a preset that no device used is dropped;
  - in the very first files, a preset that had a USB device becomes a named device and a profile of the same name.

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

## Security

RSC opens no network connection, and its code holds no `unsafe` (`#![forbid(unsafe_code)]`). What a device sends is bounded:

- in line mode, the terminal sequences other than colours are removed;
- in terminal mode, the screen plays them, and RSC answers only questions of a fixed form (cursor position, state, identity, screen size): it never sends back a text it received. What is pasted cannot close the bracketed paste before its end;
- the lines, the curves and the history have a maximum size;
- a curve name written as a formula (`@SUM(1)`) leaves the CSV as text.

The last `cargo audit` (19/11/2026, 1 550 advisories) finds no known vulnerability, vt100 and vte included; it gives 5 warnings, for packages no longer maintained. A known defect of vte 0.15.0 (a character lost after a character cut between two reads) is worked around in RSC.

## Development

```bash
cargo clippy --all-targets      # no warning tolerated: lints in deny
cargo test                      # the whole suite
RSC_CAPTURES=1 cargo test captures                                    # the window, light and dark, in build/captures
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

- **Measures in Rust** write their CSV into `RSC_MEASURES`, otherwise into `build/measures`.
- **The accessibility audit** (`src/ui/audit/`, one file per criterion: contrast, names and targets, keyboard, 200 %, colour and drags, screen readers) measures the window against WCAG 2.2 AA in the test harness, without judging. The contrast of each pair of colours drawn, light and dark; the accessibility tree of each screen (role, name, size of the targets); Tab from end to end, the focused image compared with the same one without focus; the keys that could leave the terminal's screen; each screen at 200 %. Its tables and captures go into `RSC_AUDIT`, otherwise into `build/audit`. `looks` captures the screens at 100 %, light and dark, and the focus outline. Ordinary tests of the same files keep what it had corrected:
  - the focus outline, the order of Tab, leaving the terminal's screen, the separation with the keyboard, the settings that keep the keyboard, a name for each control, the pill's whole name, the contrast of each pair of colours;
  - at the size by default, the list of the settings' sections whole, without scrolling;
  - at the least size: the whole state, right of the tabs if need be, each section of the settings reached, the section chosen in view, the line of shortcuts and the cards of a profile and of a device whole, the curves and the **Display** choice, the window that holds on a Full HD screen at 200 % and on a 1366 × 768 laptop at 150 %, the first start maximised on a screen too low;
  - the vertical marks of each chart, over seven ranges of values, at both sizes, each written whole and apart from the others;
  - without colour or drag: a tab's state by its mark and in words, the name at the end of each curve, apart from the legend, the separation at a click, the pause's buttons; the visible labels of the send line and of the search;
  - for screen readers: the console's lines in view and its frame in the window, the rows of the terminal's screen and its cursor, the values of each chart, the state line announced, once per change; the console with the keyboard (Page Up, Page Down, Home, End).

  What the audit does not see: what a real screen reader says (Narrator, NVDA). `device-check.ps1 -Curves` reads the real window's tree through UI Automation, as Narrator does, and listens to Windows' announcements, saying nothing aloud.
- **The vocabulary**: `tests/vocabulary.rs` checks that the window's texts, in both languages, and this README call a device a device. It also reads, when the machine has one, a local list of words that must never be written, `tests/banned.local`, which git ignores: one word per line, `=word` for a whole word only, `#` for a comment.
- **A script edited from Windows** (on `\\wsl.localhost`) loses its right to be run, and `./build.sh` then answers "Permission denied". `tests/scripts.rs` sees it under Linux and macOS and gives the command that repairs it (`chmod +x <script>`).
- **`.\measure.ps1`** (Windows) measures the start and the rest of `build\rsc.exe`, with temporary settings (27 starts by default; `-Starts` and `-Rests` for fewer), and writes its CSV into `build\measures` or into the folder of `-Out`. `-Tabs 4` opens RSC on 4 tabs without a port; `-Terminal` puts them in terminal mode; `-Exe` measures another executable, an earlier version for instance.
- **`.\window-check.ps1 -Port COM3`** (Windows) goes through the real window of `build\rsc.exe` as a person would, through Windows accessibility, without moving the mouse: the least size, the Profiles and Devices sections, a profile of the library chosen through the pill on an unnamed USB port, the device named, About, a licence, the window that saves the notices (its type checked as `*.txt`, then cancelled), and the terminal's line of shortcuts. It captures the window at each stage in `build\captures\vraie-<theme>-*.png`, except the save window, which shows your folders. Temporary settings; the port is listed, never opened; `-Theme dark` for a single theme.
- **The test program for Arduino** (`tests/arduino/rsc_essai/rsc_essai.ino`, written for a Leonardo): a small console `essai>` that handles its own echo, Backspace, the up arrow and Ctrl+C. Its commands exercise RSC on a real device:
  - `aide` and `version`: the console and line editing;
  - `taille`: the device asks the screen's size (`ESC [ 6 n`, `ESC [ 18 t`) and shows RSC's answers; in line mode, it says no terminal answered;
  - `menu`: a menu on the alternate screen, in a frame of DEC's set, the chosen line in reverse, led with the arrows, Enter and Escape;
  - `couleurs`: the 16 colours, the cube of 256, direct colours; `journal`: coloured lines like ESP-IDF's;
  - `mesures`: `temp=21.50 pression=1013.2 compteur=42` every 100 ms, until Ctrl+C, Enter or `q`;
  - `flux`: 3 s of numbered lines as fast as USB takes them, then their count and the throughput.
- **`tests\arduino\flash.ps1 -Port COM3`** (Windows) builds and flashes this program with arduino-cli, kept with its `arduino:avr` core in `%LOCALAPPDATA%\rsc-arduino` (about 460 MB; `-Tools` for another folder). `-Keep <file.hex>` first reads the program the board holds, through its bootloader, to put it back later. It refuses any port that does not carry an Arduino (USB vendor 2341).
- **`.\device-check.ps1 -Port COM3`** (Windows): the real window of `build\rsc.exe` on an Arduino running the test program, with Windows' real keyboard. Temporary settings (one tab on this port, in terminal mode); it clicks **Connect** through accessibility, brings the window to the front, then types `taille`, `menu`, down arrow, Enter, `couleurs`, `mesures` and Ctrl+C. Before each key, it checks RSC's window is in front, and stops otherwise. Type nothing during its 20 s or so. Captures in `build\captures\appareil-*.png`. With `-Curves`, the curves on and `pression` in the lower chart, then what a screen reader receives, read through UI Automation: the terminal screen's rows, the state line, the console's lines; last, it closes the port while listening to Windows' announcements.
- **The icons of Linux and macOS**: `python3 assets/make_platform_icons.py` draws them from `rsc.ico`, with Python alone.

## The files

| File | Role |
|---|---|
| `Cargo.toml`, `Cargo.lock` | the package, its own root; versions pinned, lints in deny, LTO in release |
| `LICENSE` | RSC's licence: the Eclipse Public License 2.0 |
| `build.rs` | under Windows, the icon and the properties of `rsc.exe` |
| `build.ps1`, `build.sh` | release build and copy into `build/`; under Windows and Linux, install and uninstall; under macOS, `RSC.app`; Linux x86 and arm64 for glibc 2.31 (`--linux-x86`, `--linux-arm64`) |
| `measure.ps1` | under Windows, RSC's start and rest, on one or several tabs, as CSV |
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
| `tests/arduino.rs`, `tests/arduino/` | an Arduino named by `RSC_ARDUINO`, run by hand: the link's thread, the terminal's screen and RSC's console on the test program; `rsc_essai/` is that program, `flash.ps1` flashes it |
| `tests/measures.rs` | the measures of the console, the curves and the list of the ports, with an allocator that counts memory, run by hand |
| `tests/assets.rs` | the form of each system's icons and of the `.desktop` file |
| `tests/licences.rs` | the licence record: making it, and checking it follows the machine's dependencies |
| `tests/scripts.rs` | under Linux and macOS, each script run straight (`*.sh`, or a first line `#!/`) keeps its right to be run |
| `tests/vocabulary.rs` | the window's texts (French and English) and the README: a generic tool, which says "device" and names no company |
| `assets/fonts/` | the icons' font (Lucide 1.74.0), its licence and where it comes from |
| `assets/licences/` | the licence record (`licences.json`, which the program carries), its text (`THIRD-PARTY-LICENSES.txt`), the standard texts and where they come from (`textes/`) |
| `assets/make_icons.py` | the logo, with Pillow: `rsc.svg`, `rsc-256.png` (the window's icon), `rsc.ico` (the executable's icon) |
| `assets/make_platform_icons.py` | from `rsc.ico`: `linux/rsc-<size>.png` and `rsc.icns` (macOS) |
| `assets/rsc.desktop` | RSC's entry in Linux menus, which `build.sh --install` puts in place |

## The logo

A serial connector drawn freehand, a little askew, with a signal over it.

- `assets/make_icons.py` (Pillow) makes it in three styles from a single geometry: "hand" (a filled connector in a gradient), "neon" (a glowing outline on a night background) and "flat" (a white line on a gradient tile), RSC's.
- `python make_icons.py` writes the style of `STYLE`; `python make_icons.py --preview <folder>` writes all three to compare them.

## Licence

RSC © 2026 Jean-François Erdelyi, under the terms of the Eclipse Public License 2.0 (`LICENSE`, SPDX `EPL-2.0`), with no Secondary License.

- **What it allows**: using RSC, copying it, changing it and handing it out.
- **What it asks of whoever hands it out**:
  - a changed version keeps its changed files open, under the EPL; the rest combines freely with other code;
  - the executable alone goes with its licence and says how to get the source code.
- **RSC's source code** is at https://github.com/jferdelyi/rsc. RSC's own notice and its About section say so.
- **The notice** is here, in `LICENSE` and in About, not at the head of each source file: the EPL allows it when the notice is where one looks for it.
- **The text of `LICENSE`** is the EPL-2.0 that the packages uhlc 0.8.2 and ringbuffer-spsc 0.1.15 carry (the same but for blanks), taken from cargo's cache. A test checks it is whole and that `Cargo.toml` says the same licence.

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
