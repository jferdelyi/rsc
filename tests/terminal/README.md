# Real programs on the terminal's screen

These files check that RSC's terminal mode draws a screen the way a real terminal does. The reference is tmux: six real programs were run in an 80 × 24 tmux pane, and what they wrote and what tmux showed were kept. [`../terminal.rs`](../terminal.rs) replays what each program wrote into RSC's screen and compares the result with tmux's, cell by cell.

## The programs

| Name | What it does on screen |
|---|---|
| `shell` | bash: line editing, the history, colours, DEC's line-drawing set, a completion |
| `vi` | vi on the alternate screen, with line numbers |
| `less` | less on a long file, one page down |
| `nano` | nano, with text typed |
| `menu` | a whiptail menu, its choice moved with the down arrow |
| `couleurs` | a curses screen in 256 colours, drawn by [`couleurs.py`](couleurs.py) |

## Four files for each program

| File | What it holds | In `vi`'s files |
|---|---|---|
| `<name>.out` | the bytes the program wrote, escape sequences included, exactly | the command typed, `vi -u NONE -i NONE -n notes.txt`, then `ESC [ ? 1049 h` (vi takes the alternate screen), then the text and its colours |
| `<name>.screen` | the screen tmux made of them, as plain text | `1 Première ligne`, `2 deuxième ligne`, then rows of `~` |
| `<name>.colors` | the same screen with its colours, as tmux writes them out | `ESC [ 38;5;130 m` before each line number (colour 130 of 256, a brown), `ESC [ 94 m` before the `~` (bright blue) |
| `<name>.state` | the cursor's column and row, counted from 0, and 1 when the alternate screen is on | `21 1 1`: on the last letter of `deuxième ligne`, on the alternate screen |

`terminal.rs` checks three things:
- each `.out`, replayed into RSC's screen 1, 7 and then 4096 bytes at a time as a serial port would deliver it, gives the characters and colours of `.colors`, and the cursor and screen of `.state`;
- `.colors` replayed gives the text of `.screen`, so the two files agree;
- a screen spoilt on purpose (a character, a colour, the cursor) is found different, so the comparison cannot pass for nothing.

## Recording them again

[`record.sh`](record.sh) runs the six programs again and rewrites the 24 files. It runs under Linux only, with tmux, on a tmux server of its own, in a temporary home folder with no rc file and the prompt `$ `. Linux only, on purpose: the files are what Linux's bash, vi, less, nano and whiptail write, and a Mac's versions would write other bytes. From Windows, run it in WSL.

```bash
tests/terminal/record.sh              # every program
tests/terminal/record.sh vi less      # some of them
```

The whole folder is marked `-text` in [`.gitattributes`](../../.gitattributes): git keeps its bytes exactly, the `\r\n` that programs write included, on every system.
