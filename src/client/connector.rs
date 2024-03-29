use std::{sync::Arc, time::Duration};

use snafu::ResultExt;
use tokio::{
    io::{AsyncRead, AsyncWrite, AsyncWriteExt},
    net::TcpStream,
};

use crate::{
    client::{error, handshake::ClientHandshake, Error, ProxyStream},
    common::{HostAddress, ProxyHost, ProxyStrategy},
};

#[derive(Clone)]
pub struct ProxyConnector {
    strategy: Arc<ProxyStrategy>,
}

impl ProxyConnector {
    pub fn new(strategy: Arc<ProxyStrategy>) -> Result<Self, Error> { Ok(Self { strategy }) }

    pub async fn connect(&self, host: &HostAddress) -> Result<ProxyStream, Error> {
        let strategy = self.strategy.clone();
        let mut socket = Self::build_socket(&strategy).await?;

        let res = match self.strategy.as_ref() {
            ProxyStrategy::Single(proxy) => Self::handshake(&mut socket, proxy, host).await,
            ProxyStrategy::Chained(proxies) => match proxies.last() {
                Some(proxy_host) => Self::handshake(&mut socket, proxy_host, host).await,
                None => return Err(Error::NoProxyServiceProvided),
            },
        };

        if let Err(err) = res {
            socket.shutdown().await.context(error::ShutdownSnafu)?;
            return Err(err);
        }

        Ok(ProxyStream::from_raw(socket, strategy))
    }

    pub async fn probe_liveness(
        strategy: &ProxyStrategy,
        timeout: Option<Duration>,
    ) -> Result<bool, Error> {
        let mut socket = match timeout {
            Some(t) => tokio::time::timeout(t, Self::build_socket(strategy))
                .await
                .map_err(|_| Error::Timeout)??,
            None => Self::build_socket(strategy).await?,
        };
        socket.shutdown().await.context(error::ShutdownSnafu)?;
        Ok(true)
    }

    #[inline]
    async fn build_socket(strategy: &ProxyStrategy) -> Result<TcpStream, Error> {
        let socket = match strategy {
            ProxyStrategy::Single(proxy) => {
                let host = proxy.host_address();
                TcpStream::connect(host.to_string())
                    .await
                    .context(error::ConnectProxyServerSnafu)?
            }
            ProxyStrategy::Chained(proxies) => match proxies.len() {
                0 => return Err(Error::NoProxyServiceProvided),
                len => {
                    let proxy_host = proxies[0].host_address();
                    let mut socket = TcpStream::connect(proxy_host.to_string())
                        .await
                        .context(error::ConnectProxyServerSnafu)?;

                    for i in 0..(len - 1) {
                        let proxy_host = &proxies[i];
                        let target_host = proxies[i + 1].host_address();
                        if let Err(err) =
                            Self::handshake(&mut socket, proxy_host, &target_host).await
                        {
                            drop(socket.shutdown().await);
                            return Err(err);
                        };
                    }

                    socket
                }
            },
        };

        Ok(socket)
    }

    async fn handshake<Stream>(
        stream: &mut Stream,
        proxy_host: &ProxyHost,
        target_host: &HostAddress,
    ) -> Result<(), Error>
    where
        Stream: Unpin + Send + Sync + AsyncRead + AsyncWrite,
    {
        let mut handshake = ClientHandshake::new(stream);
        match proxy_host {
            ProxyHost::Socks4a { .. } => {
                handshake.handshake_socks_v4_tcp_connect(target_host, None).await?;
            }
            ProxyHost::Socks5 { username, password, .. } => {
                handshake
                    .handshake_socks_v5_tcp_connect(
                        target_host,
                        username.as_deref(),
                        password.as_deref(),
                    )
                    .await?;
            }
            ProxyHost::HttpTunnel { user_agent, .. } => {
                handshake.handshake_http_tunnel(target_host, user_agent.as_deref()).await?;
            }
        }

        Ok(())
    }
}
