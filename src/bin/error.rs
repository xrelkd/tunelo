use snafu::Snafu;

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("Could not initialize tokio runtime, error: {}", source))]
    InitializeTokioRuntime { source: tokio::io::Error },

    #[snafu(display("Transport error, error: {}", source))]
    Transport { source: tunelo::transport::Error },

    #[snafu(display("Could not run SOCKs proxy server, error: {}", source))]
    RunSocksServer { source: crate::command::socks_server::Error },

    #[snafu(display("Could not run HTTP proxy server, error: {}", source))]
    RunHttpServer { source: crate::command::http_server::Error },

    #[snafu(display("Could not run proxy chain, error: {}", source))]
    RunProxyChain { source: crate::command::proxy_chain::Error },

    #[snafu(display("Could not multi proxy, error: {}", source))]
    RunMultiProxy { source: crate::command::multi_proxy::Error },
}

impl From<tunelo::transport::Error> for Error {
    fn from(source: tunelo::transport::Error) -> Error { Error::Transport { source } }
}

impl From<crate::command::http_server::Error> for Error {
    fn from(source: crate::command::http_server::Error) -> Error { Error::RunHttpServer { source } }
}

impl From<crate::command::socks_server::Error> for Error {
    fn from(source: crate::command::socks_server::Error) -> Error {
        Error::RunSocksServer { source }
    }
}

impl From<crate::command::proxy_chain::Error> for Error {
    fn from(source: crate::command::proxy_chain::Error) -> Error { Error::RunProxyChain { source } }
}

impl From<crate::command::multi_proxy::Error> for Error {
    fn from(source: crate::command::multi_proxy::Error) -> Error { Error::RunMultiProxy { source } }
}
