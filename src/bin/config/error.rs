use std::path::PathBuf;

use snafu::Snafu;

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("Invalid proxy server: {}", server))]
    InvalidProxyServer { server: String },

    #[snafu(display("Read configuration file {:?}, error: {}", file_name.display(), source))]
    ReadConfigFile { source: std::io::Error, file_name: PathBuf },

    #[snafu(display("Deserialize configuration file {:?}, error: {}", file_name.display(), source))]
    DeserializeConfig { source: toml::de::Error, file_name: PathBuf },

    #[snafu(display("No UDP port is provided"))]
    NoUdpPortProvided,

    #[snafu(display("No SOCKS command is enabled"))]
    NoSocksCommandEnabled,
}
