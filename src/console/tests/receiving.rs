//! Bytes received, made lines: lines cut between reads, line ends, characters, tabs, ANSI
//! colours and other escape sequences, the lines kept, a line too long, times, lines sent and
//! notices, clearing.

use super::*;

fn texts(console: &Console) -> Vec<&str> {
    console.lines().iter().map(Line::text).collect()
}

#[test]
fn a_line_cut_between_two_reads_is_one_line() {
    let console = received(&[b"boot: rea", b"dy\nsecond", b" line\n"]);
    assert_eq!(texts(&console), ["boot: ready", "second line"]);
    assert!(console.open_line().is_none());
}

#[test]
fn a_line_without_its_end_is_shown_as_it_grows() {
    let mut console = received(&[b"login: "]);
    assert_eq!(texts(&console), ["login: "]);
    assert_eq!(console.open_line().map(Line::text), Some("login: "));
    console.receive(b"root\n", at(1, 0));
    assert_eq!(texts(&console), ["login: root"]);
}

#[test]
fn cr_lf_cr_and_lf_each_end_one_line() {
    assert_eq!(texts(&received(&[b"a\r\nb\rc\nd"])), ["a", "b", "c", "d"]);
    assert_eq!(texts(&received(&[b"a\r", b"\nb\n"])), ["a", "b"], "CR LF cut between two reads");
    assert_eq!(texts(&received(&[b"a\n\nb\r\n\r\nc\n"])), ["a", "", "b", "", "c"], "empty lines are kept");
    assert_eq!(texts(&received(&[b"a\x1b[0m\r\x1b[0m\nb\n"])), ["a", "b"], "a colour reset between CR and LF");
    assert_eq!(texts(&received(&[b"a\r\r\nb\r\r\n"])), ["a", "b"], "a line end converted twice");
}

#[test]
fn a_character_cut_between_two_reads_is_put_back_together() {
    let bytes = "température 21 °C\n".as_bytes();
    for cut in 1..bytes.len() {
        let console = received(&[&bytes[..cut], &bytes[cut..]]);
        assert_eq!(texts(&console), ["température 21 °C"], "cut at {cut}");
    }
}

#[test]
fn bytes_that_are_not_text_show_as_the_replacement_character() {
    assert_eq!(texts(&received(&[b"ok \xff\xfe fin\n"])), ["ok \u{fffd}\u{fffd} fin"]);
}

#[test]
fn tabs_line_up_and_other_control_characters_are_dropped() {
    assert_eq!(texts(&received(&[b"ab\tc\x07\x00d\n"])), ["ab      cd"]);
    assert_eq!(texts(&received(&[b"abc\x08\x08d\n"])), ["ad"], "a backspace takes the character back");
}

#[test]
fn ansi_colours_are_kept_as_spans_and_leave_the_text() {
    let console = received(&[b"\x1b[0;32mI (312) boot: ok\x1b[0m\n\x1b[1;31mE fail\x1b[0m plain\n"]);
    assert_eq!(texts(&console), ["I (312) boot: ok", "E fail plain"]);
    assert_eq!(console.lines()[0].spans(), [Span { start: 0, color: Color::Ansi(2) }]);
    assert_eq!(
        console.lines()[1].spans(),
        [Span { start: 0, color: Color::Ansi(9) }, Span { start: 6, color: Color::Default }],
        "bold red is bright red"
    );
}

#[test]
fn a_colour_holds_until_it_is_reset_even_across_lines() {
    let console = received(&[b"\x1b[33mwarn one\nwarn two\x1b[39m\nplain\n"]);
    assert_eq!(console.lines()[1].spans(), [Span { start: 0, color: Color::Ansi(3) }]);
    assert!(console.lines()[2].spans().is_empty());
}

#[test]
fn an_escape_sequence_cut_between_two_reads_still_counts() {
    let bytes = b"a\x1b[0;32mb\x1b[0mc\n";
    for cut in 1..bytes.len() {
        let console = received(&[&bytes[..cut], &bytes[cut..]]);
        assert_eq!(texts(&console), ["abc"], "cut at {cut}");
        assert_eq!(
            console.lines()[0].spans(),
            [Span { start: 1, color: Color::Ansi(2) }, Span { start: 2, color: Color::Default }],
            "cut at {cut}"
        );
    }
}

#[test]
fn other_escape_sequences_are_taken_out() {
    let console = received(&[b"\x1b[2J\x1b[Hmenu\x1b[K\x1b]0;title\x07 end\x1b(B\n\x1b[38;5;202mx\x1b[38;2;1;2;3my\x1b[38;5;4mz\n"]);
    assert_eq!(texts(&console), ["menu end", "xyz"]);
    assert_eq!(console.lines()[1].spans(), [Span { start: 2, color: Color::Ansi(4) }], "only the 16 colours are kept");
}

#[test]
fn a_broken_escape_sequence_does_not_swallow_the_line_end() {
    assert_eq!(texts(&received(&[b"a\x1b[12\nb\x1b]never closed\nc\n"])), ["a", "b", "c"]);
}

#[test]
fn only_the_latest_lines_are_kept_and_keep_their_number() {
    let mut console = Console::new(3);
    console.receive(b"0\n1\n2\n3\n4\n", at(0, 0));
    assert_eq!(texts(&console), ["2", "3", "4"]);
    assert_eq!(console.first(), 2);
    assert_eq!(console.line(3).map(Line::text), Some("3"));
    assert!(console.line(1).is_none());
    console.set_limit(1);
    assert_eq!(texts(&console), ["4"]);
}

#[test]
fn a_line_with_no_end_is_cut_before_it_grows_for_ever() {
    let console = received(&[&vec![b'x'; LONGEST_LINE + 10]]);
    assert_eq!(console.lines().len(), 2);
    assert_eq!(console.lines()[0].text().len(), LONGEST_LINE);
    assert_eq!(console.lines()[1].text().len(), 10);
}

#[test]
fn a_line_bears_the_time_of_its_first_character() {
    let mut console = Console::new(100);
    console.receive(b"begun", at(22, 104));
    console.receive(b" then ended\n", at(23, 0));
    assert_eq!(console.lines()[0].display(TIMED), "10:15:22.104 begun then ended");
    assert_eq!(console.lines()[0].display(TEXT), "begun then ended");
    assert_eq!(console.lines()[0].width(TIMED), TIME_WIDTH + 16);
}

#[test]
fn sent_lines_and_notices_take_a_line_of_their_own() {
    let mut console = Console::new(100);
    console.receive(b"prompt> ", at(0, 0));
    console.sent("status", b"status\n", at(1, 0));
    console.receive(b"uptime 9 s\n", at(2, 0));
    console.notice("port fermé", at(3, 0));
    assert_eq!(texts(&console), ["prompt> ", "> status", "uptime 9 s", "# port fermé"]);
    let kinds: Vec<Kind> = console.lines().iter().map(|line| line.kind).collect();
    assert_eq!(kinds, [Kind::Received, Kind::Sent, Kind::Received, Kind::Notice]);
}

#[test]
fn clearing_empties_the_console_and_keeps_counting() {
    let mut console = received(&[b"a\nb\nhalf"]);
    console.clear();
    assert!(console.is_empty());
    assert_eq!(console.first(), 3);
    console.receive(b"c\n", at(0, 0));
    assert_eq!(texts(&console), ["c"]);
    assert_eq!(console.line(3).map(Line::text), Some("c"));
    assert_eq!(console.lines()[0].raw(), b"c\n");
}
