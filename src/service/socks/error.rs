use snafu::Snafu;

use crate::{
    common::HostAddress,
    protocol::{
        self,
        socks::{v5::Method, SocksCommand, SocksVersion},
    },
    transport,
};

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
pub enum Error {
    #[snafu(display("Failed to get SOCKS version from host: {}, error: {}", peer_addr, source))]
    DetectSocksVersion { peer_addr: std::net::SocketAddr, source: std::io::Error },

    #[snafu(display("Could not bind UDP socket {}, error: {}", addr, source))]
    BindUdpSocket { addr: std::net::SocketAddr, source: std::io::Error },

    #[snafu(display("Error occurred while shutting down connection, error: {}", source))]
    Shutdown { source: std::io::Error },

    #[snafu(display("Could not write stream, error: {}", source))]
    WriteStream { source: std::io::Error },

    #[snafu(display("Error occurred while flushing stream, error: {}", source))]
    FlushStream { source: std::io::Error },

    #[snafu(display("Error occurred while relaying stream, error: {}", source))]
    RelayStream { source: transport::Error },

    #[snafu(display("Could not establish connection with {}, error: {}", host, source))]
    ConnectRemoteHost { host: HostAddress, source: transport::Error },

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

    #[snafu(display("Could not parse request, error: {}", source))]
    ParseRequest { source: protocol::socks::Error },

    #[snafu(display("Could not parse handshake request, error: {}", source))]
    ParseHandshakeRequest { source: protocol::socks::Error },
}
