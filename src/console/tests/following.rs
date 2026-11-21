//! The lines that end, handed once to who follows them (the journal, the curves).

use super::*;

#[test]
fn lines_that_end_are_handed_once_to_who_follows() {
    let mut console = Console::new(2);
    console.receive(b"before\n", at(0, 0));
    console.follow(true);
    console.receive(b"one\ntwo\nthree\nha", at(1, 0));
    console.sent("go", b"go\n", at(2, 0));
    console.receive(b"\n", at(3, 0));
    let ended: Vec<String> = console.take_ended().iter().map(|line| line.text().to_owned()).collect();
    assert_eq!(ended, ["one", "two", "three", "ha", "> go", ""], "even those the console no longer keeps");
    assert!(console.take_ended().is_empty());
    console.follow(false);
    console.receive(b"later\n", at(4, 0));
    assert!(console.take_ended().is_empty());
}

#[test]
fn following_again_loses_no_line() {
    let mut console = Console::new(10);
    console.follow(true);
    console.receive(b"one\n", at(0, 0));
    console.follow(true);
    console.receive(b"two\n", at(0, 0));
    let ended: Vec<String> = console.take_ended().iter().map(|line| line.text().to_owned()).collect();
    assert_eq!(ended, ["one", "two"]);
}

#[test]
fn two_equal_lines_in_a_row_are_both_handed() {
    let mut console = Console::new(10);
    console.follow(true);
    console.receive(b"ok\nok\n", at(0, 0));
    console.notice("fin", at(1, 0));
    console.notice("fin", at(1, 0));
    let ended: Vec<String> = console.take_ended().iter().map(|line| line.text().to_owned()).collect();
    assert_eq!(ended, ["ok", "ok", "# fin", "# fin"]);
}
