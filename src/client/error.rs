use snafu::Snafu;

use crate::{client::handshake, common::HostAddress};

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("StdIo error: {}", source))]
    StdIo { source: std::io::Error },

    #[snafu(display("Handshake error: {}", source))]
    Handshake { source: handshake::Error },

    #[snafu(display("Try to connect a forbidden host {:?}", addr))]
    ConnectForbiddenHost { addr: HostAddress },

    #[snafu(display("Remote host does not provide proxy service"))]
    NoProxyProvided,

    #[snafu(display("Datagram endpoint is closed"))]
    DatagramClosed,

    #[snafu(display("Received bad SOCKS reply"))]
    BadSocksReply,
}

impl From<std::io::Error> for Error {
    fn from(source: std::io::Error) -> Error { Error::StdIo { source } }
}

impl From<handshake::Error> for Error {
    fn from(err: handshake::Error) -> Error {
        match err {
            handshake::Error::StdIo { source } => Error::StdIo { source },
            err => Error::Handshake { source: err },
        }
    }
}
