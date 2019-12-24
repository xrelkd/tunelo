use crate::protocol::socks::SocksVersion;

#[derive(Debug)]
pub enum Error {
    StdIo(std::io::Error),
    UnsupportedSocksVersion(SocksVersion),
    InvalidSocksVersion(u8),
    InvalidAddressType(u8),
    InvalidCommand(u8),
    InvalidUserPasswordVersion(u8),
    BadRequest,
    BadReply,
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Error {
        Error::StdIo(err)
    }
}
