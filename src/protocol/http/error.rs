#[derive(Debug)]
pub enum Error {
    StdIo(std::io::Error),
    UnsupportedMethod(String),
    BadRequest,
    BadResponse,
    HostUnreachable,
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Error { Error::StdIo(err) }
}
