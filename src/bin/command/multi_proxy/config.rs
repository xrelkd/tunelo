use std::{
    collections::HashSet,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::Path,
    str::FromStr,
    time::Duration,
};

pub use crate::error::Error;

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Config {
    pub proxy_servers: HashSet<ProxyServer>,

    pub socks_server: Option<SocksServer>,
    pub http_server: Option<HttpServer>,
}

impl Config {
    impl_config_load!(Config);

    pub fn enable_socks(&self) -> bool {
        self.proxy_servers.contains(&ProxyServer::Socks) && self.socks_server.is_some()
    }

    pub fn enable_http(&self) -> bool {
        self.proxy_servers.contains(&ProxyServer::Http) && self.http_server.is_some()
    }
}

impl Default for Config {
    fn default() -> Config {
        let proxy_servers = vec![ProxyServer::Http, ProxyServer::Socks].into_iter().collect();

        Config {
            proxy_servers,
            socks_server: Some(SocksServer::default()),
            http_server: Some(HttpServer::default()),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ProxyServer {
    Socks,
    Http,
}

impl FromStr for ProxyServer {
    type Err = Error;

    fn from_str(server: &str) -> Result<ProxyServer, Self::Err> {
        match server.to_lowercase().as_ref() {
            "socks" => Ok(ProxyServer::Socks),
            "http" => Ok(ProxyServer::Http),
            _ => Err(Error::InvalidProxyServer { server: server.to_owned() }),
        }
    }
}

impl ToString for ProxyServer {
    fn to_string(&self) -> String {
        match self {
            ProxyServer::Socks => "socks".to_owned(),
            ProxyServer::Http => "http".to_owned(),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct SocksServer {
    tcp_ip: IpAddr,
    tcp_port: u16,

    udp_ip: IpAddr,
    udp_ports: Vec<u16>,

    enable_socks4a: bool,
    enable_socks5: bool,

    enable_tcp_connect: bool,
    enable_tcp_bind: bool,
    enable_udp_associate: bool,

    connection_timeout: u64,
    tcp_keepalive: u64,
    udp_cache_expiry_duration: u64,
}

impl Default for SocksServer {
    fn default() -> SocksServer {
        SocksServer {
            tcp_ip: IpAddr::V4(Ipv4Addr::LOCALHOST),
            tcp_port: 3128,

            udp_ip: IpAddr::V4(Ipv4Addr::LOCALHOST),
            udp_ports: vec![3129],

            enable_socks4a: true,
            enable_socks5: true,

            enable_tcp_connect: true,
            enable_tcp_bind: false,
            enable_udp_associate: false,

            connection_timeout: 20,
            tcp_keepalive: 5,
            udp_cache_expiry_duration: 30,
        }
    }
}

impl Into<tunelo::server::socks::ServerOptions> for SocksServer {
    fn into(self) -> tunelo::server::socks::ServerOptions {
        use tunelo::protocol::socks::{SocksCommand, SocksVersion};

        let listen_address = self.tcp_ip;
        let listen_port = self.tcp_port;
        let udp_ports: HashSet<_> = self.udp_ports.into_iter().collect();

        let supported_versions = {
            let mut versions = HashSet::new();
            if self.enable_socks4a {
                versions.insert(SocksVersion::V4);
            }
            if self.enable_socks5 {
                versions.insert(SocksVersion::V5);
            }
            versions
        };

        let supported_commands = {
            let mut commands = HashSet::new();
            if self.enable_tcp_connect {
                commands.insert(SocksCommand::TcpConnect);
            }

            match (self.enable_udp_associate, udp_ports.is_empty()) {
                (false, _) => {}
                (true, true) => {}
                (true, false) => {
                    commands.insert(SocksCommand::UdpAssociate);
                }
            }

            if self.enable_tcp_bind {
                commands.insert(SocksCommand::TcpBind);
            }

            commands
        };

        tunelo::server::socks::ServerOptions {
            listen_address,
            listen_port,
            udp_ports,

            supported_versions,
            supported_commands,

            udp_cache_expiry_duration: Duration::from_secs(self.udp_cache_expiry_duration),
            connection_timeout: Duration::from_secs(self.connection_timeout),
            tcp_keepalive: Duration::from_secs(self.tcp_keepalive),
        }
    }
}

impl SocksServer {
    pub fn listen_socket(&self) -> SocketAddr { SocketAddr::new(self.tcp_ip, self.tcp_port) }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct HttpServer {
    host: IpAddr,
    port: u16,
}

impl Default for HttpServer {
    fn default() -> HttpServer { HttpServer { host: IpAddr::V4(Ipv4Addr::LOCALHOST), port: 8080 } }
}

impl Into<tunelo::server::http::ServerOptions> for HttpServer {
    fn into(self) -> tunelo::server::http::ServerOptions {
        let listen_address = self.host;
        let listen_port = self.port;
        tunelo::server::http::ServerOptions { listen_address, listen_port }
    }
}

impl HttpServer {
    pub fn listen_socket(&self) -> SocketAddr { SocketAddr::new(self.host, self.port) }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthenticationMethod {}

impl ToString for AuthenticationMethod {
    fn to_string(&self) -> String { String::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_load() {
        let path = {
            let mut p = std::env::temp_dir();
            p.push(format!(".test-{:?}", std::time::Instant::now()));
            p
        };

        let content = r#"
proxy_servers = ["socks", "http"]

[socks_server]
tcp_ip = "127.0.0.1"
tcp_port = 3128

udp_ip = "127.0.0.1"
udp_ports = [10001, 10002, 10003]

enable_socks4a = true
enable_socks5 = true

enable_tcp_connect = true
enable_tcp_bind = false
enable_udp_associate = true

connection_timeout = 10
tcp_keepalive = 10
udp_cache_expiry_duration = 10

[http_server]
host = "127.0.0.1"
port = 8118
"#;

        let config = Config {
            proxy_servers: vec![ProxyServer::Http, ProxyServer::Socks].into_iter().collect(),
            socks_server: Some(SocksServer {
                tcp_ip: "127.0.0.1".parse().unwrap(),
                tcp_port: 3128,

                udp_ip: "127.0.0.1".parse().unwrap(),
                udp_ports: vec![10001, 10002, 10003],

                enable_socks4a: true,
                enable_socks5: true,

                enable_tcp_connect: true,
                enable_tcp_bind: false,
                enable_udp_associate: true,

                connection_timeout: 10,
                tcp_keepalive: 10,
                udp_cache_expiry_duration: 10,
            }),
            http_server: Some(HttpServer { host: "127.0.0.1".parse().unwrap(), port: 8118 }),
        };
        std::fs::write(&path, &content).unwrap();
        assert_eq!(config, Config::load(&path).unwrap());
        std::fs::remove_file(&path).unwrap();
    }
}
