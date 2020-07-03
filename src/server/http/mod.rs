use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;

use futures::FutureExt;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;

use crate::{
    authentication::AuthenticationManager,
    service::http::{Error, Service},
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
    tcp_address: SocketAddr,

    transport: Arc<Transport<TcpStream>>,
    authentication_manager: Arc<Mutex<AuthenticationManager>>,
}

impl Server {
    pub fn new(
        config: ServerConfig,
        transport: Arc<Transport<TcpStream>>,
        authentication_manager: Arc<Mutex<AuthenticationManager>>,
    ) -> Server {
        let tcp_address = SocketAddr::new(config.listen_address, config.listen_port);

        Server { tcp_address, transport, authentication_manager }
    }

    pub async fn serve_with_shutdown<F: std::future::Future<Output = ()>>(
        self,
        shutdown_signal: F,
    ) -> Result<(), Error> {
        let mut tcp_listener = TcpListener::bind(self.tcp_address).await?;
        info!("Starting HTTP proxy server at {}", self.tcp_address);

        let service = Arc::new(Service::new(self.transport, self.authentication_manager));

        let shutdown = shutdown_signal.fuse();
        futures::pin_mut!(shutdown);

        loop {
            let stream = futures::select! {
                stream = tcp_listener.accept().fuse() => stream,
                _ = shutdown => {
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
