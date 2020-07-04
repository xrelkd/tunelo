use std::{collections::HashSet, net::IpAddr, time::Duration};

use snafu::Snafu;
use structopt::StructOpt;

use tunelo::server::{http, socks};

#[derive(Debug, StructOpt)]
pub struct SocksOptions {
    #[structopt(long = "disable-socks4a", help = "Disable SOCKS4a support")]
    disable_socks4a: bool,

    #[structopt(long = "disable-socks5", help = "Disable SOCKS5 support")]
    disable_socks5: bool,

    #[structopt(long = "enable-tcp-connect", help = "Enable \"TCP Connect\" support")]
    enable_tcp_connect: bool,

    #[structopt(long = "enable-tcp-bind", help = "Enable \"TCP Bind\" support")]
    enable_tcp_bind: bool,

    #[structopt(long = "enable-udp-associate", help = "Enable \"UDP Associate\" support")]
    enable_udp_associate: bool,

    #[structopt(long = "connection-timeout", default_value = "20", help = "Connection timeout")]
    connection_timeout: u64,

    #[structopt(long = "ip", default_value = "127.0.0.1", help = "IP address to listen")]
    ip: IpAddr,

    #[structopt(long = "port", default_value = "3128", help = "Port number to listen")]
    port: u16,

    #[structopt(long = "udp-ports", help = "UDP ports to provide UDP associate service")]
    udp_ports: Vec<u16>,
}

impl std::convert::TryInto<socks::ServerOptions> for SocksOptions {
    type Error = Error;

    fn try_into(self) -> Result<socks::ServerOptions, Self::Error> {
        use tunelo::{
            protocol::socks::{SocksCommand, SocksVersion},
            server::socks::ServerOptions,
        };

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
                (true, true) => return Err(Error::NoUdpPortProvided),
            }

            if self.enable_tcp_bind {
                warn!("TCP bind is not supported yet");
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

#[derive(Debug, StructOpt)]
pub struct HttpOptions {
    #[structopt(long = "ip", default_value = "127.0.0.1", help = "IP address to listen")]
    ip: IpAddr,

    #[structopt(long = "port", default_value = "8118", help = "Port number to listen")]
    port: u16,
}

impl Into<http::ServerOptions> for HttpOptions {
    fn into(self) -> http::ServerOptions {
        let listen_address = self.ip;
        let listen_port = self.port;

        http::ServerOptions { listen_address, listen_port }
    }
}

#[derive(Debug, StructOpt)]
pub struct ProxyCheckerOptions {}

#[derive(Debug, StructOpt)]
pub struct ProxyChainOptions {}

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
}
