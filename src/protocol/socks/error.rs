use snafu::Snafu;

use crate::protocol::socks::SocksVersion;

#[derive(Debug, Snafu)]
#[snafu(visibility = "pub(crate)")]
pub enum Error {
    #[snafu(display("Could not read stream, error: {}", source))]
    ReadStream { source: std::io::Error },

    #[snafu(display("Could not write stream, error: {}", source))]
    WriteStream { source: std::io::Error },

    #[snafu(display("Could not bind UDP socket, error: {}", source))]
    BindUdpSocket { source: std::io::Error },

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
