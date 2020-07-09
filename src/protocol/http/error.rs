use snafu::Snafu;

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("Unsupported method: {}", method))]
    UnsupportedMethod { method: String },

    #[snafu(display("Could not parse HTTP request, error: {}", source))]
    ParseRequest { source: httparse::Error },

    #[snafu(display("Could not parse HTTP response, error: {}", source))]
    ParseResponse { source: httparse::Error },

    #[snafu(display("Host is unreachable"))]
    HostUnreachable,

    #[snafu(display("No HTTP method provided"))]
    NoMethodProvided,

    #[snafu(display("No HTTP path provided"))]
    NoPathProvided,

    #[snafu(display("Invalid HTTP method: {}", method))]
    InvalidMethod { method: String },

    #[snafu(display("Invalid path: {}", path))]
    InvalidPath { path: String },

    #[snafu(display("Invalid HTTP header name: {}", name))]
    InvalidHeaderName { name: String },

    #[snafu(display("Invalid HTTP header value: {}", value))]
    InvalidHeaderValue { value: String },
}
