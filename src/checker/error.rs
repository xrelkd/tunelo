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

    #[snafu(display("Could not construct a DNSNameRef from `{}`, error: {}", name, source))]
    ConstructsDNSNameRef { name: String, source: webpki::InvalidDNSNameError },

    #[snafu(display("Operation timed out"))]
    Timeout,
}

mod report {
    use snafu::Snafu;

    use crate::checker::error::Error;

    #[derive(Debug, Clone, Eq, PartialEq, Snafu)]
    pub enum ReportError {
        #[snafu(display("Could not connect proxy server, error: {}", message))]
        ConnectProxyServer { message: String },

        #[snafu(display("Could not initialize TLS stream, error: {}", message))]
        InitializeTlsStream { message: String },

        #[snafu(display("Error occurred when shutdown, error: {}", message))]
        Shutdown { message: String },

        #[snafu(display("No host is provided"))]
        NoHostProvided,

        #[snafu(display("No port is provided"))]
        NoPortProvided,

        #[snafu(display("No path is provided"))]
        NoPathProvided,

        #[snafu(display("Unknown scheme: {}", scheme))]
        UnknownScheme { scheme: String },

        #[snafu(display("Could not read HTTP response, error: {}", message))]
        ReadHttpResponse { message: String },

        #[snafu(display("Could not write HTTP request, error: {}", message))]
        WriteHttpRequest { message: String },

        #[snafu(display("Could not parse HTTP request, error: {}", source))]
        ParseHttpRequest { source: httparse::Error },

        #[snafu(display("Could not parse HTTP response, error: {}", source))]
        ParseHttpResponse { source: httparse::Error },

        #[snafu(display("Incomplete HTTP response"))]
        IncompleteHttpResponse,

        #[snafu(display("Could not construct a DNSNameRef from `{}`, error: {}", name, source))]
        ConstructsDNSNameRef { name: String, source: webpki::InvalidDNSNameError },

        #[snafu(display("Operation timed out"))]
        Timeout,
    }

    impl From<Error> for ReportError {
        fn from(err: Error) -> ReportError {
            match err {
                Error::ConnectProxyServer { source } => {
                    ReportError::ConnectProxyServer { message: source.to_string() }
                }
                Error::InitializeTlsStream { source } => {
                    ReportError::InitializeTlsStream { message: source.to_string() }
                }
                Error::Shutdown { source } => ReportError::Shutdown { message: source.to_string() },
                Error::NoHostProvided => ReportError::NoHostProvided,
                Error::NoPortProvided => ReportError::NoPortProvided,
                Error::NoPathProvided => ReportError::NoPathProvided,
                Error::UnknownScheme { scheme } => ReportError::UnknownScheme { scheme },
                Error::ReadHttpResponse { source } => {
                    ReportError::ReadHttpResponse { message: source.to_string() }
                }
                Error::WriteHttpRequest { source } => {
                    ReportError::WriteHttpRequest { message: source.to_string() }
                }
                Error::ParseHttpRequest { source } => ReportError::ParseHttpRequest { source },
                Error::ParseHttpResponse { source } => ReportError::ParseHttpResponse { source },
                Error::IncompleteHttpResponse => ReportError::IncompleteHttpResponse,
                Error::ConstructsDNSNameRef { name, source } => {
                    ReportError::ConstructsDNSNameRef { name, source }
                }
                Error::Timeout => ReportError::Timeout,
            }
        }
    }
}
