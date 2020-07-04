use snafu::Snafu;

use crate::{
    protocol::{
        self,
        socks::{v5::Method, SocksCommand, SocksVersion},
    },
    transport,
};

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("StdIo error: {}", source))]
    StdIo { source: std::io::Error },

    #[snafu(display("Transport error: {}", source))]
    Transport { source: transport::Error },

    #[snafu(display("Protocol error: {}", source))]
    Protocol { source: protocol::socks::Error },

    #[snafu(display("Unsupported SOCKS command: {}", command))]
    UnsupportedCommand { command: SocksCommand },

    #[snafu(display("Unsupported SOCKS version: {}", version))]
    UnsupportedSocksVersion { version: SocksVersion },

    #[snafu(display("Unsupported method: {}", method))]
    UnsupportedMethod { method: Method },

    #[snafu(display(
        "Access denied user name: {}, password: {}",
        String::from_utf8_lossy(user_name),
        String::from_utf8_lossy(password)
    ))]
    AccessDenied { user_name: Vec<u8>, password: Vec<u8> },

    #[snafu(display("Invalid SOCKS version: {}", version))]
    InvalidSocksVersion { version: u8 },

    #[snafu(display("Invalid address type: {}", ty))]
    InvalidAddressType { ty: u8 },
}

impl From<std::io::Error> for Error {
    fn from(source: std::io::Error) -> Error { Error::StdIo { source } }
}

impl From<transport::Error> for Error {
    fn from(source: transport::Error) -> Error { Error::Transport { source } }
}

impl From<protocol::socks::Error> for Error {
    fn from(source: protocol::socks::Error) -> Error { Error::Protocol { source } }
}
