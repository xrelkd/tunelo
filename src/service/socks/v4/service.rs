use std::{
    collections::HashSet,
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
    sync::Arc,
};

use tokio::{
    io::{AsyncRead, AsyncWrite, AsyncWriteExt},
    sync::Mutex,
};

use crate::{
    authentication::AuthenticationManager,
    protocol::socks::v4::{Command, Reply, Request},
    service::socks::Error,
    transport::Transport,
};

pub struct Service<ClientStream, TransportStream> {
    supported_commands: HashSet<Command>,
    transport: Arc<Transport<TransportStream>>,
    _authentication_manager: Arc<Mutex<AuthenticationManager>>,
    _phantom: std::marker::PhantomData<ClientStream>,
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
    ) -> Service<ClientStream, TransportStream> {
        let supported_commands = {
            let mut commands = HashSet::new();
            if enable_tcp_connect {
                info!("SOCKS4: TCP Connect is supported.");
                commands.insert(Command::TcpConnect);
            }

            if enable_tcp_bind {
                info!("SOCKS4: TCP Bind is supported.");
                commands.insert(Command::TcpBind);
            }

            if commands.is_empty() {
                warn!("No SOCKS4 command is supported.");
            }
            commands
        };

        Service {
            supported_commands,
            transport,
            _authentication_manager: authentication_manager,
            _phantom: Default::default(),
        }
    }

    pub async fn handle(
        &self,
        mut stream: ClientStream,
        peer_addr: SocketAddr,
    ) -> Result<(), Error> {
        info!("Receive request from {}", peer_addr);

        let request = Request::from_reader(&mut stream).await?;

        if !self.supported_commands.contains(&request.command) {
            let reply = Reply::rejected(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0));
            let _ = stream.write(&reply.into_bytes()).await?;
            stream.shutdown().await?;
            return Err(Error::UnsupportedCommand);
        }

        match request.command {
            Command::TcpConnect => {
                let remote_host = request.destination_socket.as_ref();
                use crate::common::HostAddress;

                let (remote_socket, remote_addr) = match self.transport.connect(&remote_host).await
                {
                    Ok((socket, addr)) => {
                        info!("Remote host {} is connected", remote_host.to_string());
                        let remote_addr = match addr {
                            HostAddress::Socket(SocketAddr::V4(addr)) => addr,
                            HostAddress::Socket(_) | HostAddress::DomainName(..) => {
                                SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0)
                            }
                        };

                        (socket, remote_addr)
                    }
                    Err(err) => {
                        let reply = Reply::unreachable(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0));
                        let _ = stream.write(&reply.into_bytes()).await?;
                        stream.shutdown().await?;
                        return Err(err.into());
                    }
                };

                let reply = Reply::granted(remote_addr);
                let _ = stream.write(&reply.into_bytes()).await?;

                self.transport
                    .relay(
                        stream,
                        remote_socket,
                        Some(Box::new(move || {
                            info!("Remote host {} is disconnected", remote_addr);
                        })),
                    )
                    .await?;

                Ok(())
            }
            Command::TcpBind => {
                debug!("Unsupported SOCKS command, close connection: {:?}", peer_addr);
                let _ = stream.shutdown().await?;
                Err(Error::UnsupportedCommand)
            }
        }
    }
}

#[cfg(test)]
mod tests {}
