use snafu::Snafu;

pub use self::report::ReportError;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
pub enum Error {
    #[snafu(display("Could not connect proxy server, error: {}", source))]
    ConnectProxyServer { source: crate::client::Error },

    #[snafu(display("Could not initialize TLS stream, error: {}", source))]
    InitializeTlsStream { source: std::io::Error },

    #[snafu(display("Error occurred when shutdown, error: {}", source))]
    Shutdown { source: std::io::Error },

    #[snafu(display("No host is provided"))]
    NoHostProvided,

    #[snafu(display("No port is provided"))]
    NoPortProvided,

    #[snafu(display("No path is provided"))]
    NoPathProvided,

    #[snafu(display("Unknown scheme: {}", scheme))]
    UnknownScheme { scheme: String },

    #[snafu(display("Could not read HTTP response, error: {}", source))]
    ReadHttpResponse { source: std::io::Error },

    #[snafu(display("Could not write HTTP request, error: {}", source))]
    WriteHttpRequest { source: std::io::Error },

    #[snafu(display("Could not parse HTTP request, error: {}", source))]
    ParseHttpRequest { source: httparse::Error },

    #[snafu(display("Could not parse HTTP response, error: {}", source))]
    ParseHttpResponse { source: httparse::Error },

    #[snafu(display("Incomplete HTTP response"))]
    IncompleteHttpResponse,

    #[snafu(display("Could not construct a DNSNameRef from `{dns_name}`, error: {source}"))]
    InvalidDnsName { dns_name: String, source: tokio_rustls::rustls::client::InvalidDnsNameError },

    #[snafu(display("Operation timed out"))]
    Timeout,
}

mod report {
    use snafu::Snafu;

    use crate::checker::error::Error;

    #[derive(Clone, Debug, Eq, PartialEq, Snafu)]
    pub enum ReportError {
        #[snafu(display("Could not connect proxy server, error: {message}"))]
        ConnectProxyServer { message: String },

        #[snafu(display("Could not initialize TLS stream, error: {message}"))]
        InitializeTlsStream { message: String },

        #[snafu(display("Error occurred when shutdown, error: {message}"))]
        Shutdown { message: String },

        #[snafu(display("No host is provided"))]
        NoHostProvided,

        #[snafu(display("No port is provided"))]
        NoPortProvided,

        #[snafu(display("No path is provided"))]
        NoPathProvided,

        #[snafu(display("Unknown scheme: {scheme}"))]
        UnknownScheme { scheme: String },

        #[snafu(display("Could not read HTTP response, error: {message}"))]
        ReadHttpResponse { message: String },

        #[snafu(display("Could not write HTTP request, error: {message}"))]
        WriteHttpRequest { message: String },

        #[snafu(display("Could not parse HTTP request, error: {source}"))]
        ParseHttpRequest { source: httparse::Error },

        #[snafu(display("Could not parse HTTP response, error: {source}",))]
        ParseHttpResponse { source: httparse::Error },

        #[snafu(display("Incomplete HTTP response"))]
        IncompleteHttpResponse,

        #[snafu(display("Invalid DNS name `{dns_name}`"))]
        InvalidDnsName { dns_name: String },

        #[snafu(display("Operation timed out"))]
        Timeout,
    }

    impl From<Error> for ReportError {
        fn from(err: Error) -> Self {
            match err {
                Error::ConnectProxyServer { source } => {
                    Self::ConnectProxyServer { message: source.to_string() }
                }
                Error::InitializeTlsStream { source } => {
                    Self::InitializeTlsStream { message: source.to_string() }
                }
                Error::Shutdown { source } => Self::Shutdown { message: source.to_string() },
                Error::NoHostProvided => Self::NoHostProvided,
                Error::NoPortProvided => Self::NoPortProvided,
                Error::NoPathProvided => Self::NoPathProvided,
                Error::UnknownScheme { scheme } => Self::UnknownScheme { scheme },
                Error::ReadHttpResponse { source } => {
                    Self::ReadHttpResponse { message: source.to_string() }
                }
                Error::WriteHttpRequest { source } => {
                    Self::WriteHttpRequest { message: source.to_string() }
                }
                Error::ParseHttpRequest { source } => Self::ParseHttpRequest { source },
                Error::ParseHttpResponse { source } => Self::ParseHttpResponse { source },
                Error::IncompleteHttpResponse => Self::IncompleteHttpResponse,
                Error::InvalidDnsName { dns_name, .. } => Self::InvalidDnsName { dns_name },
                Error::Timeout => Self::Timeout,
            }
        }
    }
}
