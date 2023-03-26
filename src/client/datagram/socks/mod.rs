use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

use snafu::ResultExt;
use tokio::{
    io::AsyncWriteExt,
    net::{TcpStream, UdpSocket},
    time,
};

use crate::{
    client::{error, handshake::*, Error},
    common::HostAddress,
    protocol::socks::v5::Datagram,
};

mod split;

use self::split::{RecvHalf, SendHalf};

pub struct Socks5Datagram {
    rx: RecvHalf,
    tx: SendHalf,
}

impl Socks5Datagram {
    pub async fn bind(
        proxy_addr: &HostAddress,
        user_name: Option<&str>,
        password: Option<&str>,
    ) -> Result<Socks5Datagram, Error> {
        let socket = {
            let addr = SocketAddr::new(IpAddr::from(Ipv4Addr::UNSPECIFIED), 0);
            UdpSocket::bind(addr).await.context(error::BindUdpSocketSnafu { addr })?
        };

        let (mut stream, endpoint_addr) = {
            let proxy_addr = proxy_addr.to_string();
            let port = socket.local_addr().context(error::GetLocalAddressSnafu)?.port();
            let destination_socket =
                HostAddress::from(SocketAddr::new(Ipv4Addr::UNSPECIFIED.into(), port));
            let stream =
                TcpStream::connect(proxy_addr).await.context(error::ConnectProxyServerSnafu)?;
            let mut handshake = ClientHandshake::new(stream);
            let bind_socket = handshake
                .handshake_socks_v5_udp_associate(&destination_socket, user_name, password)
                .await?;
            (handshake.into_inner(), bind_socket)
        };

        socket
            .connect(endpoint_addr.to_string())
            .await
            .context(error::ConnectUdpSocketSnafu { addr: endpoint_addr })?;

        let closed = Arc::new(AtomicBool::new(false));
        tokio::spawn({
            let closed = closed.clone();
            async move {
                while !closed.load(Ordering::Acquire) {
                    let buf = [0u8; 1];
                    match stream.write(&buf).await {
                        Ok(0) => closed.store(true, Ordering::Release),
                        Ok(_n) => time::delay_for(Duration::from_millis(500)).await,
                        Err(_err) => closed.store(true, Ordering::Release),
                    }
                }
            }
        });

        let (rx, tx) = split::split(socket, closed);
        Ok(Socks5Datagram { rx, tx })
    }

    #[inline]
    pub fn split(self) -> (RecvHalf, SendHalf) { (self.rx, self.tx) }

    #[inline]
    pub async fn recv_from(&mut self, buf: &mut [u8]) -> Result<(usize, HostAddress), Error> {
        self.rx.recv_from(buf).await
    }

    #[inline]
    pub async fn recv_datagram(&mut self) -> Result<Datagram, Error> {
        self.rx.recv_datagram().await
    }

    #[inline]
    pub async fn send_to(&mut self, buf: &[u8], target_addr: &HostAddress) -> Result<usize, Error> {
        self.tx.send_to(buf, target_addr).await
    }
}
