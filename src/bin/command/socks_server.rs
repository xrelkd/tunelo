use std::{
    collections::HashSet,
    convert::TryInto,
    net::{IpAddr, Ipv4Addr},
    path::Path,
    sync::Arc,
    time::Duration,
};

use clap::Args;
use serde::{Deserialize, Serialize};
use snafu::ResultExt;
use tokio::sync::Mutex;

use tunelo::{
    authentication::AuthenticationManager,
    filter::SimpleFilter,
    server::socks::{self, Server, ServerOptions},
    transport::{Resolver, Transport},
};

use crate::{error, error::Error, shutdown, signal_handler};

pub async fn run<P: AsRef<Path>>(
    resolver: Arc<dyn Resolver>,
    options: Options,
    config_file: Option<P>,
) -> Result<(), Error> {
    let config = match config_file {
        Some(path) => Config::load(&path)?.merge(options),
        None => Config::default().merge(options),
    };
    let server_config: ServerOptions = config.try_into()?;

    let socks_server = {
        let filter = {
            let mut f = SimpleFilter::deny_list();
            f.add_socket(server_config.listen_socket());
            Arc::new(f)
        };

        let transport = Arc::new(Transport::direct(resolver, filter));
        let authentication_manager = Arc::new(Mutex::new(AuthenticationManager::new()));
        Server::new(server_config, transport, authentication_manager)
    };

    let (tx, mut rx) = shutdown::new();
    signal_handler::start(Box::new(move || {
        tx.shutdown();
    }));

    socks_server
        .serve_with_shutdown(async move {
            rx.wait().await;
        })
        .await
        .context(error::RunSocksServerSnafu)?;

    Ok(())
}

impl TryInto<socks::ServerOptions> for Config {
    type Error = Error;

    fn try_into(self) -> Result<socks::ServerOptions, Self::Error> {
        use tunelo::protocol::socks::{SocksCommand, SocksVersion};

        let listen_address = self.ip;

        let listen_port = self.port;
        let udp_ports: HashSet<_> = self.udp_ports.into_iter().collect();

        let supported_versions = {
            let mut versions = HashSet::new();

            if !self.disable_socks4a {
                versions.insert(SocksVersion::V4);
            }

            if !self.disable_socks5 {
                versions.insert(SocksVersion::V5);
            }

            if versions.is_empty() {
                return Err(Error::NoSocksServiceEnabled);
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
                (true, false) => {
                    commands.insert(SocksCommand::UdpAssociate);
                }
                (true, true) => return Err(Error::NoSocksUdpPort),
            }

            if self.enable_tcp_bind {
                tracing::warn!("TCP bind is not supported yet");
                commands.insert(SocksCommand::TcpBind);
            }

            if commands.is_empty() {
                return Err(Error::NoSocksCommandEnabled);
            }

            commands
        };

        Ok(ServerOptions {
            supported_versions,
            supported_commands,
            listen_address,
            listen_port,
            udp_ports,
            udp_cache_expiry_duration: Duration::from_millis(30),
            connection_timeout: Duration::from_secs(self.connection_timeout),
            tcp_keepalive: Duration::from_secs(5),
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    disable_socks4a: bool,
    disable_socks5: bool,
    enable_tcp_connect: bool,
    enable_tcp_bind: bool,
    enable_udp_associate: bool,
    connection_timeout: u64,
    ip: IpAddr,
    port: u16,
    udp_ports: Vec<u16>,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            disable_socks4a: false,
            disable_socks5: false,
            enable_tcp_connect: true,
            enable_tcp_bind: false,
            enable_udp_associate: true,
            connection_timeout: 20,
            ip: IpAddr::V4(Ipv4Addr::LOCALHOST),
            port: 3128,
            udp_ports: vec![3129],
        }
    }
}

impl Config {
    impl_config_load!(Config);

    pub fn merge(mut self, opts: Options) -> Config {
        let Options {
            mut disable_socks4a,
            mut disable_socks5,
            mut enable_tcp_connect,
            mut enable_udp_associate,
            mut enable_tcp_bind,
            mut connection_timeout,
            mut ip,
            mut port,
            mut udp_ports,
        } = opts;

        merge_option_field!(self, disable_socks4a);
        merge_option_field!(self, disable_socks5);
        merge_option_field!(self, enable_tcp_connect);
        merge_option_field!(self, enable_tcp_bind);
        merge_option_field!(self, enable_udp_associate);
        merge_option_field!(self, disable_socks4a);
        merge_option_field!(self, connection_timeout);
        merge_option_field!(self, ip);
        merge_option_field!(self, port);
        merge_option_field!(self, udp_ports);

        self
    }
}

#[derive(Args, Debug)]
pub struct Options {
    #[arg(long = "ip", help = "IP address to listen")]
    ip: Option<IpAddr>,

    #[arg(long = "port", help = "Port number to listen")]
    port: Option<u16>,

    #[arg(long = "disable-socks4a", help = "Disable SOCKS4a support")]
    disable_socks4a: Option<bool>,

    #[arg(long = "disable-socks5", help = "Disable SOCKS5 support")]
    disable_socks5: Option<bool>,

    #[arg(long = "enable-tcp-connect", help = "Enable \"TCP Connect\" support")]
    enable_tcp_connect: Option<bool>,

    #[arg(long = "enable-tcp-bind", help = "Enable \"TCP Bind\" support")]
    enable_tcp_bind: Option<bool>,

    #[arg(long = "enable-udp-associate", help = "Enable \"UDP Associate\" support")]
    enable_udp_associate: Option<bool>,

    #[arg(long = "udp-ports", help = "UDP ports to provide UDP associate service")]
    udp_ports: Option<Vec<u16>>,

    #[arg(long = "connection-timeout", help = "Connection timeout")]
    connection_timeout: Option<u64>,
}
