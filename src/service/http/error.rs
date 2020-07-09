use snafu::Snafu;

use crate::{protocol, transport};

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("Could not read buffer, error: {}", source))]
    ReadBuf { source: std::io::Error },

    #[snafu(display("Could not write stream error: {}", source))]
    WriteStream { source: std::io::Error },

    #[snafu(display("Error occurred while shutting down TCP stream, error: {}", source))]
    Shutdown { source: std::io::Error },

    #[snafu(display("HTTP request is too large"))]
    RequestTooLarge,

    #[snafu(display("Transport error: {}", source))]
    Transport { source: transport::Error },

    #[snafu(display("Protocol error: {}", source))]
    Protocol { source: protocol::http::Error },
}

impl From<transport::Error> for Error {
    fn from(source: transport::Error) -> Error { Error::Transport { source } }
}

impl From<protocol::http::Error> for Error {
    fn from(source: protocol::http::Error) -> Error { Error::Protocol { source } }
}
