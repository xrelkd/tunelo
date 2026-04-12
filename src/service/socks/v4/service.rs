use std::{
    collections::HashSet,
    marker::PhantomData,
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
    sync::Arc,
};

use snafu::ResultExt;
use tokio::{
    io::{AsyncRead, AsyncWrite, AsyncWriteExt},
    sync::Mutex,
};

use crate::{
    authentication::AuthenticationManager,
    common::HostAddress,
    protocol::socks::v4::{Command, Reply, Request},
    service::socks::{Error, error},
    transport::Transport,
};

pub struct Service<ClientStream, TransportStream> {
    supported_commands: HashSet<Command>,
    transport: Arc<Transport<TransportStream>>,
    _authentication_manager: Arc<Mutex<AuthenticationManager>>,
    _phantom: PhantomData<ClientStream>,
}

impl<ClientStream, TransportStream> Service<ClientStream, TransportStream>
where
    ClientStream: Unpin + AsyncRead + AsyncWrite,
    TransportStream: Unpin + AsyncRead + AsyncWrite,
{
    pub fn new(
        transport: Arc<Transport<TransportStream>>,
        authentication_manager: Arc<Mutex<AuthenticationManager>>,
        enable_tcp_connect: bool,
        enable_tcp_bind: bool,
    ) -> Self {
        let supported_commands = {
            let mut commands = HashSet::new();
            if enable_tcp_connect {
                tracing::info!("SOCKS4: TCP Connect is supported.");
                let _unused = commands.insert(Command::TcpConnect);
            }

            if enable_tcp_bind {
                tracing::info!("SOCKS4: TCP Bind is supported.");
                let _unused = commands.insert(Command::TcpBind);
            }

            if commands.is_empty() {
                tracing::warn!("No SOCKS4 command is supported.");
            }
            commands
        };

        Self {
            supported_commands,
            transport,
            _authentication_manager: authentication_manager,
            _phantom: PhantomData,
        }
    }

    /// Handles a SOCKS4 connection request.
    ///
    /// # Errors
    /// Returns an error if the request parsing fails or connection to the
    /// remote host fails.
    #[expect(
        clippy::future_not_send,
        reason = "Service is designed for single-threaded execution; stream is not Send"
    )]
    pub async fn handle(
        &self,
        mut stream: ClientStream,
        peer_addr: SocketAddr,
    ) -> Result<(), Error> {
        tracing::info!("Receive request from {}", peer_addr);

        let request = Request::from_reader(&mut stream).await.context(error::ParseRequestSnafu)?;

        if !self.supported_commands.contains(&request.command) {
            let reply = Reply::rejected(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0));
            let _ = stream.write(&reply.into_bytes()).await.context(error::WriteStreamSnafu)?;
            stream.shutdown().await.context(error::ShutdownSnafu)?;
            return Err(Error::UnsupportedCommand { command: request.command.into() });
        }

        match request.command {
            Command::TcpConnect => {
                let remote_host = request.destination_socket.as_ref();

                let (remote_socket, remote_addr) = match self.transport.connect(remote_host).await {
                    Ok((socket, addr)) => {
                        tracing::info!("Remote host {} is connected", remote_host.to_string());
                        let remote_addr = match addr {
                            HostAddress::Socket(SocketAddr::V4(addr)) => addr,
                            HostAddress::Socket(_) | HostAddress::DomainName(..) => {
                                SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0)
                            }
                        };

                        (socket, remote_addr)
                    }
                    Err(source) => {
                        let reply = Reply::unreachable(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0));
                        let _ = stream
                            .write(&reply.into_bytes())
                            .await
                            .context(error::WriteStreamSnafu)?;
                        stream.shutdown().await.context(error::ShutdownSnafu)?;

                        return Err(Error::ConnectRemoteHost { source, host: remote_host.clone() });
                    }
                };

                let reply = Reply::granted(remote_addr);
                let _ = stream.write(&reply.into_bytes()).await.context(error::WriteStreamSnafu)?;

                self.transport
                    .relay(
                        stream,
                        remote_socket,
                        Some(Box::new(move || {
                            tracing::info!("Remote host {} is disconnected", remote_addr);
                        })),
                    )
                    .await
                    .context(error::RelayStreamSnafu)?;

                Ok(())
            }
            Command::TcpBind => {
                tracing::debug!("Unsupported SOCKS command, close connection: {:?}", peer_addr);
                stream.shutdown().await.context(error::ShutdownSnafu)?;
                Err(Error::UnsupportedCommand { command: Command::TcpBind.into() })
            }
        }
    }
}

#[cfg(test)]
mod tests {}
