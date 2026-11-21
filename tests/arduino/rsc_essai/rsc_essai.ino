// RSC's test program, for an Arduino Leonardo (any board with a serial port will do): a small
// console that exercises what RSC shows and sends, in its line mode and in its terminal mode.
//
// At the prompt "essai> ": aide, version, taille, menu, couleurs, journal, mesures, flux. What is
// typed is echoed; Backspace (DEL or BS) takes the last character back; the up arrow brings the last
// command back; Ctrl+C stops a running command, or forgets the line. A line ends with CR or LF.
//
// Flashed by tests/arduino/flash.ps1. The texts are kept in flash (F()): the Leonardo has 2.5 KB of
// memory.

const byte LONGEST = 48;
char line[LONGEST + 1];
byte length = 0;
char last[LONGEST + 1];
// A line ended by CR: the LF that may follow belongs to it.
bool after_cr = false;

// The keys read, beyond plain characters.
enum Key { NONE = -1, UP = -2, DOWN = -3, LEFT = -4, RIGHT = -5, ENTER = -6, ESCAPE = -7, INTERRUPT = -8, ERASE = -9 };

// A key typed, waited for up to `wait` milliseconds: a character, or one of Key. The arrows come as
// ESC [ A or ESC O A; Escape alone is an ESC that nothing follows within 50 ms.
int read_key(unsigned long wait) {
    unsigned long start = millis();
    while (!Serial.available()) {
        if (millis() - start >= wait) return NONE;
    }
    int c = Serial.read();
    if (c == 0x1b) {
        unsigned long since = millis();
        while (!Serial.available() && millis() - since < 50) {}
        if (!Serial.available()) return ESCAPE;
        int next = Serial.read();
        if (next != '[' && next != 'O') return ESCAPE;
        since = millis();
        while (!Serial.available() && millis() - since < 50) {}
        switch (Serial.read()) {
            case 'A': return UP;
            case 'B': return DOWN;
            case 'C': return RIGHT;
            case 'D': return LEFT;
            default: return NONE;
        }
    }
    if (c == '\n' && after_cr) {
        after_cr = false;
        return NONE;
    }
    after_cr = c == '\r';
    if (c == '\r' || c == '\n') return ENTER;
    if (c == 0x03) return INTERRUPT;
    if (c == 0x7f || c == 0x08) return ERASE;
    return c;
}

// Whether the user asks a running command to stop: Ctrl+C, Enter or q.
bool stop_asked() {
    while (Serial.available()) {
        int key = read_key(0);
        if (key == INTERRUPT || key == ENTER || key == 'q') return true;
    }
    return false;
}

void prompt() {
    Serial.print(F("essai> "));
}

void help() {
    Serial.print(F(
        "Commandes :\r\n"
        "  aide       ces lignes\r\n"
        "  version    le programme et sa date\r\n"
        "  taille     la taille de l'\xc3\xa9" "cran, demand\xc3\xa9" "e au terminal\r\n"
        "  menu       un menu plein \xc3\xa9" "cran, aux fl\xc3\xa8" "ches\r\n"
        "  couleurs   16 couleurs, le cube des 256, des couleurs directes\r\n"
        "  journal    des lignes color\xc3\xa9" "es, comme ESP-IDF\r\n"
        "  mesures    des mesures toutes les 100 ms, jusqu'\xc3\xa0" " Ctrl+C\r\n"
        "  flux       3 s de lignes au plus vite, puis le d\xc3\xa9" "bit\r\n"));
}

// A report of the terminal, ESC [ numbers ; ... final, waited for 500 ms: its numbers, the last two
// kept. False when nothing came: no terminal answers, a line console say.
bool read_report(char final, int *first, int *second) {
    unsigned long start = millis();
    int numbers[3] = {0, 0, 0};
    byte count = 0;
    byte state = 0;
    while (millis() - start < 500) {
        if (!Serial.available()) continue;
        int c = Serial.read();
        if (state == 0) {
            if (c == 0x1b) state = 1;
        } else if (state == 1) {
            state = c == '[' ? 2 : 0;
        } else if (c >= '0' && c <= '9') {
            if (count == 0) count = 1;
            numbers[count - 1] = numbers[count - 1] * 10 + (c - '0');
        } else if (c == ';') {
            if (count < 3) count++;
        } else if (c == final) {
            *first = count >= 2 ? numbers[count - 2] : 0;
            *second = numbers[count - 1];
            return true;
        } else {
            return false;
        }
    }
    return false;
}

void size() {
    int rows, cols;
    // The cursor kept, sent as far as it goes, asked for, put back: how `resize` learns the size.
    Serial.print(F("\x1b" "7" "\x1b[999;999H" "\x1b[6n"));
    bool far = read_report('R', &rows, &cols);
    Serial.print(F("\x1b" "8"));
    if (far) {
        Serial.print(F("Curseur au plus loin : ligne "));
        Serial.print(rows);
        Serial.print(F(", colonne "));
        Serial.print(cols);
        Serial.print(F("\r\n"));
    }
    Serial.print(F("\x1b[18t"));
    if (read_report('t', &rows, &cols)) {
        Serial.print(F("Taille annonc\xc3\xa9" "e : "));
        Serial.print(cols);
        Serial.print(F("x"));
        Serial.print(rows);
        Serial.print(F("\r\n"));
    } else if (!far) {
        Serial.print(F("Aucun terminal n'a r\xc3\xa9" "pondu (une console en mode ligne ?)\r\n"));
    }
}

const char ITEM_0[] PROGMEM = "Lire les capteurs";
const char ITEM_1[] PROGMEM = "R\xc3\xa9" "gler l'heure";
const char ITEM_2[] PROGMEM = "Red\xc3\xa9" "marrer";
const char ITEM_3[] PROGMEM = "Quitter";
const char *const ITEMS[] PROGMEM = {ITEM_0, ITEM_1, ITEM_2, ITEM_3};
const byte COUNT = 4;
const byte TOP = 3, LEFT_COL = 10, WIDTH = 40, HEIGHT = 9;

void move_to(byte row, byte col) {
    Serial.print(F("\x1b["));
    Serial.print(row);
    Serial.print(';');
    Serial.print(col);
    Serial.print('H');
}

void print_item(byte index, bool chosen) {
    char text[24];
    strcpy_P(text, (const char *)pgm_read_ptr(&ITEMS[index]));
    move_to(TOP + 2 + index, LEFT_COL + 3);
    Serial.print(chosen ? F("\x1b[7m ") : F("\x1b[0m "));
    Serial.print(text);
    Serial.print(F(" \x1b[0m"));
}

// A menu on the alternate screen, in a frame of the line-drawing set, led by the arrows: Enter
// chooses, Escape or q leaves.
void menu() {
    Serial.print(F("\x1b[?1049h\x1b[?25l\x1b[2J"));
    // The frame: corners and lines of the line-drawing set (ESC ( 0), l k m j q x.
    move_to(TOP, LEFT_COL);
    Serial.print(F("\x1b(0l"));
    for (byte i = 0; i < WIDTH - 2; i++) Serial.print('q');
    Serial.print(F("k"));
    for (byte row = 1; row < HEIGHT - 1; row++) {
        move_to(TOP + row, LEFT_COL);
        Serial.print('x');
        move_to(TOP + row, LEFT_COL + WIDTH - 1);
        Serial.print('x');
    }
    move_to(TOP + HEIGHT - 1, LEFT_COL);
    Serial.print('m');
    for (byte i = 0; i < WIDTH - 2; i++) Serial.print('q');
    Serial.print(F("j\x1b(B"));
    move_to(TOP, LEFT_COL + 3);
    Serial.print(F("\x1b[1;36m Menu d'essai \x1b[0m"));
    move_to(TOP + HEIGHT + 1, LEFT_COL);
    Serial.print(F("\x1b[2mFl\xc3\xa8" "ches : choisir \xc2\xb7 Entr\xc3\xa9" "e : valider \xc2\xb7 \xc3\x89" "chap : quitter\x1b[0m"));
    byte chosen = 0;
    for (byte i = 0; i < COUNT; i++) print_item(i, i == chosen);
    int result = -1;
    while (true) {
        int key = read_key(60000UL);
        if (key == UP || key == DOWN) {
            print_item(chosen, false);
            chosen = key == UP ? (chosen + COUNT - 1) % COUNT : (chosen + 1) % COUNT;
            print_item(chosen, true);
        } else if (key == ENTER) {
            result = chosen;
            break;
        } else if (key == ESCAPE || key == 'q' || key == INTERRUPT || key == NONE) {
            break;
        }
    }
    Serial.print(F("\x1b[?25h\x1b[?1049l"));
    if (result < 0) {
        Serial.print(F("Menu quitt\xc3\xa9\r\n"));
    } else {
        char text[24];
        strcpy_P(text, (const char *)pgm_read_ptr(&ITEMS[result]));
        Serial.print(F("Choisi : "));
        Serial.print(text);
        Serial.print(F("\r\n"));
    }
}

void colours() {
    for (byte i = 0; i < 16; i++) {
        Serial.print(F("\x1b[48;5;"));
        Serial.print(i);
        Serial.print(F("m  "));
    }
    Serial.print(F("\x1b[0m  16 couleurs\r\n"));
    for (byte row = 0; row < 6; row++) {
        for (byte col = 0; col < 36; col++) {
            Serial.print(F("\x1b[48;5;"));
            Serial.print(16 + row * 36 + col);
            Serial.print(F("m "));
        }
        Serial.print(F("\x1b[0m\r\n"));
    }
    for (byte i = 0; i < 36; i++) {
        Serial.print(F("\x1b[48;2;"));
        Serial.print(i * 7);
        Serial.print(';');
        Serial.print(64);
        Serial.print(';');
        Serial.print(255 - i * 7);
        Serial.print(F("m "));
    }
    Serial.print(F("\x1b[0m  couleurs directes\r\n"));
}

void journal() {
    for (byte i = 0; i < 20; i++) {
        unsigned long now = millis();
        if (i % 7 == 6) {
            Serial.print(F("\x1b[0;31mE ("));
        } else if (i % 4 == 3) {
            Serial.print(F("\x1b[0;33mW ("));
        } else {
            Serial.print(F("\x1b[0;32mI ("));
        }
        Serial.print(now);
        Serial.print(F(") essai: ligne "));
        Serial.print(i);
        Serial.print(F(" du journal\x1b[0m\r\n"));
    }
}

// Measures every 100 ms until asked to stop: two scales far apart, and a count.
void measures() {
    unsigned long count = 0;
    while (!stop_asked()) {
        float t = count / 10.0;
        Serial.print(F("temp="));
        Serial.print(21.0 + sin(t), 2);
        Serial.print(F(" pression="));
        Serial.print(1013.0 + 5.0 * cos(t / 3.0), 1);
        Serial.print(F(" compteur="));
        Serial.print(count);
        Serial.print(F("\r\n"));
        count++;
        unsigned long wait = millis();
        while (millis() - wait < 100) {
            if (Serial.available()) break;
        }
    }
    Serial.print(F("Mesures arr\xc3\xaat\xc3\xa9" "es apr\xc3\xa8s "));
    Serial.print(count);
    Serial.print(F(" lignes\r\n"));
}

// Lines as fast as the USB link takes them for 3 s, then how many.
void flow() {
    unsigned long start = millis(), lines = 0, bytes = 0;
    while (millis() - start < 3000UL && !stop_asked()) {
        Serial.print(F("flux "));
        Serial.print(lines);
        Serial.print(F(" abcdefghijklmnopqrstuvwxyz 0123456789\r\n"));
        bytes += 46 + (lines > 9 ? 1 : 0) + (lines > 99 ? 1 : 0) + (lines > 999 ? 1 : 0) + (lines > 9999 ? 1 : 0);
        lines++;
    }
    unsigned long took = millis() - start;
    Serial.print(F("flux : "));
    Serial.print(lines);
    Serial.print(F(" lignes, environ "));
    Serial.print(bytes);
    Serial.print(F(" octets en "));
    Serial.print(took);
    Serial.print(F(" ms, soit "));
    Serial.print(bytes * 1000UL / (took ? took : 1));
    Serial.print(F(" octets/s\r\n"));
}

void run(const char *command) {
    if (!strcmp_P(command, PSTR("aide"))) help();
    else if (!strcmp_P(command, PSTR("version"))) {
        Serial.print(F("rsc_essai 1, compil\xc3\xa9 le " __DATE__ " " __TIME__ "\r\n"));
    } else if (!strcmp_P(command, PSTR("taille"))) size();
    else if (!strcmp_P(command, PSTR("menu"))) menu();
    else if (!strcmp_P(command, PSTR("couleurs"))) colours();
    else if (!strcmp_P(command, PSTR("journal"))) journal();
    else if (!strcmp_P(command, PSTR("mesures"))) measures();
    else if (!strcmp_P(command, PSTR("flux"))) flow();
    else if (command[0]) {
        Serial.print(F("Commande inconnue : "));
        Serial.print(command);
        Serial.print(F(" (aide)\r\n"));
    }
}

void setup() {
    Serial.begin(115200);
    // A Leonardo's port is open once the host sets DTR.
    while (!Serial) {}
    Serial.print(F("\r\nrsc_essai : la console d'essai de RSC. Tapez aide.\r\n"));
    prompt();
}

void loop() {
    int key = read_key(1000);
    if (key == NONE) return;
    if (key == ENTER) {
        Serial.print(F("\r\n"));
        line[length] = 0;
        if (length) strcpy(last, line);
        run(line);
        length = 0;
        prompt();
    } else if (key == ERASE) {
        if (length) {
            length--;
            Serial.print(F("\b \b"));
        }
    } else if (key == INTERRUPT) {
        Serial.print(F("^C\r\n"));
        length = 0;
        prompt();
    } else if (key == UP) {
        // The line typed so far erased, the last command in its place.
        while (length) {
            length--;
            Serial.print(F("\b \b"));
        }
        strcpy(line, last);
        length = strlen(line);
        Serial.print(line);
    } else if (key >= 0x20 && key < 0x100 && length < LONGEST) {
        line[length++] = key;
        Serial.write(key);
    }
}
