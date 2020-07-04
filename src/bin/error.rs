use std::path::PathBuf;

use snafu::Snafu;

use crate::{command::options, config::ConfigError};

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("Options error: {}", source))]
    OptionsError {
        source: options::OptionsError,
    },

    #[snafu(display("Could not read configuration from {:?}, error: {}", file_path.display(), source))]
    ReadConfigFile {
        file_path: PathBuf,
        source: ConfigError,
    },

    #[snafu(display("Could not initialize tokio runtime, error: {}", source))]
    InitializeTokioRuntime {
        source: tokio::io::Error,
    },

    #[snafu(display("Transport error, error: {}", source))]
    Transport {
        source: tunelo::transport::Error,
    },

    #[snafu(display("Could not run SOCKs proxy service, error: {}", source))]
    RunSocksService {
        source: tunelo::service::socks::Error,
    },

    #[snafu(display("Could not run HTTP proxy service, error: {}", source))]
    RunHttpService {
        source: tunelo::service::http::Error,
    },

    ErrorCollection {
        errors: Vec<Error>,
    },
}

impl From<options::OptionsError> for Error {
    fn from(source: options::OptionsError) -> Error { Error::OptionsError { source } }
}

impl From<tunelo::transport::Error> for Error {
    fn from(source: tunelo::transport::Error) -> Error { Error::Transport { source } }
}

impl From<tunelo::service::http::Error> for Error {
    fn from(source: tunelo::service::http::Error) -> Error { Error::RunHttpService { source } }
}

impl From<tunelo::service::socks::Error> for Error {
    fn from(source: tunelo::service::socks::Error) -> Error { Error::RunSocksService { source } }
}
