use std::{
    convert::TryFrom,
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
};

use snafu::ResultExt;
use tokio::io::{AsyncRead, AsyncReadExt};

use crate::{
    common::HostAddress,
    protocol::socks::{consts, error, Address, Error, SocksVersion},
};

#[derive(Debug, Hash, Clone, Copy, Eq, PartialEq)]
pub enum Command {
    TcpConnect,
    TcpBind,
}

impl From<Command> for u8 {
    fn from(val: Command) -> Self {
        match val {
            Command::TcpConnect => consts::SOCKS4_CMD_TCP_CONNECT,
            Command::TcpBind => consts::SOCKS4_CMD_TCP_BIND,
        }
    }
}

impl TryFrom<u8> for Command {
    type Error = Error;

    fn try_from(cmd: u8) -> Result<Self, Self::Error> {
        match cmd {
            consts::SOCKS4_CMD_TCP_CONNECT => Ok(Self::TcpConnect),
            consts::SOCKS4_CMD_TCP_BIND => Ok(Self::TcpBind),
            command => Err(Error::InvalidCommand { command }),
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum ReplyField {
    Granted,
    Rejected,
    Unreachable,
    InvalidId,
}

impl From<ReplyField> for u8 {
    fn from(val: ReplyField) -> Self {
        match val {
            ReplyField::Granted => consts::SOCKS4_REPLY_GRANTED,
            ReplyField::Rejected => consts::SOCKS4_REPLY_REJECTED,
            ReplyField::Unreachable => consts::SOCKS4_REPLY_UNREACHABLE,
            ReplyField::InvalidId => consts::SOCKS4_REPLY_INVALID_ID,
        }
    }
}

impl TryFrom<u8> for ReplyField {
    type Error = Error;

    fn try_from(reply: u8) -> Result<Self, Self::Error> {
        match reply {
            consts::SOCKS4_REPLY_GRANTED => Ok(Self::Granted),
            consts::SOCKS4_REPLY_REJECTED => Ok(Self::Rejected),
            consts::SOCKS4_REPLY_UNREACHABLE => Ok(Self::Unreachable),
            consts::SOCKS4_REPLY_INVALID_ID => Ok(Self::InvalidId),
            _ => Err(Error::BadReply),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Request {
    pub command: Command,
    pub destination_socket: Address,
    pub id: Vec<u8>,
}

impl Request {
    pub async fn from_reader<R>(rdr: &mut R) -> Result<Self, Error>
    where
        R: AsyncRead + Unpin,
    {
        let command = Command::try_from(rdr.read_u8().await.context(error::ReadStreamSnafu)?)?;
        let port = rdr.read_u16().await.context(error::ReadStreamSnafu)?;
        let mut ip_buf = [0u8; 4];
        let _ = rdr.read(&mut ip_buf).await.context(error::ReadStreamSnafu)?;

        let (id, host) = {
            let mut buf = [0u8; 128];
            let _ = rdr.read(&mut buf).await.context(error::ReadStreamSnafu)?;

            let parts: Vec<_> = buf.split(|ch| *ch == 0x00).collect();

            match parts.len() {
                0 => (Vec::new(), Vec::new()),
                1 => (parts[0].to_vec(), Vec::new()),
                _ => (parts[0].to_vec(), parts[1].to_vec()),
            }
        };

        let has_domain_name =
            ip_buf[0] == 0x00 && ip_buf[1] == 0x00 && ip_buf[2] == 0x00 && ip_buf[3] != 0x00;
        let destination_socket = if has_domain_name {
            Address::new_domain(&host, port)
        } else {
            let host = Ipv4Addr::from(ip_buf);
            Address::from(SocketAddrV4::new(host, port))
        };

        Ok(Self { command, destination_socket, id })
    }

    #[inline]
    #[must_use]
    pub fn into_bytes(self) -> Vec<u8> { self.to_bytes() }

    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> {
        // +----+----+----+----+----+----+----+----+----+----+....+----+
        // | VN | CD | DSTPORT |      DSTIP        | USERID       |NULL|
        // +----+----+----+----+----+----+----+----+----+----+....+----+
        //   1    1      2              4           variable       1
        //
        use byteorder::{BigEndian, WriteBytesExt};

        let mut buf = Vec::with_capacity(128);

        // version
        buf.push(SocksVersion::V4.into());

        // command
        buf.push(self.command.into());

        // port
        let _unused = buf.write_u16::<BigEndian>(self.destination_socket.port());

        // IP and user ID
        match self.destination_socket.as_ref() {
            HostAddress::Socket(socket) => match socket {
                SocketAddr::V4(socket) => {
                    buf.extend(&socket.ip().octets());
                    // user ID
                    buf.extend(&self.id);
                    buf.push(0x00);
                }
                SocketAddr::V6(_) => unreachable!(),
            },
            HostAddress::DomainName(host, _) => {
                buf.extend(&[0x00, 0x00, 0x00, 0x07]);

                // user ID
                buf.extend(&self.id);
                buf.push(0x00);

                // host
                buf.extend(host.as_bytes());
                buf.push(0x00);
            }
        };

        buf
    }
}

// +----+----+----+----+----+----+----+----+
// | VN | CD | DSTPORT |      DSTIP        |
// +----+----+----+----+----+----+----+----+
// | 1  | 1  |    2    |         4         |
// +---+-----+---------+----+----+----+----+

#[derive(Debug, Clone)]
pub struct Reply {
    pub reply: ReplyField,
    pub destination_socket: SocketAddrV4,
}

impl Reply {
    pub async fn from_reader<R>(rdr: &mut R) -> Result<Self, Error>
    where
        R: AsyncRead + Unpin,
    {
        if rdr.read_u8().await.context(error::ReadStreamSnafu)? != 0x00 {
            return Err(Error::BadReply);
        }

        let reply = ReplyField::try_from(rdr.read_u8().await.context(error::ReadStreamSnafu)?)?;
        let destination_socket = {
            let port = rdr.read_u16().await.context(error::ReadStreamSnafu)?;
            let mut ip = [0u8; 4];
            rdr.read(&mut ip).await.context(error::ReadStreamSnafu)?;
            SocketAddrV4::new(Ipv4Addr::from(ip), port)
        };

        Ok(Self { reply, destination_socket })
    }

    #[must_use]
    pub const fn granted(destination_socket: SocketAddrV4) -> Self {
        Self { reply: ReplyField::Granted, destination_socket }
    }

    #[must_use]
    pub const fn rejected(destination_socket: SocketAddrV4) -> Self {
        Self { reply: ReplyField::Rejected, destination_socket }
    }

    #[must_use]
    pub const fn unreachable(destination_socket: SocketAddrV4) -> Self {
        Self { reply: ReplyField::Unreachable, destination_socket }
    }

    #[allow(dead_code)]
    #[must_use]
    pub const fn invalid_id(destination_socket: SocketAddrV4) -> Self {
        Self { reply: ReplyField::InvalidId, destination_socket }
    }

    #[must_use]
    pub fn into_bytes(self) -> Vec<u8> { self.to_bytes() }

    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> {
        // +----+----+----+----+----+----+----+----+
        // | VN | CD | DSTPORT |      DSTIP        |
        // +----+----+----+----+----+----+----+----+
        //   1    1      2              4
        use byteorder::{BigEndian, WriteBytesExt};

        let mut buf = Vec::with_capacity(8);

        // version
        buf.push(0x00);

        // result code
        buf.push(self.reply.into());

        // port
        buf.write_u16::<BigEndian>(self.destination_socket.port()).unwrap();

        // destination IP
        buf.extend(&self.destination_socket.ip().octets());

        buf
    }
}

#[cfg(test)]
mod tests {}
