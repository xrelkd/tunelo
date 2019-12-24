use std::collections::HashSet;
use std::net::IpAddr;
use std::str::FromStr;
use std::time::Duration;

use structopt::{clap::Shell as ClapShell, StructOpt};

use crate::exit_code;
use crate::http_server;
use crate::multi_server;
use crate::proxy_checker;
use crate::socks_server;

#[derive(Debug, StructOpt)]
pub enum Command {
    /// Shows current version
    Version,

    /// Shows shell completions
    Completions {
        shell: ClapShell,
    },

    SocksServer {
        /// Disable SOCKS4a support
        #[structopt(long = "disable-socks4a")]
        disable_socks4a: bool,

        /// Disable SOCKS5 support
        #[structopt(long = "disable-socks5")]
        disable_socks5: bool,

        /// Enable "TCP Connect" support
        #[structopt(long = "enable-tcp-connect")]
        enable_tcp_connect: bool,

        /// Enable "TCP Bind" support
        #[structopt(long = "enable-tcp-bind")]
        enable_tcp_bind: bool,

        /// Enable "UDP Associate" support
        #[structopt(long = "enable-udp-associate")]
        enable_udp_associate: bool,

        /// Connection timeout
        #[structopt(long = "connection-timeout", default_value = "20")]
        connection_timeout: u64,

        /// IP address to listen
        #[structopt(long = "ip", default_value = "127.0.0.1")]
        ip: String,

        /// Port number to listen
        #[structopt(long = "port", default_value = "3128")]
        port: u16,

        /// UDP ports to provide UDP associate service
        #[structopt(long = "udp-ports")]
        udp_ports: Vec<u16>,
    },
    ProxyChecker,
    HttpServer {
        /// IP address to listen
        #[structopt(long = "ip", default_value = "127.0.0.1")]
        ip: String,

        /// Port number to listen
        #[structopt(long = "port", default_value = "8118")]
        port: u16,
    },
    MultiServer {},
}

impl Command {
    #[inline]
    pub fn app_name() -> String {
        Command::clap().get_name().to_owned()
    }

    pub fn run(self) {
        match self {
            Command::Version => match Command::clap().write_version(&mut std::io::stdout()) {
                Ok(_) => std::process::exit(0),
                Err(err) => {
                    eprintln!("failed to print version, error: {}", err);
                    std::process::exit(1);
                }
            },
            Command::Completions { shell } => {
                let app_name = Command::app_name();
                Command::clap().gen_completions_to(app_name, shell, &mut std::io::stdout());
                std::process::exit(0);
            }
            Command::SocksServer {
                ip,
                port,
                udp_ports,
                disable_socks4a,
                disable_socks5,
                enable_tcp_connect,
                enable_udp_associate,
                enable_tcp_bind,
                connection_timeout,
            } => {
                use tunelo::protocol::socks::{SocksCommand, SocksVersion};
                use tunelo::server::socks::ServerConfig;

                let listen_address = match IpAddr::from_str(&ip) {
                    Ok(ip) => ip,
                    Err(_err) => {
                        error!("Failed to parse IP address: {}", ip);
                        std::process::exit(exit_code::EXIT_FAILURE);
                    }
                };

                let listen_port = port;
                let udp_ports: HashSet<_> = udp_ports.into_iter().collect();

                let supported_versions = {
                    let mut versions = HashSet::new();

                    if !disable_socks4a {
                        versions.insert(SocksVersion::V4);
                    }

                    if !disable_socks5 {
                        versions.insert(SocksVersion::V5);
                    }

                    if versions.is_empty() {
                        warn!("No SOCKS service is enabled");
                        std::process::exit(exit_code::EXIT_FAILURE);
                    }

                    versions
                };

                let supported_commands = {
                    let mut commands = HashSet::new();
                    if enable_tcp_connect {
                        commands.insert(SocksCommand::TcpConnect);
                    }

                    match (enable_udp_associate, udp_ports.is_empty()) {
                        (false, _) => {}
                        (true, false) => {
                            commands.insert(SocksCommand::UdpAssociate);
                        }
                        (true, true) => {
                            error!("UDP associate is enabled but no UDP port is provided");
                            std::process::exit(exit_code::EXIT_FAILURE);
                        }
                    }

                    if enable_tcp_bind {
                        warn!("TCP bind is not supported yet");
                        commands.insert(SocksCommand::TcpBind);
                    }

                    if commands.is_empty() {
                        warn!("No SOCKS command is enabled, try to enable some commands");
                        std::process::exit(exit_code::EXIT_FAILURE);
                    }

                    commands
                };

                let config = ServerConfig {
                    supported_versions,
                    supported_commands,
                    listen_address,
                    listen_port,
                    udp_ports,
                    udp_cache_expiry_duration: Duration::from_millis(30),
                    connection_timeout: Duration::from_secs(connection_timeout),
                    tcp_keepalive: Duration::from_secs(5),
                };

                std::process::exit(socks_server::run(config));
            }
            Command::ProxyChecker => {
                std::process::exit(proxy_checker::run());
            }
            Command::HttpServer { ip, port } => {
                use tunelo::server::http::ServerConfig;

                let listen_address = match IpAddr::from_str(&ip) {
                    Ok(ip) => ip,
                    Err(_err) => {
                        error!("Failed to parse IP address: {}", ip);
                        std::process::exit(exit_code::EXIT_FAILURE);
                    }
                };

                let config = ServerConfig { listen_address, listen_port: port };
                std::process::exit(http_server::run(config))
            }
            Command::MultiServer {} => {
                use tunelo::protocol::socks::{SocksCommand, SocksVersion};
                use tunelo::server::http::ServerConfig as HttpServerConfig;
                use tunelo::server::socks::ServerConfig as SocksServerConfig;

                let ip = "127.0.0.1";

                let listen_address = match IpAddr::from_str(&ip) {
                    Ok(ip) => ip,
                    Err(_err) => {
                        error!("Failed to parse IP address: {}", ip);
                        std::process::exit(exit_code::EXIT_FAILURE);
                    }
                };

                let socks_server_config = SocksServerConfig {
                    supported_versions: {
                        let mut versions = HashSet::new();
                        versions.insert(SocksVersion::V4);
                        versions.insert(SocksVersion::V5);
                        versions
                    },
                    supported_commands: {
                        let mut commands = HashSet::new();
                        commands.insert(SocksCommand::TcpConnect);
                        commands.insert(SocksCommand::TcpBind);
                        commands
                    },
                    listen_address,
                    listen_port: 3000,
                    udp_ports: {
                        let mut ports = HashSet::new();
                        ports.insert(43581);
                        ports.insert(13581);
                        ports
                    },
                    udp_cache_expiry_duration: Duration::from_millis(30),
                    connection_timeout: Duration::from_secs(30),
                    tcp_keepalive: Duration::from_secs(30),
                };
                let http_server_config = HttpServerConfig { listen_address, listen_port: 3001 };
                std::process::exit(multi_server::run(socks_server_config, http_server_config))
            }
        }
    }
}
