#!/usr/bin/env bash
# The screens of real programs as tmux draws them, for tests/terminal.rs. For each program:
#   <name>.out      what it wrote, byte for byte
#   <name>.screen   the screen tmux made of it, as text
#   <name>.colors   the same with its colours, as tmux writes them out
#   <name>.state    where the cursor is (column, row) and whether the alternate screen is on
# Under Linux, with tmux, on a server of its own: the user's tmux is not touched. Nothing of the
# user's goes in: no rc file, a home of its own in a temporary folder, the prompt "$ ".
#
#   tests/terminal/record.sh              every program
#   tests/terminal/record.sh vi less      some of them
set -euo pipefail

here=$(cd "$(dirname "$0")" && pwd)
work=$(mktemp -d)
server=(-L rsc-record -f /dev/null)
trap 'tmux "${server[@]}" kill-server 2>/dev/null || true; rm -rf "$work"' EXIT

# A shell in an 80 by 24 pane, its output recorded from its very first byte.
start() {
    local name=$1
    rm -f "$here/$name.out"
    tmux "${server[@]}" new-session -d -s "$name" -x 80 -y 24 \
        "sleep 1; cd '$work' && exec env -i PATH=/usr/bin:/bin HOME='$work' TERM=xterm-256color LANG=C.UTF-8 PS1='\$ ' bash --norc --noprofile -i"
    tmux "${server[@]}" pipe-pane -t "$name" "cat > '$here/$name.out'"
    sleep 2
}

# Keys by their tmux name (Enter, Up, Escape), or text as it is.
keys() { tmux "${server[@]}" send-keys -t "$1" "${@:2}"; sleep 0.6; }
text() { tmux "${server[@]}" send-keys -t "$1" -l "$2"; sleep 0.6; }

finish() {
    local name=$1
    sleep 1
    tmux "${server[@]}" capture-pane -p -t "$name" > "$here/$name.screen"
    # -N keeps the blanks at the end of each row: they may be in inverse or on a colour.
    tmux "${server[@]}" capture-pane -p -e -N -t "$name" > "$here/$name.colors"
    tmux "${server[@]}" display-message -p -t "$name" '#{cursor_x} #{cursor_y} #{alternate_on}' > "$here/$name.state"
    tmux "${server[@]}" pipe-pane -t "$name"
    tmux "${server[@]}" kill-session -t "$name"
    echo "$name: $(wc -c < "$here/$name.out") bytes"
}

# The shell's line editing and history, colours, the line-drawing set, a completion.
record_shell() {
    printf 'contenu de alpha\n' > "$work/alpha.txt"
    start shell
    text shell "echo ac"; keys shell Left; text shell "b"; keys shell Enter
    keys shell Up; keys shell Enter
    text shell "printf '\\033[1;31mrouge\\033[0m \\033[38;5;208morange\\033[0m \\033[48;2;0;90;160mfond\\033[0m \\033[4msous\\033[0m \\033[7minverse\\033[0m\\n'"; keys shell Enter
    text shell "printf '\\033(0lqqqk\\033(B cadre\\n\\033(0x   x\\033(B\\n\\033(0mqqqj\\033(B\\n'"; keys shell Enter
    text shell "cat alp"; keys shell Tab; keys shell Enter
    finish shell
}

# vi on the alternate screen, with line numbers.
record_vi() {
    start vi
    text vi "vi -u NONE -i NONE -n notes.txt"; keys vi Enter
    text vi "iPremière ligne"; keys vi Enter; text vi "deuxième ligne"; keys vi Escape
    sleep 1.2
    text vi ":set number"; keys vi Enter
    finish vi
}

# less, a page down.
record_less() {
    seq -f 'ligne %g de cent' 1 100 > "$work/cent.txt"
    start less
    text less "less cent.txt"; keys less Enter; keys less Space
    finish less
}

# nano, its bars in inverse.
record_nano() {
    start nano
    text nano "nano -I notes.txt"; keys nano Enter; text nano "Bonjour nano"
    finish nano
}

# A whiptail menu: a framed box, the line chosen moved down.
record_menu() {
    start menu
    text menu "whiptail --title Liaison --menu 'Choisir un débit' 14 50 4 un '115200 bauds' deux '9600 bauds' trois '921600 bauds'"; keys menu Enter
    keys menu Down
    finish menu
}

# curses in 256 colours, a border of lines and corners.
record_couleurs() {
    cp "$here/couleurs.py" "$work/"
    start couleurs
    text couleurs "python3 couleurs.py"; keys couleurs Enter
    finish couleurs
}

names=("$@")
[ ${#names[@]} -eq 0 ] && names=(shell vi less nano menu couleurs)
for name in "${names[@]}"; do
    "record_$name"
done
