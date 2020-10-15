#[macro_use]
extern crate tracing;

#[macro_use]
extern crate serde;

use std::sync::atomic;

pub static SHUTDOWN: atomic::AtomicBool = atomic::AtomicBool::new(false);

mod command;
mod error;
mod shutdown;
mod signal_handler;

use self::command::Command;

mod consts {
    pub const THREAD_NAME: &str = "tunelo";
}

fn main() {
    let cmd = Command::new();
    if let Err(err) = cmd.run() {
        eprintln!("{}", err);
        std::process::exit(1);
    }
}
