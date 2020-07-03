#[macro_use]
extern crate serde_derive;

#[macro_use]
extern crate log;

mod command;
mod http_server;
mod multi_server;
mod proxy_checker;
mod settings;
mod shutdown;
mod signal_handler;
mod socks_server;

use std::sync::atomic;

pub static SHUTDOWN: atomic::AtomicBool = atomic::AtomicBool::new(false);

mod consts {
    pub const THREAD_NAME: &str = "tunelo";
}

mod exit_code {
    pub const EXIT_SUCCESS: i32 = 0;
    pub const EXIT_FAILURE: i32 = 1;
}

use structopt::StructOpt;

fn main() {
    use log::Level;
    simple_logger::init_with_level(Level::Info).unwrap();

    let cmd = command::Command::from_args();
    cmd.run();
}
