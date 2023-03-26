use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::Arc,
};

use futures::FutureExt;
use snafu::ResultExt;
use tokio::{
    net::{TcpListener, TcpStream},
    sync::Mutex,
};

use crate::{
    authentication::AuthenticationManager,
    server::error::{self, Error},
    service::http::Service,
    transport::Transport,
};

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ServerOptions {
    pub listen_address: IpAddr,
    pub listen_port: u16,
}

impl Default for ServerOptions {
    fn default() -> ServerOptions {
        ServerOptions { listen_address: IpAddr::V4(Ipv4Addr::LOCALHOST), listen_port: 8118 }
    }
}

impl ServerOptions {
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
        config: ServerOptions,
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
        let tcp_listener =
            TcpListener::bind(self.tcp_address).await.context(error::BindTcpListenerSnafu)?;
        tracing::info!("Starting HTTP proxy server at {}", self.tcp_address);

        let service = Arc::new(Service::new(self.transport, self.authentication_manager));

        let shutdown = shutdown_signal.fuse();
        futures::pin_mut!(shutdown);

        loop {
            let stream = futures::select! {
                stream = tcp_listener.accept().fuse() => stream,
                _ = shutdown => {
                    tracing::info!("Stopping HTTP server");
                    break;
                },
            };

            match stream {
                Ok((socket, socket_addr)) => {
                    let service = service.clone();
                    tokio::spawn(async move {
                        let _n = service.handle(socket, socket_addr).await;
                    });
                }
                Err(source) => {
                    let err = Error::AcceptTcpStream { source };
                    tracing::warn!("Server error: {}", err);
                }
            }
        }

        tracing::info!("HTTP Proxy Server stopped");
        Ok(())
    }
}
