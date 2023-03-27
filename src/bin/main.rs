mod command;
mod consts;
mod error;
mod shutdown;
mod signal_handler;

use std::sync::atomic;

pub static SHUTDOWN: atomic::AtomicBool = atomic::AtomicBool::new(false);

use self::command::Cli;

fn main() {
    if let Err(err) = Cli::default().run() {
        eprintln!("{err}");
        std::process::exit(1);
    }
}
