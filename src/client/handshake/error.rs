use snafu::Snafu;

use crate::protocol::{
    http::Error as HttpError,
    socks::{v5::Method as SocksV5Method, Error as SocksError, SocksVersion},
};

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("StdIo error: {}", source))]
    StdIo { source: std::io::Error },

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

    #[snafu(display("Unsupported SOCKS version: {}", version))]
    UnsupportedSocksVersion { version: SocksVersion },

    #[snafu(display("Unsupported SOCKS method: {}", method))]
    UnsupportedSocksMethod { method: SocksV5Method },

    #[snafu(display("Invalid SOCKS4a id: {}", String::from_utf8_lossy(id)))]
    InvalidSocks4aId { id: Vec<u8> },

    #[snafu(display("Invalid SOCKS version: {}", version))]
    InvalidSocksVersion { version: u8 },

    #[snafu(display("Invalid SOCKS address type: {}", ty))]
    InvalidSocksAddressType { ty: u8 },

    #[snafu(display("Invalid SOCKS command: {}", command))]
    InvalidSocksCommand { command: u8 },

    #[snafu(display("Invalid SOCKS username/password version: {}", version))]
    InvalidSocksUserPasswordVersion { version: u8 },

    #[snafu(display("Bad SOCKS request"))]
    BadSocksRequest,

    #[snafu(display("Bad SOCKS reply"))]
    BadSocksReply,

    #[snafu(display("Unsupported HTTP method: {}", method))]
    UnsupportedHttpMethod { method: String },

    #[snafu(display("Bad HTTP request"))]
    BadHttpRequest,

    #[snafu(display("Bad HTTP response"))]
    BadHttpResponse,
}

impl From<std::io::Error> for Error {
    fn from(source: std::io::Error) -> Error { Error::StdIo { source } }
}

impl From<SocksError> for Error {
    fn from(err: SocksError) -> Error {
        match err {
            SocksError::StdIo { source } => Error::StdIo { source },
            SocksError::UnsupportedSocksVersion { version } => {
                Error::UnsupportedSocksVersion { version }
            }
            SocksError::InvalidSocksVersion { version } => Error::InvalidSocksVersion { version },
            SocksError::InvalidAddressType { ty } => Error::InvalidSocksAddressType { ty },
            SocksError::InvalidCommand { command } => Error::InvalidSocksCommand { command },
            SocksError::InvalidUserPasswordVersion { version } => {
                Error::InvalidSocksUserPasswordVersion { version }
            }
            SocksError::BadRequest => Error::BadSocksRequest,
            SocksError::BadReply => Error::BadSocksReply,
        }
    }
}

impl From<HttpError> for Error {
    fn from(err: HttpError) -> Error {
        match err {
            HttpError::StdIo { source } => Error::StdIo { source },
            HttpError::UnsupportedMethod { method } => Error::UnsupportedHttpMethod { method },
            HttpError::BadRequest => Error::BadHttpRequest,
            HttpError::BadResponse => Error::BadHttpResponse,
            HttpError::HostUnreachable => Error::HostUnreachable,
        }
    }
}
