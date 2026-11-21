//! The console's tests, a file a theme: bytes received, bytes kept, selections, following,
//! folding. What they share: the ways of showing, and consoles made of what was received.

mod bytes;
mod fold;
mod following;
mod receiving;
mod selection;

use super::*;

const TEXT: Shown = Shown { timestamps: false, hex: false };
const TIMED: Shown = Shown { timestamps: true, hex: false };
const HEX: Shown = Shown { timestamps: false, hex: true };

fn received(chunks: &[&[u8]]) -> Console {
    let mut console = Console::new(1000);
    for chunk in chunks {
        console.receive(chunk, at(0, 0));
    }
    console
}
