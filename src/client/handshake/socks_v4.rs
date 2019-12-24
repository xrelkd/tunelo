use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt};

use crate::client::handshake::{ClientHandshake, Error};
use crate::common::HostAddress;
use crate::protocol::socks::{
    v4::{Command, Reply, ReplyField, Request},
    Address,
};

impl<Stream> ClientHandshake<Stream>
where
    Stream: Unpin + Send + Sync + AsyncRead + AsyncWrite,
{
    async fn handshake_socks_v4(
        &mut self,
        command: Command,
        destination_socket: &HostAddress,
        id: Option<&[u8]>,
    ) -> Result<(), Error> {
        let id = match id {
            Some(id) => id.to_vec(),
            None => vec![],
        };

        let destination_socket = Address::from(destination_socket.clone());
        let req = Request { command, destination_socket, id: id.clone() };

        let _ = self.stream.write(&req.into_bytes()).await?;

        let reply = Reply::from_reader(&mut self.stream).await?;
        match reply.reply {
            ReplyField::Granted => Ok(()),
            ReplyField::Rejected => Err(Error::ProxyRejected),
            ReplyField::Unreachable => Err(Error::HostUnreachable),
            ReplyField::InvalidId => Err(Error::InvalidSocks4aId(id)),
        }
    }

    #[inline]
    pub async fn handshake_socks_v4_tcp_connect(
        &mut self,
        destination_socket: &HostAddress,
        id: Option<&[u8]>,
    ) -> Result<(), Error> {
        self.handshake_socks_v4(Command::TcpConnect, destination_socket, id).await
    }

    #[inline]
    pub async fn handshake_socks_v4_tcp_bind(
        &mut self,
        destination_socket: &HostAddress,
        id: Option<&[u8]>,
    ) -> Result<(), Error> {
        self.handshake_socks_v4(Command::TcpBind, destination_socket, id).await
    }
}
