use snafu::Snafu;

use crate::{common::HostAddress, protocol, transport};

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

    #[snafu(display("Error occurred while relaying stream, error: {}", source))]
    RelayStream { source: transport::Error },

    #[snafu(display("Could not establish connection with {}, error: {}", host, source))]
    ConnectRemoteHost { host: HostAddress, source: transport::Error },

    #[snafu(display("Occurred protocol error: {}", source))]
    OtherProtocolError { source: protocol::http::Error },
}
