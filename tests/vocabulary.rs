//! RSC is a generic tool: what it says calls a device a device, and names no company nor any of its
//! products. The texts of the window, in both languages as `t!` and `tr!` are given them, and the
//! README are read and checked. The check is shown to find a word on texts written on purpose.
//!
//! A company's words are not written here, where anyone reads them: they are read from
//! `tests/banned.local`, which git ignores, on the machine that has it. One word per line;
//! `=word` counts only as a word of its own; a line that begins with `#` is a comment.

use std::fs;
use std::path::{Path, PathBuf};

/// The words that must not be said, in lower case, and whether only the whole word counts: a
/// device profile's former name.
const BANNED: [(&str, bool); 3] = [("équipement", false), ("equipement", false), ("equipment", false)];

/// The list of the words kept on the machine, from the package's root.
const LOCAL: &str = "tests/banned.local";

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The Rust files under a folder, at any depth.
fn sources(folder: &Path, found: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(folder).unwrap_or_else(|error| panic!("{}: {error}", folder.display())) {
        let path = entry.expect("an entry of the folder").path();
        if path.is_dir() {
            sources(&path, found);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            found.push(path);
        }
    }
}

/// The string literal at the start of `code`, after any blank, and what follows it: "..." or
/// r#"..."#. Its escapes are left as they are written.
fn literal(code: &str) -> Option<(&str, &str)> {
    let code = code.trim_start();
    if let Some(raw) = code.strip_prefix('r') {
        let hashes = raw.len() - raw.trim_start_matches('#').len();
        let body = raw[hashes..].strip_prefix('"')?;
        let end = body.find(&format!("\"{}", "#".repeat(hashes)))?;
        return Some((&body[..end], &body[end + 1 + hashes..]));
    }
    let body = code.strip_prefix('"')?;
    let mut escaped = false;
    for (at, c) in body.char_indices() {
        match c {
            _ if escaped => escaped = false,
            '\\' => escaped = true,
            '"' => return Some((&body[..at], &body[at + 1..])),
            _ => {}
        }
    }
    None
}

/// The texts of the window in a file's code: for each `t!` and `tr!`, its French text and its
/// English one, with the line where it is written.
fn texts(code: &str) -> Vec<(usize, String)> {
    let mut found = Vec::new();
    for macro_name in ["t!(", "tr!("] {
        for (at, _) in code.match_indices(macro_name) {
            // "assert!(" and "print!(" end as "t!(" does: only a macro of that very name counts.
            if code[..at].chars().next_back().is_some_and(|before| before.is_alphanumeric() || before == '_') {
                continue;
            }
            let line = code[..at].matches('\n').count() + 1;
            let Some((french, rest)) = literal(&code[at + macro_name.len()..]) else {
                continue;
            };
            let Some((english, _)) = rest.trim_start().strip_prefix(',').and_then(literal) else {
                continue;
            };
            found.push((line, french.to_owned()));
            found.push((line, english.to_owned()));
        }
    }
    found
}

/// The words of a local list, in lower case, and whether only the whole word counts: one word per
/// line, `=` before a word that counts only on its own, `#` before a comment.
fn local_words(list: &str) -> Vec<(String, bool)> {
    list.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(|line| line.strip_prefix('=').map_or_else(|| (line.to_lowercase(), false), |word| (word.trim().to_lowercase(), true)))
        .collect()
}

/// Every banned word: those written here, then those of the local list when the machine has one.
fn banned() -> Vec<(String, bool)> {
    let mut words: Vec<(String, bool)> = BANNED.iter().map(|&(word, whole)| (word.to_owned(), whole)).collect();
    if let Ok(list) = fs::read_to_string(root().join(LOCAL)) {
        words.extend(local_words(&list));
    }
    words
}

/// The banned words a text says.
fn banned_in(text: &str, banned: &[(String, bool)]) -> Vec<String> {
    let text = text.to_lowercase();
    let whole = |at: usize, word: &str| {
        let before = text[..at].chars().next_back();
        let after = text[at + word.len()..].chars().next();
        !before.is_some_and(char::is_alphanumeric) && !after.is_some_and(char::is_alphanumeric)
    };
    banned.iter().filter(|(word, only_whole)| text.match_indices(word.as_str()).any(|(at, _)| !only_whole || whole(at, word))).map(|(word, _)| word.clone()).collect()
}

#[test]
fn the_check_finds_the_texts_and_the_words() {
    let code = "assert!(x);\nlet a = t!(\"Nommer l'\u{e9}quipement\", \"Name the \\\"equipment\\\"\");\nlet b = tr!(\n    r#\"{n} propres\"#,\n    \"{n} of its own\",\n    n = 2\n);";
    let found = texts(code);
    assert_eq!(found, [(2, "Nommer l'\u{e9}quipement".to_owned()), (2, "Name the \\\"equipment\\\"".to_owned()), (3, "{n} propres".to_owned()), (3, "{n} of its own".to_owned())]);
    assert_eq!(banned_in(&found[0].1, &banned()), ["équipement"]);
    // Made-up words, as a local list would give them: a company, a product, a name of its own.
    let local = [("acme".to_owned(), false), ("rx-7".to_owned(), false), ("zq".to_owned(), true)];
    assert_eq!(banned_in("Exported as by ZQ; Carte RX-7 n°3; Acme", &local), ["acme", "rx-7", "zq"]);
    assert!(banned_in("Steps, temps, a Zqx board", &local).is_empty(), "ZQ counts only as a word of its own");
}

#[test]
fn a_local_list_is_read_line_by_line() {
    let list = "# Made-up words\nAcme\n\n  RX-7  \n=ZQ\n";
    assert_eq!(local_words(list), [("acme".to_owned(), false), ("rx-7".to_owned(), false), ("zq".to_owned(), true)]);
}

#[test]
fn git_never_takes_the_local_list() {
    let ignored = fs::read_to_string(root().join(".gitignore")).expect("the .gitignore");
    assert!(ignored.lines().any(|line| line.trim() == format!("/{LOCAL}")), "the .gitignore names /{LOCAL}");
}

#[test]
fn no_text_of_the_window_says_equipment_nor_a_company_name() {
    let list = banned();
    let mut files = Vec::new();
    sources(&root().join("src"), &mut files);
    let mut said = Vec::new();
    let mut count = 0;
    for file in &files {
        let code = fs::read_to_string(file).unwrap_or_else(|error| panic!("{}: {error}", file.display()));
        for (line, text) in texts(&code) {
            count += 1;
            let words = banned_in(&text, &list);
            if !words.is_empty() {
                said.push(format!("{}:{line}: {words:?} in \"{text}\"", file.strip_prefix(root()).unwrap_or(file).display()));
            }
        }
    }
    assert!(count > 300, "the texts of the window are found: {count}");
    assert!(said.is_empty(), "{} texts:\n{}", said.len(), said.join("\n"));
}

/// The pages a reader of the project reads: the README, the docs, and the READMEs of the assets and
/// of the terminal's recordings.
fn pages() -> Vec<PathBuf> {
    let mut pages = vec![
        root().join("README.md"),
        root().join("assets").join("README.md"),
        root().join("tests").join("terminal").join("README.md"),
    ];
    for entry in fs::read_dir(root().join("docs")).expect("the docs") {
        let path = entry.expect("an entry of the docs").path();
        if path.extension().is_some_and(|extension| extension == "md") {
            pages.push(path);
        }
    }
    pages
}

#[test]
fn the_readme_and_the_docs_say_neither_equipment_nor_a_company_name() {
    let list = banned();
    let pages = pages();
    assert!(pages.iter().any(|page| page.ends_with("docs/guide.md")), "the user guide is read: {pages:?}");
    assert!(pages.iter().all(|page| page.is_file()), "every page is there: {pages:?}");
    let mut said = Vec::new();
    for page in &pages {
        let text = fs::read_to_string(page).unwrap_or_else(|error| panic!("{}: {error}", page.display()));
        for (index, line) in text.lines().enumerate() {
            let words = banned_in(line, &list);
            if !words.is_empty() {
                said.push(format!("{}:{}: {words:?}", page.strip_prefix(root()).unwrap_or(page).display(), index + 1));
            }
        }
    }
    assert!(said.is_empty(), "{} lines:\n{}", said.len(), said.join("\n"));
}
