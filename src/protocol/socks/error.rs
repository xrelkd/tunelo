use snafu::Snafu;

use crate::protocol::socks::SocksVersion;

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("StdIo error: {}", source))]
    StdIo { source: std::io::Error },

    #[snafu(display("Unsupported SOCKS version: {}", version))]
    UnsupportedSocksVersion { version: SocksVersion },

    #[snafu(display("Invalid SOCKS version: {}", version))]
    InvalidSocksVersion { version: u8 },

    #[snafu(display("Invalid address type: {}", ty))]
    InvalidAddressType { ty: u8 },

    #[snafu(display("Invalid user command: {}", command))]
    InvalidCommand { command: u8 },

    #[snafu(display("Invalid user password version: {}", version))]
    InvalidUserPasswordVersion { version: u8 },

    #[snafu(display("Bad request"))]
    BadRequest,

    #[snafu(display("Bad reply"))]
    BadReply,
}

impl From<std::io::Error> for Error {
    fn from(source: std::io::Error) -> Error { Error::StdIo { source } }
}
