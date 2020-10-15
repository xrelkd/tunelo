use snafu::Snafu;

use crate::{client::handshake, common::HostAddress};

#[derive(Debug, Snafu)]
#[snafu(visibility = "pub(crate)")]
pub enum Error {
    #[snafu(display("Could not receive datagram, error: {}", source))]
    RecvDatagram { source: std::io::Error },

    #[snafu(display("Could not send datagram, error: {}", source))]
    SendDatagram { source: std::io::Error },

    #[snafu(display("Could not bind UDP socket, error: {}", source))]
    BindUdpSocket { addr: std::net::SocketAddr, source: std::io::Error },

    #[snafu(display(
        "Could not get the local address that this socket is bound to, error: {}",
        source
    ))]
    GetLocalAddress { source: std::io::Error },

    #[snafu(display("Could not connect proxy server, error: {}", source))]
    ConnectProxyServer { source: std::io::Error },

    #[snafu(display("Could not connect UDP socket {}, error: {}", addr, source))]
    ConnectUdpSocket { addr: HostAddress, source: std::io::Error },

    #[snafu(display("Connection timed out"))]
    Timeout,

    #[snafu(display("Error occurred while shutting down connection, error: {}", source))]
    Shutdown { source: std::io::Error },

    #[snafu(display("Error occurred while handshaking, error: {}", source))]
    Handshake { source: handshake::Error },

    #[snafu(display("Try to connect a forbidden host {}", addr))]
    ConnectForbiddenHost { addr: HostAddress },

    #[snafu(display("Remote host does not provide proxy service"))]
    NoProxyServiceProvided,

    #[snafu(display("Datagram endpoint is closed"))]
    DatagramClosed,

    #[snafu(display("Received bad SOCKS reply"))]
    BadSocksReply,

    #[snafu(display("Could not serialize datagram, error: {}", source))]
    SerializeDatagram { source: std::io::Error },
}

impl From<handshake::Error> for Error {
    fn from(source: handshake::Error) -> Error { Error::Handshake { source } }
}
