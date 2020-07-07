use tokio::io::{AsyncRead, AsyncWrite};

use crate::{
    client::handshake::{ClientHandshake, Error},
    common::HostAddress,
    protocol::socks::{
        v5::{
            Command, HandshakeReply, HandshakeRequest, Method, Reply, ReplyField, Request,
            UserPasswordHandshakeReply, UserPasswordHandshakeRequest, UserPasswordStatus,
            UserPasswordVersion,
        },
        Address,
    },
};

impl<Stream> ClientHandshake<Stream>
where
    Stream: Unpin + Send + Sync + AsyncRead + AsyncWrite,
{
    async fn handshake_socks_v5(
        &mut self,
        command: Command,
        destination_socket: &HostAddress,
        user_name: Option<&str>,
        password: Option<&str>,
    ) -> Result<HostAddress, Error> {
        use tokio::io::AsyncWriteExt;

        let method = if user_name.is_some() && password.is_some() {
            Method::UsernamePassword
        } else {
            Method::NoAuthentication
        };

        let handshake_request = HandshakeRequest::new(vec![method]);
        self.stream
            .write(&handshake_request.to_bytes())
            .await
            .map_err(|source| Error::WriteStream { source })?;

        let handshake_reply = HandshakeReply::from_reader(&mut self.stream)
            .await
            .map_err(|source| Error::ParseSocks5Reply { source })?;

        if handshake_reply.method != method {
            return Err(Error::UnsupportedSocksMethod { method });
        }

        if method == Method::UsernamePassword {
            let user_name = user_name.clone().expect("user name is some; qed").as_bytes().to_vec();

            let password = password.clone().expect("password is some; qed").as_bytes().to_vec();

            let req = UserPasswordHandshakeRequest {
                version: UserPasswordVersion::V1,
                user_name: user_name.clone(),
                password: password.clone(),
            };
            self.stream
                .write(&req.into_bytes())
                .await
                .map_err(|source| Error::WriteStream { source })?;
            let reply = UserPasswordHandshakeReply::from_reader(&mut self.stream)
                .await
                .map_err(|source| Error::ParseSocks5Reply { source })?;
            if reply.status != UserPasswordStatus::Success {
                return Err(Error::AccessDenied { user_name, password });
            }
        }

        let destination_socket = Address::from(destination_socket.clone());
        let req = Request { command, destination_socket };

        let _ = self
            .stream
            .write(&req.into_bytes())
            .await
            .map_err(|source| Error::WriteStream { source })?;

        let reply = Reply::from_reader(&mut self.stream)
            .await
            .map_err(|source| Error::ParseSocks5Reply { source })?;
        if reply.reply != ReplyField::Success {
            return Err(Error::HostUnreachable);
        }

        Ok(reply.bind_socket.into())
    }

    #[inline]
    pub async fn handshake_socks_v5_tcp_connect(
        &mut self,
        destination_socket: &HostAddress,
        user_name: Option<&str>,
        password: Option<&str>,
    ) -> Result<HostAddress, Error> {
        self.handshake_socks_v5(Command::TcpConnect, destination_socket, user_name, password).await
    }

    #[inline]
    pub async fn handshake_socks_v5_udp_associate(
        &mut self,
        destination_socket: &HostAddress,
        user_name: Option<&str>,
        password: Option<&str>,
    ) -> Result<HostAddress, Error> {
        self.handshake_socks_v5(Command::UdpAssociate, destination_socket, user_name, password)
            .await
    }

    #[inline]
    pub async fn handshake_socks_v5_tcp_bind(
        &mut self,
        destination_socket: &HostAddress,
        user_name: Option<&str>,
        password: Option<&str>,
    ) -> Result<HostAddress, Error> {
        self.handshake_socks_v5(Command::TcpBind, destination_socket, user_name, password).await
    }
}
