use crate::protocol::{self, socks::SocksVersion};
use crate::transport;

#[derive(Debug)]
pub enum Error {
    StdIo(std::io::Error),
    Transport(transport::Error),
    Protocol(protocol::socks::Error),
    UnsupportedCommand,
    UnsupportedSocksVersion(SocksVersion),
    UnsupportedMethod,
    AccessDenied { user_name: Vec<u8>, password: Vec<u8> },
    InvalidSocksVersion(u8),
    InvalidAddressType(u8),
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Error {
        Error::StdIo(err)
    }
}

impl From<transport::Error> for Error {
    fn from(err: transport::Error) -> Error {
        Error::Transport(err)
    }
}

impl From<protocol::socks::Error> for Error {
    fn from(err: protocol::socks::Error) -> Error {
        Error::Protocol(err)
    }
}
