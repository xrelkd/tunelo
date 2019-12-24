use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;

use futures::FutureExt;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;

use crate::{
    authentication::AuthenticationManager,
    service::http::{Error, Service},
    shutdown,
    transport::Transport,
};

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ServerConfig {
    pub listen_address: IpAddr,
    pub listen_port: u16,
}

impl ServerConfig {
    pub fn listen_socket(&self) -> SocketAddr {
        SocketAddr::new(self.listen_address, self.listen_port)
    }
}

pub struct Server {
    shutdown_slot: shutdown::ShutdownSlot,
    tcp_address: SocketAddr,

    transport: Arc<Transport<TcpStream>>,
    authentication_manager: Arc<Mutex<AuthenticationManager>>,
}

impl Server {
    pub fn new(
        config: ServerConfig,
        transport: Arc<Transport<TcpStream>>,
        authentication_manager: Arc<Mutex<AuthenticationManager>>,
    ) -> (Server, shutdown::ShutdownSignal) {
        let (shutdown_signal, shutdown_slot) = shutdown::shutdown_handle();
        let tcp_address = SocketAddr::new(config.listen_address, config.listen_port);

        (Server { tcp_address, shutdown_slot, transport, authentication_manager }, shutdown_signal)
    }

    pub fn with_shutdown_slot(
        config: ServerConfig,
        authentication_manager: Arc<Mutex<AuthenticationManager>>,
        transport: Arc<Transport<TcpStream>>,
        shutdown_slot: shutdown::ShutdownSlot,
    ) -> Server {
        let tcp_address = SocketAddr::new(config.listen_address, config.listen_port);

        Server { tcp_address, transport, shutdown_slot, authentication_manager }
    }

    pub async fn serve(self) -> Result<(), Error> {
        let mut tcp_listener = TcpListener::bind(self.tcp_address).await?;
        let shutdown_slot = self.shutdown_slot;
        info!("Starting HTTP proxy server at {}", self.tcp_address);

        let service = Arc::new(Service::new(self.transport, self.authentication_manager));

        futures::pin_mut!(shutdown_slot);

        loop {
            let stream = futures::select! {
                stream = tcp_listener.accept().fuse() => stream,
                _ = shutdown_slot.wait().fuse() => {
                    info!("Stopping HTTP server");
                    break;
                },
            };

            match stream {
                Ok((socket, socket_addr)) => {
                    let service = service.clone();
                    tokio::spawn(async move {
                        let _ = service.handle(socket, socket_addr).await;
                    });
                }
                Err(err) => {
                    warn!("Server error: {:?}", err);
                }
            }
        }

        info!("HTTP Proxy Server stopped");
        Ok(())
    }
}
