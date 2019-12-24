#[macro_use]
extern crate serde_derive;

#[macro_use]
extern crate structopt;

#[macro_use]
extern crate log;

mod command;
mod http_server;
mod multi_server;
mod proxy_checker;
mod settings;
mod socks_server;

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

    command::Command::from_args().run();
}
