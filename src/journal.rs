//! The console written to a file as it goes: every line that ends is added to it, beyond what the
//! console keeps on screen.

use std::fs::{File, OpenOptions};
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};

use crate::console::{Line, Shown};

pub struct Journal {
    path: PathBuf,
    file: BufWriter<File>,
}

impl Journal {
    /// Opens `path` to add to it: what it already holds stays.
    pub fn start(path: &Path) -> io::Result<Self> {
        let file = OpenOptions::new().create(true).append(true).open(path)?;
        Ok(Self { path: path.to_owned(), file: BufWriter::new(file) })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Adds lines as the console shows them, and hands them to the system at once: a tool closed
    /// in a hurry loses nothing.
    pub fn write(&mut self, lines: &[Line], shown: Shown) -> io::Result<()> {
        for line in lines {
            writeln!(self.file, "{}", line.display(shown))?;
        }
        self.file.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::console::{Console, at};
    use crate::settings::test_folder;

    const TEXT: Shown = Shown { timestamps: false, hex: false };
    const TIMED: Shown = Shown { timestamps: true, hex: false };

    #[test]
    fn lines_are_added_to_the_file_as_shown() {
        let folder = test_folder("journal");
        let path = folder.join("capture.log");
        let mut console = Console::new(100);
        console.follow(true);

        let mut journal = Journal::start(&path).expect("started");
        console.receive(b"boot ok\r\nready\r\n", at(1, 0));
        journal.write(&console.take_ended(), TEXT).expect("written");
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "boot ok\nready\n", "on disk before the journal is closed");
        console.sent("status", b"status\n", at(2, 250));
        journal.write(&console.take_ended(), TIMED).expect("written");
        drop(journal);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "boot ok\nready\n10:15:02.250 > status\n");

        // Started again on the same file: what it holds stays. In hexadecimal when asked.
        let mut journal = Journal::start(&path).expect("started again");
        console.receive(b"later\n", at(3, 0));
        journal.write(&console.take_ended(), Shown { timestamps: false, hex: true }).expect("written");
        assert_eq!(journal.path(), path);
        drop(journal);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "boot ok\nready\n10:15:02.250 > status\n6C 61 74 65 72 0A\n");
        let _ = std::fs::remove_dir_all(folder);
    }

    #[test]
    fn a_file_that_cannot_be_opened_says_so() {
        let folder = test_folder("journal-refused");
        assert!(Journal::start(&folder.join("missing").join("capture.log")).is_err());
        let _ = std::fs::remove_dir_all(folder);
    }
}
