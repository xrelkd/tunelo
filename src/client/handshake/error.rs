use snafu::Snafu;

use crate::protocol::socks::{v5::Method as SocksV5Method, Error as SocksError};

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("Could not read stream, error: {}", source))]
    ReadStream { source: std::io::Error },

    #[snafu(display("Could not read stream, error: {}", source))]
    WriteStream { source: std::io::Error },

    #[snafu(display("Error occurred while shutdown stream, error: {}", source))]
    ShutdownStream { source: std::io::Error },

    #[snafu(display("Could not parse SOCKS4, error: {}", source))]
    ParseSocks4Reply { source: SocksError },

    #[snafu(display("Could not parse SOCKS5, error: {}", source))]
    ParseSocks5Reply { source: SocksError },

    #[snafu(display("Host is unreachable"))]
    HostUnreachable,

    #[snafu(display("Remote proxy rejected our request"))]
    ProxyRejected,

    #[snafu(display(
        "Access denied user name: {}, password: {}",
        String::from_utf8_lossy(user_name),
        String::from_utf8_lossy(password)
    ))]
    AccessDenied { user_name: Vec<u8>, password: Vec<u8> },

    #[snafu(display("Unsupported SOCKS method: {}", method))]
    UnsupportedSocksMethod { method: SocksV5Method },

    #[snafu(display("Invalid SOCKS4a id: {}", String::from_utf8_lossy(id)))]
    InvalidSocks4aId { id: Vec<u8> },

    #[snafu(display("Unsupported HTTP method: {}", method))]
    UnsupportedHttpMethod { method: String },

    #[snafu(display("Bad HTTP request"))]
    BadHttpRequest,

    #[snafu(display("Bad HTTP response"))]
    BadHttpResponse,
}
