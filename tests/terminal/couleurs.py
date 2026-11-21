"""A screen drawn by curses, for tests/terminal/record.sh: a border of lines and corners, colours
of the 256-colour cube in the background, an underlined colour, inverse and bold text."""

import curses


def main(screen):
    curses.start_color()
    curses.use_default_colors()
    screen.border()
    screen.addstr(1, 2, "Couleurs", curses.A_BOLD)
    for n in range(48):
        colour = 16 + n * 4
        curses.init_pair(n + 1, -1, colour)
        screen.addstr(3 + n // 16, 2 + (n % 16) * 4, f"{colour:3d}", curses.color_pair(n + 1))
    curses.init_pair(100, 208, -1)
    screen.addstr(8, 2, "orange 208", curses.color_pair(100) | curses.A_UNDERLINE)
    screen.addstr(9, 2, "inverse", curses.A_REVERSE)
    screen.refresh()
    screen.getch()


curses.wrapper(main)
