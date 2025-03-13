use std::{
    env,
    fmt::Display,
    io::{self, Read},
};

use saiga_vte::{Params, Perform};

/// A type implementing Perform that just logs actions
#[derive(Default)]
struct Stat {
    printed: u64,
    executed: u64,
    hooked: u64,
    putted: u64,
    unhooked: u64,
    osc_dispatched: u64,
    csi_dispatched: u64,
    esc_dispatched: u64,
}

impl Display for Stat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "printed: {}\nexecuted: {}\nhooked: {}\nunhooked: {}\nputted: {}\nosc_dispatched: {}\ncsi_dispatched: {}\nesc_dispatched: {}",
            self.printed, self.executed, self.hooked, self.unhooked, self.putted, self.osc_dispatched, self.csi_dispatched, self.esc_dispatched,
        )
    }
}

impl saiga_vte::Perform for Stat {
    fn print(&mut self, _c: char) {
        self.printed += 1
    }

    fn execute(&mut self, _byte: u8) {
        self.executed += 1
    }

    fn hook(&mut self, _params: &Params, _intermediates: &[u8], _ignore: bool, _c: char) {
        self.hooked += 1
    }

    fn put(&mut self, _byte: u8) {
        self.putted += 1
    }

    fn unhook(&mut self) {
        self.unhooked += 1
    }

    fn osc_dispatch(&mut self, _params: &[&[u8]], _bell_terminated: bool) {
        self.osc_dispatched += 1
    }

    fn csi_dispatch(&mut self, _params: &Params, _intermediates: &[u8], _ignore: bool, _c: char) {
        self.csi_dispatched += 1
    }

    fn esc_dispatch(&mut self, _intermediates: &[u8], _ignore: bool, _byte: u8) {
        self.esc_dispatched += 1
    }
}

fn main() -> io::Result<()> {
    let input = io::stdin();
    let mut handle = input.lock();

    let mut buf = Vec::new();

    handle.read_to_end(&mut buf)?;

    let mut parser = saiga_vte::Parser::new();
    let mut performer = Stat::default();

    parser.advance(&mut performer, buf.as_slice());

    println!("{performer}");

    Ok(())
}
