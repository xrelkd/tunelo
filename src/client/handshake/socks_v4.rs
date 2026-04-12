use snafu::ResultExt;
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt};

use crate::{
    client::handshake::{ClientHandshake, Error, error},
    common::HostAddress,
    protocol::socks::{
        Address,
        v4::{Command, Reply, ReplyField, Request},
    },
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
        let id = id.map_or_else(Vec::new, <[u8]>::to_vec);

        let destination_socket = Address::from(destination_socket.clone());
        let req = Request { command, destination_socket, id: id.clone() };

        let _ = self.stream.write(&req.into_bytes()).await.context(error::WriteStreamSnafu)?;

        let reply =
            Reply::from_reader(&mut self.stream).await.context(error::ParseSocks4ReplySnafu)?;
        match reply.reply {
            ReplyField::Granted => Ok(()),
            ReplyField::Rejected => Err(Error::ProxyRejected),
            ReplyField::Unreachable => Err(Error::HostUnreachable),
            ReplyField::InvalidId => Err(Error::InvalidSocks4aId { id }),
        }
    }

    /// Initiates a SOCKS4 TCP connect handshake.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Cannot write to stream ([`Error::WriteStream`])
    /// - Proxy rejected the request ([`Error::ProxyRejected`])
    /// - Host unreachable ([`Error::HostUnreachable`])
    /// - Invalid ID ([`Error::InvalidSocks4aId`])
    #[inline]
    pub async fn handshake_socks_v4_tcp_connect(
        &mut self,
        destination_socket: &HostAddress,
        id: Option<&[u8]>,
    ) -> Result<(), Error> {
        self.handshake_socks_v4(Command::TcpConnect, destination_socket, id).await
    }

    /// Initiates a SOCKS4 TCP bind handshake.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Cannot write to stream ([`Error::WriteStream`])
    /// - Proxy rejected the request ([`Error::ProxyRejected`])
    /// - Host unreachable ([`Error::HostUnreachable`])
    /// - Invalid ID ([`Error::InvalidSocks4aId`])
    #[inline]
    pub async fn handshake_socks_v4_tcp_bind(
        &mut self,
        destination_socket: &HostAddress,
        id: Option<&[u8]>,
    ) -> Result<(), Error> {
        self.handshake_socks_v4(Command::TcpBind, destination_socket, id).await
    }
}
