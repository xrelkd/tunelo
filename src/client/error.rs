use crate::client::handshake;
use crate::common::HostAddress;

#[derive(Debug)]
pub enum Error {
    StdIo(std::io::Error),
    Handshake(handshake::Error),
    ConnectForbiddenHost(HostAddress),
    NoProxyProvided,
    DatagramClosed,
    BadSocksReply,
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Error {
        Error::StdIo(err)
    }
}

impl From<handshake::Error> for Error {
    fn from(err: handshake::Error) -> Error {
        match err {
            handshake::Error::StdIo(err) => Error::StdIo(err),
            err => Error::Handshake(err),
        }
    }
}
