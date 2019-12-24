use crate::protocol;
use crate::transport;

#[derive(Debug)]
pub enum Error {
    StdIo(std::io::Error),
    Transport(transport::Error),
    Protocol(protocol::http::Error),
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

impl From<protocol::http::Error> for Error {
    fn from(err: protocol::http::Error) -> Error {
        Error::Protocol(err)
    }
}
