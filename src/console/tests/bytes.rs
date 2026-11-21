//! The bytes kept: the text copied is what was received, each line keeps the bytes it came
//! from, and shows them in hexadecimal.

use super::*;

#[test]
fn the_text_copied_is_what_was_received() {
    let mut console = Console::new(100);
    console.receive(b"one\r\ntwo\r\n", at(0, 0));
    console.receive(b"three", at(1, 500));
    assert_eq!(console.text(TEXT), "one\ntwo\nthree\n");
    assert_eq!(console.text(TIMED), "10:15:00.000 one\n10:15:00.000 two\n10:15:01.500 three\n");
}

#[test]
fn each_line_keeps_the_bytes_it_came_from_line_end_included() {
    let console = received(&[b"\x1b[32mok\x1b[0m\r", b"\nbad \xff\nlast"]);
    let raws: Vec<&[u8]> = console.lines().iter().map(Line::raw).collect();
    assert_eq!(raws, [&b"\x1b[32mok\x1b[0m\r\n"[..], b"bad \xff\n", b"last"]);
    let bytes: Vec<u8> = raws.concat();
    assert_eq!(bytes, b"\x1b[32mok\x1b[0m\r\nbad \xff\nlast", "nothing is lost, nothing is added");
}

#[test]
fn the_bytes_of_a_line_cut_anywhere_are_all_kept() {
    let bytes = "été\x1b[1m!\r\n\r\nfin\n".as_bytes();
    for cut in 1..bytes.len() {
        let console = received(&[&bytes[..cut], &bytes[cut..]]);
        let kept: Vec<u8> = console.lines().iter().flat_map(|line| line.raw().to_vec()).collect();
        assert_eq!(kept, bytes, "cut at {cut}");
    }
}

#[test]
fn in_hexadecimal_a_line_shows_its_bytes() {
    let mut console = received(&[b"Hi\r\n"]);
    console.sent("AT", b"AT\r", at(1, 0));
    console.notice("connecté", at(2, 0));
    let shown: Vec<String> = console.lines().iter().map(|line| line.display(HEX)).collect();
    assert_eq!(shown, ["48 69 0D 0A", "> 41 54 0D", "# connecté"], "a notice stays text");
    assert_eq!(console.lines()[0].width(HEX), 11);
    assert_eq!(console.lines()[0].display(Shown { timestamps: true, hex: true }), "10:15:00.000 48 69 0D 0A");
    assert_eq!(hex(&[]), "");
}
