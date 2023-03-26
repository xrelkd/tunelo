// #![type_length_limit = "2000000"]
//
// #[macro_use]
// extern crate tracing;
//
// #[macro_use]
// extern crate serde;

use std::sync::atomic;

pub static SHUTDOWN: atomic::AtomicBool = atomic::AtomicBool::new(false);

mod command;
mod consts;
mod error;
mod shutdown;
mod signal_handler;

use self::command::Cli;

fn main() {
    if let Err(err) = Cli::default().run() {
        eprintln!("{err}");
        std::process::exit(1);
    }
}
