use snafu::Snafu;

use crate::{common::HostAddress, transport};

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
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

    #[snafu(display("Could not establish connection with {host}, error: {source}"))]
    ConnectRemoteHost { host: HostAddress, source: Box<transport::Error> },

    #[snafu(display("Unsupported method: {}", method))]
    UnsupportedMethod { method: String },

    #[snafu(display("Could not parse HTTP request, error: {}", source))]
    ParseRequest { source: httparse::Error },

    #[snafu(display("Could not parse HTTP response, error: {}", source))]
    ParseResponse { source: httparse::Error },

    #[snafu(display("Could not parse URL from HTTP header, error: {}", source))]
    ParseUrl { source: url::ParseError },

    #[snafu(display("Host is unreachable"))]
    HostUnreachable,

    #[snafu(display("Invalid HTTP method: {}", method))]
    InvalidMethod { method: String },

    #[snafu(display("Invalid path: {}", path))]
    InvalidPath { path: String },

    #[snafu(display("Invalid HTTP header name: {}", name))]
    InvalidHeaderName { name: String },

    #[snafu(display("Invalid HTTP header value: {}", value))]
    InvalidHeaderValue { value: String },

    #[snafu(display("No HTTP method provided"))]
    NoMethodProvided,

    #[snafu(display("No host is provided"))]
    NoHostProvided,

    #[snafu(display("No port is provided"))]
    NoPortProvided,

    #[snafu(display("No path is provided"))]
    NoPathProvided,

    #[snafu(display("No URL is provided"))]
    NoUrlProvided,
}
