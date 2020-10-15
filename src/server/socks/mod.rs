use std::{
    collections::HashSet,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::Arc,
    time::Duration,
};

use tokio::{
    net::{TcpListener, TcpStream},
    sync::Mutex,
};

use futures::FutureExt;

use crate::{
    authentication::AuthenticationManager,
    common::utils::safe_duration,
    protocol::socks::{SocksCommand, SocksVersion},
    server::error::Error,
    service::socks::{v5::UdpAssociateManager, Service},
    transport::{MonitoredStream, TimedStream, Transport},
};

#[derive(Debug, Clone)]
pub struct ServerOptions {
    pub supported_versions: HashSet<SocksVersion>,
    pub supported_commands: HashSet<SocksCommand>,
    pub listen_address: IpAddr,
    pub listen_port: u16,
    pub udp_ports: HashSet<u16>,

    pub connection_timeout: Duration,
    pub tcp_keepalive: Duration,
    pub udp_cache_expiry_duration: Duration,
}

impl Default for ServerOptions {
    fn default() -> ServerOptions {
        ServerOptions {
            supported_versions: vec![SocksVersion::V4, SocksVersion::V5].into_iter().collect(),
            supported_commands: vec![SocksCommand::TcpConnect].into_iter().collect(),
            listen_address: IpAddr::V4(Ipv4Addr::LOCALHOST),
            listen_port: 3128,
            udp_ports: vec![3129].into_iter().collect(),
            connection_timeout: Duration::from_secs(10),
            tcp_keepalive: Duration::from_secs(10),
            udp_cache_expiry_duration: Duration::from_secs(10),
        }
    }
}

impl ServerOptions {
    pub fn listen_socket(&self) -> SocketAddr {
        SocketAddr::new(self.listen_address, self.listen_port)
    }
}

pub struct Server {
    authentication_manager: Arc<Mutex<AuthenticationManager>>,
    transport: Arc<Transport<TcpStream>>,

    supported_versions: HashSet<SocksVersion>,
    supported_commands: HashSet<SocksCommand>,

    tcp_address: SocketAddr,
    connection_timeout: Option<Duration>,
    #[allow(dead_code)]
    tcp_keepalive: Option<Duration>,

    udp_address: IpAddr,
    udp_ports: HashSet<u16>,

    // TODO
    #[allow(dead_code)]
    udp_timeout: Option<Duration>,
    #[allow(dead_code)]
    udp_session_time: Duration,

    udp_cache_expiry_duration: Duration,
}

impl Server {
    pub fn new(
        config: ServerOptions,
        transport: Arc<Transport<TcpStream>>,
        authentication_manager: Arc<Mutex<AuthenticationManager>>,
    ) -> Server {
        let tcp_address = config.listen_socket();
        let connection_timeout = safe_duration(config.connection_timeout);
        let tcp_keepalive = safe_duration(config.tcp_keepalive);
        let udp_cache_expiry_duration = config.udp_cache_expiry_duration;

        let udp_timeout = Some(Duration::from_secs(10));
        let udp_session_time = Duration::from_secs(10);

        Server {
            authentication_manager,
            transport,

            supported_versions: config.supported_versions,
            supported_commands: config.supported_commands,

            tcp_address,
            connection_timeout,
            tcp_keepalive,

            udp_address: config.listen_address,
            udp_ports: config.udp_ports,
            udp_timeout,
            udp_session_time,

            udp_cache_expiry_duration,
        }
    }

    pub async fn serve_with_shutdown<F: std::future::Future<Output = ()>>(
        self,
        shutdown_signal: F,
    ) -> Result<(), Error> {
        let mut tcp_listener = TcpListener::bind(self.tcp_address)
            .await
            .map_err(|source| Error::BindTcpListener { source })?;
        tracing::info!("Starting SOCKS server at {}", self.tcp_address);

        let (udp_associate_join_handle, udp_associate_stream_tx) =
            if self.supported_commands.contains(&SocksCommand::UdpAssociate) {
                let resolver = self.transport.resolver().clone();
                let udp_associate_manager = UdpAssociateManager::new(
                    self.udp_address,
                    self.udp_ports,
                    resolver,
                    self.udp_cache_expiry_duration,
                );

                let (tx, join_handle) = udp_associate_manager.serve();
                (Some(join_handle), Some(Mutex::new(tx)))
            } else {
                (None, None)
            };

        let enable_tcp_connect = self.supported_commands.contains(&SocksCommand::TcpConnect);
        let enable_tcp_bind = self.supported_commands.contains(&SocksCommand::TcpBind);
        let service = Arc::new(Service::new(
            self.supported_versions,
            self.transport.clone(),
            self.authentication_manager,
            enable_tcp_connect,
            enable_tcp_bind,
            udp_associate_stream_tx,
        ));

        let shutdown = shutdown_signal.fuse();
        futures::pin_mut!(shutdown);

        loop {
            let stream = futures::select! {
                stream = tcp_listener.accept().fuse() => stream,
                _ = shutdown => {
                    tracing::info!("Stopping SOCKS server");
                    break;
                },
            };

            match stream {
                Ok((socket, socket_addr)) => {
                    let service = service.clone();
                    let connection_timeout = self.connection_timeout;
                    let stat_monitor = self.transport.stat_monitor();
                    tokio::spawn(async move {
                        // let _ = socket.set_keepalive(Some(tcp_keepalive));
                        let socket = TimedStream::new(socket, connection_timeout);
                        let socket = MonitoredStream::new(socket, stat_monitor);
                        let _ = service.dispatch(socket, socket_addr).await;
                    });
                }
                Err(source) => {
                    let err = Error::AcceptTcpStream { source };
                    tracing::warn!("Server error: {:?}", err);
                }
            }
        }

        if let Some(join_handle) = udp_associate_join_handle {
            join_handle.shutdown_and_wait().await;
        }

        tracing::info!("SOCKS Server stopped");
        Ok(())
    }
}

#[cfg(test)]
mod tests {}
