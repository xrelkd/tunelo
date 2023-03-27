use std::{collections::HashSet, net::SocketAddr, sync::Arc};

use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
    sync::{mpsc, Mutex},
};

use crate::{
    authentication::AuthenticationManager,
    common::HostAddress,
    protocol::socks::SocksVersion,
    service::socks::{v4, v5, Error},
    transport::Transport,
};

pub struct Service<ClientStream, TransportStream> {
    service_v4: Option<v4::Service<ClientStream, TransportStream>>,
    service_v5: Option<v5::Service<ClientStream, TransportStream>>,
}

impl<ClientStream, TransportStream> Service<ClientStream, TransportStream>
where
    ClientStream: Unpin + AsyncRead + AsyncWrite,
    TransportStream: Unpin + AsyncRead + AsyncWrite,
{
    pub fn new(
        supported_versions: HashSet<SocksVersion>,
        transport: Arc<Transport<TransportStream>>,
        authentication_manager: Arc<Mutex<AuthenticationManager>>,
        enable_tcp_connect: bool,
        enable_tcp_bind: bool,
        udp_associate_stream_tx: Option<Mutex<mpsc::Sender<(ClientStream, HostAddress)>>>,
    ) -> Self {
        let service_v4 = if supported_versions.contains(&SocksVersion::V4) {
            tracing::info!("SOCKS4a is supported");
            Some(v4::Service::new(
                transport.clone(),
                authentication_manager.clone(),
                enable_tcp_connect,
                enable_tcp_bind,
            ))
        } else {
            None
        };

        let service_v5 = if supported_versions.contains(&SocksVersion::V5) {
            tracing::info!("SOCKS5 is supported");
            Some(v5::Service::new(
                transport,
                authentication_manager,
                enable_tcp_connect,
                enable_tcp_bind,
                udp_associate_stream_tx,
            ))
        } else {
            None
        };

        Self { service_v4, service_v5 }
    }

    pub async fn dispatch(
        &self,
        mut stream: ClientStream,
        peer_addr: SocketAddr,
    ) -> Result<(), Error> {
        match stream.read_u8().await {
            Ok(0x04) => match self.service_v4 {
                Some(ref service) => service.handle(stream, peer_addr).await,
                None => Err(Error::UnsupportedSocksVersion { version: SocksVersion::V4 }),
            },
            Ok(0x05) => match self.service_v5 {
                Some(ref service) => service.handle(stream, peer_addr).await,
                None => Err(Error::UnsupportedSocksVersion { version: SocksVersion::V5 }),
            },
            Ok(version) => {
                let _unused = stream.shutdown().await;
                Err(Error::InvalidSocksVersion { version })
            }
            Err(source) => {
                tracing::debug!(
                    "Failed to get SOCKS version from host: {}, error: {:?}",
                    peer_addr,
                    source
                );
                Err(Error::DetectSocksVersion { source, peer_addr })
            }
        }
    }

    #[allow(dead_code)]
    pub fn supported_versions(&self) -> Vec<SocksVersion> {
        let mut versions = Vec::new();
        if self.service_v4.is_some() {
            versions.push(SocksVersion::V4);
        }
        if self.service_v5.is_some() {
            versions.push(SocksVersion::V5);
        }
        versions
    }
}

#[cfg(test)]
mod tests {}
