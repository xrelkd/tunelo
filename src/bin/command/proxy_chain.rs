use std::{
    collections::HashSet,
    future::Future,
    net::{IpAddr, Ipv4Addr},
    path::{Path, PathBuf},
    pin::Pin,
    sync::Arc,
    time::Duration,
};

use futures::future::join_all;
use snafu::Snafu;
use structopt::StructOpt;
use tokio::sync::Mutex;

use tunelo::{
    authentication::AuthenticationManager,
    common::{ProxyHost, ProxyStrategy},
    filter::DefaultFilter,
    server::{http, socks},
    transport::{Resolver, Transport},
};

use crate::{shutdown, signal_handler};

pub async fn run<P: AsRef<Path>>(
    resolver: Arc<dyn Resolver>,
    options: Options,
    config_file: Option<P>,
) -> Result<(), crate::error::Error> {
    let config = match config_file {
        Some(path) => Config::load(path)?.merge(options),
        None => Config::default().merge(options),
    };

    let socks_opts = if config.enable_socks4a || config.enable_socks5 {
        use tunelo::protocol::socks::{SocksCommand, SocksVersion};

        let supported_versions = {
            let mut v = HashSet::new();
            if config.enable_socks4a {
                v.insert(SocksVersion::V4);
            }
            if config.enable_socks5 {
                v.insert(SocksVersion::V5);
            }
            v
        };

        let supported_commands = vec![SocksCommand::TcpConnect].into_iter().collect();

        let listen_address = config.socks_ip.ok_or(Error::NoSocksListenAddress)?;
        let listen_port = config.socks_port.ok_or(Error::NoSocksListenPort)?;

        Some(socks::ServerOptions {
            supported_versions,
            supported_commands,
            listen_address,
            listen_port,
            udp_ports: HashSet::new(),
            connection_timeout: Duration::from_secs(10),
            tcp_keepalive: Duration::from_secs(10),
            udp_cache_expiry_duration: Duration::from_secs(10),
        })
    } else {
        None
    };

    let http_opts = if config.enable_http {
        let listen_address = config.http_ip.ok_or(Error::NoHttpListenAddress)?;
        let listen_port = config.http_port.ok_or(Error::NoHttpListenPort)?;
        Some(http::ServerOptions { listen_address, listen_port })
    } else {
        None
    };

    let proxy_strategy = {
        let chain = match (config.proxy_chain, config.proxy_chain_file) {
            (Some(chain), _) => chain,
            (_, Some(file)) => load_proxy_chain_file(file)?,
            (None, None) => return Err(Error::NoProxyChain)?,
        };

        let chain = ProxyStrategy::Chained(chain);
        info!("Proxy chain: {}", chain);
        Arc::new(chain)
    };

    let filter = {
        let mut f = DefaultFilter::deny_list();
        socks_opts.as_ref().map(|config| f.add_socket(config.listen_socket()));
        http_opts.as_ref().map(|config| f.add_socket(config.listen_socket()));
        Arc::new(f)
    };

    let transport = Arc::new(Transport::proxy(resolver, filter, proxy_strategy)?);
    let authentication_manager = Arc::new(Mutex::new(AuthenticationManager::new()));

    let (shutdown_sender, mut shutdown_receiver) = shutdown::new();

    let mut futs: Vec<Pin<Box<dyn Future<Output = Result<(), Error>>>>> = Vec::new();

    if let Some(opts) = socks_opts {
        let socks_serve = {
            let mut shutdown_receiver = shutdown_sender.subscribe();
            let server =
                socks::Server::new(opts, transport.clone(), authentication_manager.clone());

            let signal = async move {
                let _ = shutdown_receiver.wait().await;
            };
            Box::pin(async {
                Ok(server
                    .serve_with_shutdown(signal)
                    .await
                    .map_err(|source| Error::RunSocksServer { source })?)
            })
        };
        futs.push(socks_serve);
    }

    if let Some(opts) = http_opts {
        let http_serve = {
            let server = http::Server::new(opts, transport, authentication_manager);

            let signal = async move {
                let _ = shutdown_receiver.wait().await;
            };
            Box::pin(async {
                Ok(server
                    .serve_with_shutdown(signal)
                    .await
                    .map_err(|source| Error::RunHttpServer { source })?)
            })
        };

        futs.push(http_serve);
    }

    signal_handler::start(Box::new(move || {
        let _ = shutdown_sender.shutdown();
    }));

    let handle = join_all(futs).await;
    let errors: Vec<_> = handle.into_iter().filter_map(Result::err).collect();
    if !errors.is_empty() {
        return Err(Error::ErrorCollection { errors })?;
    }

    Ok(())
}

pub fn load_proxy_chain_file<P: AsRef<Path>>(file_path: P) -> Result<Vec<ProxyHost>, Error> {
    let file =
        std::fs::File::open(&file_path).map_err(|source| Error::LoadProxyHostFile { source })?;
    let chain =
        serde_json::from_reader(&file).map_err(|source| Error::ParseProxyHost { source })?;
    Ok(chain)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    enable_socks4a: bool,
    enable_socks5: bool,
    enable_http: bool,
    socks_ip: Option<IpAddr>,
    socks_port: Option<u16>,
    http_ip: Option<IpAddr>,
    http_port: Option<u16>,
    proxy_chain_file: Option<PathBuf>,
    proxy_chain: Option<Vec<ProxyHost>>,
}

impl Config {
    impl_config_load!(Config);

    fn merge(mut self, opts: Options) -> Config {
        let Options {
            enable_socks4a,
            enable_socks5,
            enable_http,
            socks_ip,
            socks_port,
            http_ip,
            http_port,
            proxy_chain_file,
            proxy_chain,
        } = opts;

        macro_rules! merge_option {
            ($config:ident, $opt:ident) => {
                if $opt.is_some() {
                    $config.$opt = $opt;
                }
            };
        }

        merge_bool_field!(self, enable_socks4a);
        merge_bool_field!(self, enable_socks5);
        merge_bool_field!(self, enable_http);
        merge_option!(self, socks_ip);
        merge_option!(self, socks_port);
        merge_option!(self, http_ip);
        merge_option!(self, http_port);
        merge_option!(self, proxy_chain_file);
        merge_option!(self, proxy_chain);

        self
    }
}

impl Default for Config {
    fn default() -> Config {
        Config {
            enable_socks4a: true,
            enable_socks5: true,
            enable_http: true,
            socks_ip: Some(IpAddr::V4(Ipv4Addr::LOCALHOST)),
            socks_port: Some(3128),
            http_ip: Some(IpAddr::V4(Ipv4Addr::LOCALHOST)),
            http_port: Some(8118),
            proxy_chain_file: None,
            proxy_chain: None,
        }
    }
}

#[derive(Debug, StructOpt)]
pub struct Options {
    #[structopt(long = "enable-socks4a")]
    enable_socks4a: bool,

    #[structopt(long = "enable-socks5")]
    enable_socks5: bool,

    #[structopt(long = "enable-http")]
    enable_http: bool,

    #[structopt(long = "socks-ip")]
    socks_ip: Option<IpAddr>,

    #[structopt(long = "socks-port")]
    socks_port: Option<u16>,

    #[structopt(long = "http-ip")]
    http_ip: Option<IpAddr>,

    #[structopt(long = "http-port")]
    http_port: Option<u16>,

    #[structopt(long = "proxy-chain-file")]
    proxy_chain_file: Option<PathBuf>,

    #[structopt(long = "proxy-chain")]
    proxy_chain: Option<Vec<ProxyHost>>,
}

#[derive(Debug, Snafu)]
pub enum Error {
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

    #[snafu(display("Miss SOCKS listen address"))]
    NoSocksListenAddress,

    #[snafu(display("Miss SOCKS listen port"))]
    NoSocksListenPort,

    #[snafu(display("Miss HTTP listen address"))]
    NoHttpListenAddress,

    #[snafu(display("Miss HTTP listen port"))]
    NoHttpListenPort,

    #[snafu(display("Could not parse ProxyHost, error: {}", source))]
    ParseProxyHost {
        source: serde_json::Error,
    },

    #[snafu(display("Could not load ProxyHost file, error: {}", source))]
    LoadProxyHostFile {
        source: std::io::Error,
    },
}
