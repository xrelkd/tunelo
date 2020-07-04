use snafu::Snafu;

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("No SOCKS service is enabled"))]
    NoSocksServiceEnabled,

    #[snafu(display("UDP associate is enabled but no UDP port is provided"))]
    NoUdpPortProvided,

    #[snafu(display("TCP bind is not supported yet"))]
    TcpBindNotSupported,

    #[snafu(display("No SOCKS command is enabled, try to enable some commands"))]
    NoSocksCommandEnabled,

    #[snafu(display("No proxy chain provided"))]
    NoProxyChain,

    #[snafu(display("Could not load proxy chain, error: {}", source))]
    LoadProxyChain { source: tunelo::common::ProxyHostError },

    #[snafu(display("Miss SOCKS listen address"))]
    NoSocksListenAddress,

    #[snafu(display("Miss SOCKS listen port"))]
    NoSocksListenPort,

    #[snafu(display("Miss HTTP listen address"))]
    NoHttpListenAddress,

    #[snafu(display("Miss HTTP listen port"))]
    NoHttpListenPort,
}
