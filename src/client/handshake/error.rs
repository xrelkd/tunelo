use crate::protocol::{
    http::Error as HttpError,
    socks::{Error as SocksError, SocksVersion},
};

#[derive(Debug)]
pub enum Error {
    StdIo(std::io::Error),
    HostUnreachable,
    ProxyRejected,
    AccessDenied { user_name: Vec<u8>, password: Vec<u8> },

    UnsupportedSocksVersion(SocksVersion),
    UnsupportedSocksMethod,
    InvalidSocks4aId(Vec<u8>),
    InvalidSocksVersion(u8),
    InvalidSocksAddressType(u8),
    InvalidSocksCommand(u8),
    InvalidSocksUserPasswordVersion(u8),
    BadSocksRequest,
    BadSocksReply,

    UnsupportedHttpMethod(String),
    BadHttpRequest,
    BadHttpResponse,
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Error { Error::StdIo(err) }
}

impl From<SocksError> for Error {
    fn from(err: SocksError) -> Error {
        match err {
            SocksError::StdIo(err) => Error::StdIo(err),
            SocksError::UnsupportedSocksVersion(version) => Error::UnsupportedSocksVersion(version),
            SocksError::InvalidSocksVersion(v) => Error::InvalidSocksVersion(v),
            SocksError::InvalidAddressType(t) => Error::InvalidSocksAddressType(t),
            SocksError::InvalidCommand(v) => Error::InvalidSocksCommand(v),
            SocksError::InvalidUserPasswordVersion(v) => Error::InvalidSocksUserPasswordVersion(v),
            SocksError::BadRequest => Error::BadSocksRequest,
            SocksError::BadReply => Error::BadSocksReply,
        }
    }
}

impl From<HttpError> for Error {
    fn from(err: HttpError) -> Error {
        match err {
            HttpError::StdIo(err) => Error::StdIo(err),
            HttpError::UnsupportedMethod(method) => Error::UnsupportedHttpMethod(method),
            HttpError::BadRequest => Error::BadHttpRequest,
            HttpError::BadResponse => Error::BadHttpResponse,
            HttpError::HostUnreachable => Error::HostUnreachable,
        }
    }
}
