use snafu::Snafu;

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("StdIo error: {}", source))]
    StdIo { source: std::io::Error },

    #[snafu(display("Unsupported method: {}", method))]
    UnsupportedMethod { method: String },

    #[snafu(display("Bad request"))]
    BadRequest,

    #[snafu(display("Bad response"))]
    BadResponse,

    #[snafu(display("Host is unreachable"))]
    HostUnreachable,
}

impl From<std::io::Error> for Error {
    fn from(source: std::io::Error) -> Error { Error::StdIo { source } }
}
