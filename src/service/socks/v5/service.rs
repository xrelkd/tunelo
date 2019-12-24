use std::collections::HashSet;
use std::net::SocketAddr;
use std::sync::Arc;

use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt};
use tokio::sync::{mpsc, Mutex};

use crate::authentication::{Authentication, AuthenticationManager};
use crate::common::HostAddress;
use crate::protocol::socks::{
    v5::{
        Command, HandshakeReply, HandshakeRequest, Method, Reply, Request,
        UserPasswordHandshakeReply, UserPasswordHandshakeRequest,
    },
    Address,
};
use crate::service::socks::Error;
use crate::transport::Transport;

pub struct Service<ClientStream, TransportStream> {
    authentication_manager: Arc<Mutex<AuthenticationManager>>,
    transport: Arc<Transport<TransportStream>>,
    udp_associate_stream_tx: Option<Mutex<mpsc::Sender<(ClientStream, HostAddress)>>>,
    supported_commands: HashSet<Command>,
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
        udp_associate_stream_tx: Option<Mutex<mpsc::Sender<(ClientStream, HostAddress)>>>,
    ) -> Service<ClientStream, TransportStream> {
        let supported_commands = {
            let mut commands = HashSet::new();
            if enable_tcp_connect {
                info!("SOCKS5: TCP Connect is supported.");
                commands.insert(Command::TcpConnect);
            }

            if enable_tcp_bind {
                info!("SOCKS: TCP Bind is note supported yet.");
                // FIXME TCP Bind is not supported yet
                // info!("SOCKS: TCP Bind is supported.");
                // commands.insert(Command::TcpBind);
            }

            if udp_associate_stream_tx.is_some() {
                info!("SOCKS5: UDP Associate is supported.");
                commands.insert(Command::UdpAssociate);
            }

            if commands.is_empty() {
                warn!("No SOCKS5 command is supported.");
            }
            commands
        };

        Service { authentication_manager, transport, udp_associate_stream_tx, supported_commands }
    }

    #[inline]
    pub fn is_supported_command(&self, command: Command) -> bool {
        self.supported_commands.contains(&command)
    }

    pub async fn handle(
        &self,
        mut stream: ClientStream,
        client_addr: SocketAddr,
    ) -> Result<(), Error> {
        self.handshake(&mut stream, client_addr).await?;

        let request = {
            let req = Request::from_reader(&mut stream).await?;

            // check if we support this SOCKS5 command
            if !self.is_supported_command(req.command) {
                let reply = Reply::not_supported(req.address_type());

                debug!(
                    "Command {:?} is not supported, close connection {}",
                    req.command, client_addr,
                );
                let _ = stream.write(&reply.into_bytes()).await?;
                stream.flush().await?;
                stream.shutdown().await?;

                return Err(Error::UnsupportedCommand);
            }

            req
        };

        match request.command {
            Command::TcpConnect => {
                let remote_addr: &HostAddress = request.destination_socket.as_ref();

                let (remote_socket, remote_addr) = match self.transport.connect(&remote_addr).await
                {
                    Ok((socket, addr)) => {
                        info!("Remote host {} is connected", remote_addr.to_string());
                        (socket, addr)
                    }
                    Err(err) => {
                        let reply = Reply::unreachable(request.address_type());
                        let _ = stream.write(&reply.into_bytes()).await?;
                        stream.flush().await?;
                        stream.shutdown().await?;
                        return Err(err.into());
                    }
                };

                let reply = Reply::success(Address::empty_ipv4());
                let _ = stream.write(&reply.into_bytes()).await?;

                self.transport
                    .relay(
                        stream,
                        remote_socket,
                        Some(Box::new(move || {
                            info!("Remote host {} is disconnected", remote_addr.to_string());
                        })),
                    )
                    .await?;

                Ok(())
            }
            Command::UdpAssociate => match self.udp_associate_stream_tx {
                Some(ref tx) => {
                    let target_addr: HostAddress = request.destination_socket.into();
                    let _ = tx.lock().await.send((stream, target_addr)).await;
                    Ok(())
                }
                None => unreachable!(),
            },
            Command::TcpBind => {
                //
                todo!()
            }
        }
    }

    async fn handshake(
        &self,
        client: &mut ClientStream,
        client_addr: SocketAddr,
    ) -> Result<(), Error> {
        let req = HandshakeRequest::from_reader(client).await?;
        debug!("Received {:?}", req);

        let supported_method: Method =
            self.authentication_manager.lock().await.supported_method(&client_addr).into();

        if !req.contains_method(supported_method) {
            let reply = HandshakeReply::new(Method::NotAcceptable);
            client.write(&reply.into_bytes()).await?;

            return Err(Error::UnsupportedMethod);
        }

        let reply = HandshakeReply::new(supported_method);
        client.write(&reply.into_bytes()).await?;

        match supported_method {
            Method::NoAuthentication => {}
            Method::UsernamePassword => {
                let request = UserPasswordHandshakeRequest::from_reader(client).await?;

                // check authentication
                info!(
                    "Received authentication from user: {}",
                    String::from_utf8_lossy(&request.user_name).to_owned()
                );
                let auth_passed = {
                    let handler = self.authentication_manager.lock().await;
                    let auth = Authentication::UsernamePassword {
                        user_name: request.user_name.clone(),
                        password: request.password.clone(),
                    };
                    handler.authenticate(auth).await
                };

                if !auth_passed {
                    let reply = UserPasswordHandshakeReply::failure();
                    client.write(&reply.into_bytes()).await?;
                    client.flush().await?;

                    warn!(
                        "Invalid authentication from user: {}",
                        String::from_utf8_lossy(&request.user_name).to_owned()
                    );

                    client.shutdown().await?;
                    return Err(Error::AccessDenied {
                        user_name: request.user_name,
                        password: request.password,
                    });
                }

                let reply = UserPasswordHandshakeReply::success();
                client.write(&reply.into_bytes()).await?;
                client.flush().await?;
            }
            Method::GSSAPI => {
                // TODO
                client.shutdown().await?;
                return Err(Error::UnsupportedCommand);
            }
            Method::NotAcceptable => unreachable!(),
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {}
