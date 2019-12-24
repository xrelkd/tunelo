use std::collections::HashSet;
use std::net::SocketAddr;
use std::sync::Arc;

use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::sync::{mpsc, Mutex};

use crate::authentication::AuthenticationManager;
use crate::common::HostAddress;
use crate::protocol::socks::{Error as ProtocolError, SocksVersion};
use crate::service::socks::{v4, v5, Error};
use crate::transport::Transport;

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
    ) -> Service<ClientStream, TransportStream> {
        let service_v4 = if supported_versions.contains(&SocksVersion::V4) {
            info!("SOCKS4a is supported");
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
            info!("SOCKS5 is supported");
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

        Service { service_v4, service_v5 }
    }

    pub async fn dispatch(
        &self,
        mut stream: ClientStream,
        peer_addr: SocketAddr,
    ) -> Result<(), Error> {
        match stream.read_u8().await {
            Ok(0x04) => match self.service_v4 {
                Some(ref service) => service.handle(stream, peer_addr).await,
                None => Err(Error::UnsupportedSocksVersion(SocksVersion::V4)),
            },
            Ok(0x05) => match self.service_v5 {
                Some(ref service) => service.handle(stream, peer_addr).await,
                None => Err(Error::UnsupportedSocksVersion(SocksVersion::V5)),
            },
            Ok(v) => {
                let _ = stream.shutdown().await;
                Err(ProtocolError::InvalidSocksVersion(v).into())
            }
            Err(err) => {
                debug!("Failed to get SOCKS version from host: {:?}, error: {:?}", peer_addr, err);
                Err(ProtocolError::BadRequest.into())
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
