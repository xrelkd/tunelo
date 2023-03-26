use std::{
    collections::HashSet,
    future::Future,
    net::{IpAddr, Ipv4Addr},
    path::{Path, PathBuf},
    pin::Pin,
    sync::Arc,
    time::Duration,
};

use clap::Args;
use futures::future::join_all;
use serde::{Deserialize, Serialize};
use snafu::ResultExt;
use tokio::sync::Mutex;

use tunelo::{
    authentication::AuthenticationManager,
    common::{ProxyHost, ProxyStrategy},
    filter::SimpleFilter,
    server::{http, socks},
    transport::{Resolver, Transport},
};

use crate::{
    error::{self, Error},
    shutdown, signal_handler,
};

pub async fn run<P: AsRef<Path>>(
    resolver: Arc<dyn Resolver>,
    options: Options,
    config_file: Option<P>,
) -> Result<(), Error> {
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
        let strategy = match (config.proxy_chain, config.proxy_chain_file) {
            (Some(chain), _) => ProxyStrategy::Chained(chain),
            (_, Some(file)) => ProxyChain::load(file)?.into(),
            (None, None) => return Err(Error::NoProxyChain),
        };

        tracing::info!("Proxy chain: {}", strategy);
        Arc::new(strategy)
    };

    let filter = {
        let mut f = SimpleFilter::deny_list();
        if let Some(config) = socks_opts.as_ref() {
            f.add_socket(config.listen_socket())
        }
        if let Some(config) = http_opts.as_ref() {
            f.add_socket(config.listen_socket())
        }
        Arc::new(f)
    };

    let transport = Arc::new(
        Transport::proxy(resolver, filter, proxy_strategy).context(error::CreateTransportSnafu)?,
    );
    let authentication_manager = Arc::new(Mutex::new(AuthenticationManager::new()));

    let (shutdown_sender, mut shutdown_receiver) = shutdown::new();

    type ServeFuture = Pin<Box<dyn Future<Output = Result<(), Error>>>>;
    let mut futs: Vec<ServeFuture> = Vec::new();

    if let Some(opts) = socks_opts {
        let socks_serve = {
            let mut shutdown_receiver = shutdown_sender.subscribe();
            let server =
                socks::Server::new(opts, transport.clone(), authentication_manager.clone());

            let signal = async move {
                shutdown_receiver.wait().await;
            };
            Box::pin(async {
                server.serve_with_shutdown(signal).await.context(error::RunSocksServerSnafu)
            })
        };

        futs.push(socks_serve);
    }

    if let Some(opts) = http_opts {
        let http_serve = {
            let server = http::Server::new(opts, transport, authentication_manager);

            let signal = async move {
                shutdown_receiver.wait().await;
            };
            Box::pin(async {
                server.serve_with_shutdown(signal).await.context(error::RunHttpServerSnafu)
            })
        };

        futs.push(http_serve);
    }

    if futs.is_empty() {
        return Err(Error::NoProxyServer);
    }

    signal_handler::start(Box::new(move || {
        shutdown_sender.shutdown();
    }));

    let handle = join_all(futs).await;
    let errors: Vec<_> = handle.into_iter().filter_map(Result::err).collect();
    if !errors.is_empty() {
        return Err(Error::Collection { errors });
    }

    Ok(())
}

#[derive(Clone, Debug, Deserialize, Eq, Serialize, PartialEq)]
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
            disable_socks4a,
            disable_socks5,
            disable_http,
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

        self.enable_socks4a = !disable_socks4a;
        self.enable_socks5 = !disable_socks5;
        self.enable_http = !disable_http;

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

#[derive(Args, Debug)]
pub struct Options {
    #[arg(long = "disable-socks4a")]
    disable_socks4a: bool,

    #[arg(long = "disable-socks5")]
    disable_socks5: bool,

    #[arg(long = "disable-http")]
    disable_http: bool,

    #[arg(long = "socks-ip")]
    socks_ip: Option<IpAddr>,

    #[arg(long = "socks-port")]
    socks_port: Option<u16>,

    #[arg(long = "http-ip")]
    http_ip: Option<IpAddr>,

    #[arg(long = "http-port")]
    http_port: Option<u16>,

    #[arg(long = "proxy-chain-file")]
    proxy_chain_file: Option<PathBuf>,

    #[arg(long = "proxy-chain")]
    proxy_chain: Option<Vec<ProxyHost>>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProxyChain {
    proxy_chain: Vec<ProxyHost>,
}

impl ProxyChain {
    pub fn from_json(json: &[u8]) -> Result<ProxyChain, Error> {
        serde_json::from_slice(json).context(error::ParseProxyChainJsonSnafu)
    }

    pub fn from_toml(toml: &[u8]) -> Result<ProxyChain, Error> {
        let content = String::from_utf8_lossy(toml);
        toml::from_str(content.to_string().as_str()).context(error::ParseProxyChainTomlSnafu)
    }

    pub fn load<P: AsRef<Path>>(file_path: P) -> Result<ProxyChain, Error> {
        let file_path = file_path.as_ref();
        match file_path.extension() {
            None => Err(Error::DetectProxyChainFormat { file_path: file_path.to_owned() }),
            Some(ext) => match ext.to_str() {
                Some("json") => ProxyChain::load_json_file(file_path),
                Some("toml") => ProxyChain::load_toml_file(file_path),
                Some(ext) => Err(Error::ProxyChainFormatNotSupported { format: ext.to_owned() }),
                None => Err(Error::DetectProxyChainFormat { file_path: file_path.to_owned() }),
            },
        }
    }

    pub fn load_json_file<P: AsRef<Path>>(file_path: P) -> Result<ProxyChain, Error> {
        let content = std::fs::read(&file_path).context(error::LoadProxyChainFileSnafu)?;
        Self::from_json(&content)
    }

    pub fn load_toml_file<P: AsRef<Path>>(file_path: P) -> Result<ProxyChain, Error> {
        let content = std::fs::read(&file_path).context(error::LoadProxyChainFileSnafu)?;
        Self::from_toml(&content)
    }
}

impl From<ProxyChain> for ProxyStrategy {
    fn from(val: ProxyChain) -> Self { ProxyStrategy::Chained(val.proxy_chain) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proxy_chain_from_json() {
        let json = r#"
{
  "proxyChain": [
    { "type": "socks5", "host": "127.99.0.1", "port": 3128 },
    { "type": "socks4a", "host": "127.99.0.2", "port": 3128 },
    { "type": "httpTunnel", "host": "127.99.0.3", "port": 1080 }
  ]
}
        "#;

        let chain = ProxyChain {
            proxy_chain: vec![
                ProxyHost::Socks5 {
                    host: "127.99.0.1".to_owned(),
                    port: 3128,
                    username: None,
                    password: None,
                },
                ProxyHost::Socks4a { host: "127.99.0.2".to_owned(), port: 3128, id: None },
                ProxyHost::HttpTunnel {
                    host: "127.99.0.3".to_owned(),
                    port: 1080,
                    username: None,
                    password: None,
                    user_agent: None,
                },
            ],
        };

        assert_eq!(ProxyChain::from_json(json.as_bytes()).unwrap(), chain);
    }

    #[test]
    fn proxy_chain_from_toml() {
        let toml = r#"
[[proxyChain]]
type = "socks5"
host = "127.99.0.1"
port = 3128

[[proxyChain]]
type = "socks4a"
host = "127.99.0.2"
port = 3128

[[proxyChain]]
type = "httpTunnel"
host = "127.99.0.3"
port = 1080
        "#;

        let chain = ProxyChain {
            proxy_chain: vec![
                ProxyHost::Socks5 {
                    host: "127.99.0.1".to_owned(),
                    port: 3128,
                    username: None,
                    password: None,
                },
                ProxyHost::Socks4a { host: "127.99.0.2".to_owned(), port: 3128, id: None },
                ProxyHost::HttpTunnel {
                    host: "127.99.0.3".to_owned(),
                    port: 1080,
                    username: None,
                    password: None,
                    user_agent: None,
                },
            ],
        };

        assert_eq!(ProxyChain::from_toml(toml.as_bytes()).unwrap(), chain);
    }

    #[test]
    fn config_from_toml() {
        let config = Config {
            enable_socks4a: true,
            enable_socks5: true,
            enable_http: true,
            socks_ip: Some("127.0.83.1".parse().unwrap()),
            socks_port: Some(3944),
            http_ip: Some("127.0.83.1".parse().unwrap()),
            http_port: Some(3293),
            proxy_chain_file: Some(PathBuf::from("/tmp/proxy_file.json")),
            proxy_chain: Some(vec![
                ProxyHost::Socks5 {
                    host: "127.99.0.1".to_owned(),
                    port: 3128,
                    username: None,
                    password: None,
                },
                ProxyHost::Socks4a { host: "127.99.0.2".to_owned(), port: 3128, id: None },
                ProxyHost::HttpTunnel {
                    host: "127.99.0.3".to_owned(),
                    port: 1080,
                    username: None,
                    password: None,
                    user_agent: None,
                },
            ]),
        };

        let toml = r#"
enable_socks4a = true
enable_socks5 = true
enable_http = true
socks_ip = "127.0.83.1"
socks_port = 3944
http_ip = "127.0.83.1"
http_port = 3293
proxy_chain_file = "/tmp/proxy_file.json"

[[proxy_chain]]
type = "socks5"
host = "127.99.0.1"
port = 3128

[[proxy_chain]]
type = "socks4a"
host = "127.99.0.2"
port = 3128

[[proxy_chain]]
type = "httpTunnel"
host = "127.99.0.3"
port = 1080
            "#;

        assert_eq!(Config::from_toml(toml).unwrap(), config);
    }
}
