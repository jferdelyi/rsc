# RSC user guide

Everything RSC's window does, part by part. To get RSC and start, see the [README](../README.md).

- [The window](#the-window)
- [The terminal mode](#the-terminal-mode)
- [Profiles and settings](#profiles-and-settings)
- [The settings file](#the-settings-file)
- [Security](#security)

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
- **About**: RSC's version, where its source is, and the licences of the components it is made of (see [Dependencies and licences](development.md#dependencies-and-licences)), which **Find** filters.

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

## Security

RSC opens no network connection, and its code holds no `unsafe` (`#![forbid(unsafe_code)]`). What a device sends is bounded:

- in line mode, the terminal sequences other than colours are removed;
- in terminal mode, the screen plays them, and RSC answers only questions of a fixed form (cursor position, state, identity, screen size): it never sends back a text it received. What is pasted cannot close the bracketed paste before its end;
- the lines, the curves and the history have a maximum size;
- a curve name written as a formula (`@SUM(1)`) leaves the CSV as text.

The last `cargo audit` (19/11/2026, 1 550 advisories) finds no known vulnerability, vt100 and vte included; it gives 5 warnings, for packages no longer maintained. A known defect of vte 0.15.0 (a character lost after a character cut between two reads) is worked around in RSC.
