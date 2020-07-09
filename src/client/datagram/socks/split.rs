use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use tokio::net::{
    udp::{RecvHalf as UdpRecvHalf, SendHalf as UdpSendHalf},
    UdpSocket,
};

use crate::{
    client::Error,
    common::HostAddress,
    protocol::socks::{v5::Datagram, Address},
};

pub struct RecvHalf {
    closed: Arc<AtomicBool>,
    socket_recv: UdpRecvHalf,
}

pub struct SendHalf {
    closed: Arc<AtomicBool>,
    socket_send: UdpSendHalf,
}

#[inline]
pub fn split(socket: UdpSocket, closed: Arc<AtomicBool>) -> (RecvHalf, SendHalf) {
    let (socket_recv, socket_send) = socket.split();
    let recv_half = RecvHalf { closed: closed.clone(), socket_recv };
    let send_half = SendHalf { closed, socket_send };
    (recv_half, send_half)
}

impl Drop for RecvHalf {
    fn drop(&mut self) { self.closed.store(true, Ordering::Release); }
}

impl RecvHalf {
    pub async fn recv_from(&mut self, buf: &mut [u8]) -> Result<(usize, HostAddress), Error> {
        if self.closed.load(Ordering::Acquire) {
            return Err(Error::DatagramClosed);
        }

        let mut header = vec![0u8; 3 + Address::max_len()];
        if header[0] != 0x00 || header[1] != 0x00 {
            return Err(Error::BadSocksReply);
        }

        if header[2] != 0x00 {
            return Err(Error::BadSocksReply);
        };

        let (address, n) =
            Address::from_bytes(&mut header[4..]).map_err(|_err| Error::BadSocksReply)?;

        let mut data_len = header.len() - n;
        buf.copy_from_slice(&header[n..]);
        data_len += self
            .socket_recv
            .recv(&mut buf[n + 1..])
            .await
            .map_err(|source| Error::RecvDatagram { source })?;

        Ok((data_len, address.into_inner()))
    }

    pub async fn recv_datagram(&mut self) -> Result<Datagram, Error> {
        use bytes::BytesMut;
        let mut buf = BytesMut::with_capacity(1024);
        let (_n, addr) = self.recv_from(&mut buf).await?;
        Ok(Datagram::new(0, addr.into(), buf))
    }
}

impl Drop for SendHalf {
    fn drop(&mut self) { self.closed.store(true, Ordering::Release); }
}

impl SendHalf {
    pub async fn send_to(&mut self, buf: &[u8], target_addr: &HostAddress) -> Result<usize, Error> {
        if self.closed.load(Ordering::Acquire) {
            return Err(Error::DatagramClosed);
        }

        let mut data = Vec::with_capacity(3 + Address::max_len() + buf.len());
        let mut wrt = std::io::Cursor::new(&mut data);
        let n = Datagram::serialize(&mut wrt, 0, target_addr, buf)
            .map_err(|source| Error::SerializeDatagram { source })?;
        Ok(self
            .socket_send
            .send(&mut data[..n])
            .await
            .map_err(|source| Error::SendDatagram { source })?)
    }
}
