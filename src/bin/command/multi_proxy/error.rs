use std::path::PathBuf;

use snafu::Snafu;

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("Could not run SOCKs proxy server, error: {}", source))]
    RunSocksServer {
        source: tunelo::service::socks::Error,
    },

    #[snafu(display("Could not run HTTP proxy server, error: {}", source))]
    RunHttpServer {
        source: tunelo::service::http::Error,
    },

    ErrorCollection {
        errors: Vec<Error>,
    },

    #[snafu(display("Read configuration file {}, error: {}", file_path.display(), source))]
    ReadConfigFile {
        source: std::io::Error,
        file_path: PathBuf,
    },

    #[snafu(display("Deserialize configuration file {:?}, error: {}", file_path.display(), source))]
    DeserializeConfig {
        source: toml::de::Error,
        file_path: PathBuf,
    },

    #[snafu(display("Invalid proxy server: {}", server))]
    InvalidProxyServer {
        server: String,
    },

    #[snafu(display("No UDP port is provided"))]
    NoUdpPortProvided,

    #[snafu(display("No SOCKS command is enabled"))]
    NoSocksCommandEnabled,
}
