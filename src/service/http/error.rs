use snafu::Snafu;

use crate::{protocol, transport};

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("StdIo error: {}", source))]
    StdIo { source: std::io::Error },

    #[snafu(display("Transport error: {}", source))]
    Transport { source: transport::Error },

    #[snafu(display("Protocol error: {}", source))]
    Protocol { source: protocol::http::Error },
}

impl From<std::io::Error> for Error {
    fn from(source: std::io::Error) -> Error { Error::StdIo { source } }
}

impl From<transport::Error> for Error {
    fn from(source: transport::Error) -> Error { Error::Transport { source } }
}

impl From<protocol::http::Error> for Error {
    fn from(source: protocol::http::Error) -> Error { Error::Protocol { source } }
}
