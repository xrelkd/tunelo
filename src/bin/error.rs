use std::{fmt, path::PathBuf};

use snafu::Snafu;

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("Could not initialize tokio runtime, error: {}", source))]
    InitializeTokioRuntime { source: tokio::io::Error },

    #[snafu(display("Could not initialize domain name resolver, error: {}", source))]
    InitializeDomainNameResolver { source: tunelo::transport::Error },

    #[snafu(display("Read configuration file {}, error: {}", file_path.display(), source))]
    ReadConfigFile { source: std::io::Error, file_path: PathBuf },

    #[snafu(display("Deserialize configuration file {:?}, error: {}", file_path.display(), source))]
    DeserializeConfig { source: toml::de::Error, file_path: PathBuf },

    #[snafu(display("No proxy server is enabled"))]
    NoProxyServer,

    #[snafu(display("Could not run SOCKs proxy server, error: {}", source))]
    RunSocksServer { source: tunelo::service::socks::Error },

    #[snafu(display("Could not run HTTP proxy server, error: {}", source))]
    RunHttpServer { source: tunelo::service::http::Error },

    #[snafu(display("Errors occurred: {}", Errors::from(errors)))]
    ErrorCollection { errors: Vec<Error> },

    #[snafu(display("Could not create Transport, error: {}", source))]
    CreateTransport { source: tunelo::transport::Error },

    #[snafu(display("No SOCKS service is enabled"))]
    NoSocksServiceEnabled,

    #[snafu(display("No SOCKS command is enabled, try to enable some commands"))]
    NoSocksCommandEnabled,

    #[snafu(display("UDP associate is enabled but no UDP port is provided"))]
    NoSocksUdpPort,

    #[snafu(display("TCP bind is not supported yet"))]
    TcpBindNotSupported,

    #[snafu(display("No proxy chain provided"))]
    NoProxyChain,

    #[snafu(display("Miss SOCKS listen address"))]
    NoSocksListenAddress,

    #[snafu(display("Miss SOCKS listen port"))]
    NoSocksListenPort,

    #[snafu(display("Miss HTTP listen address"))]
    NoHttpListenAddress,

    #[snafu(display("Miss HTTP listen port"))]
    NoHttpListenPort,

    #[snafu(display("Could not parse ProxyHost, error: {}", source))]
    ParseProxyHost { source: serde_json::Error },

    #[snafu(display("Could not load ProxyHost file, error: {}", source))]
    LoadProxyHostFile { source: std::io::Error },

    #[snafu(display("Invalid proxy server: {}", server))]
    InvalidProxyServer { server: String },
}

pub struct Errors<'a>(&'a Vec<Error>);

impl<'a> From<&'a Vec<Error>> for Errors<'a> {
    fn from(errors: &'a Vec<Error>) -> Errors<'a> { Errors(errors) }
}

impl<'a> fmt::Display for Errors<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let errors = self.0.iter().map(|e| e.to_string()).collect::<Vec<_>>().join("\n");
        write!(f, "{}", errors)
    }
}
