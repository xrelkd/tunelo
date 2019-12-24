use std::convert::TryFrom;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};

use tokio::io::{AsyncRead, AsyncReadExt};

use crate::common::HostAddress;
use crate::protocol::socks::{consts, Address, Error, SocksVersion};

#[derive(Debug, Hash, Clone, Copy, Eq, PartialEq)]
pub enum Command {
    TcpConnect,
    TcpBind,
}

impl Into<u8> for Command {
    fn into(self) -> u8 {
        match self {
            Command::TcpConnect => consts::SOCKS4_CMD_TCP_CONNECT,
            Command::TcpBind => consts::SOCKS4_CMD_TCP_BIND,
        }
    }
}

impl TryFrom<u8> for Command {
    type Error = Error;
    fn try_from(cmd: u8) -> Result<Self, Self::Error> {
        match cmd {
            consts::SOCKS4_CMD_TCP_CONNECT => Ok(Command::TcpConnect),
            consts::SOCKS4_CMD_TCP_BIND => Ok(Command::TcpBind),
            cmd => Err(Error::InvalidCommand(cmd)),
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

impl Into<u8> for ReplyField {
    fn into(self) -> u8 {
        match self {
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
            consts::SOCKS4_REPLY_GRANTED => Ok(ReplyField::Granted),
            consts::SOCKS4_REPLY_REJECTED => Ok(ReplyField::Rejected),
            consts::SOCKS4_REPLY_UNREACHABLE => Ok(ReplyField::Unreachable),
            consts::SOCKS4_REPLY_INVALID_ID => Ok(ReplyField::InvalidId),
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
    pub async fn from_reader<R>(rdr: &mut R) -> Result<Request, Error>
    where
        R: AsyncRead + Unpin,
    {
        let command = Command::try_from(rdr.read_u8().await?)?;
        let port = rdr.read_u16().await?;
        let mut ip_buf = [0u8; 4];
        let _ = rdr.read(&mut ip_buf).await?;

        let (id, host) = {
            let mut buf = [0u8; 128];
            let _ = rdr.read(&mut buf).await?;

            let parts: Vec<_> = buf.split(|ch| *ch == 0x00).collect();

            match parts.len() {
                0 => (vec![], vec![]),
                1 => (parts[0].to_vec(), vec![]),
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

        Ok(Request { command, destination_socket, id })
    }

    pub fn into_bytes(&self) -> Vec<u8> {
        self.to_bytes()
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        //
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
        let _ = buf.write_u16::<BigEndian>(self.destination_socket.port());

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
    pub async fn from_reader<R>(rdr: &mut R) -> Result<Reply, Error>
    where
        R: AsyncRead + AsyncRead + Unpin,
    {
        if rdr.read_u8().await? != 0x00 {
            return Err(Error::BadReply);
        }

        let reply = ReplyField::try_from(rdr.read_u8().await?)?;
        let destination_socket = {
            let port = rdr.read_u16().await?;
            let mut ip = [0u8; 4];
            rdr.read(&mut ip).await?;
            SocketAddrV4::new(Ipv4Addr::from(ip), port)
        };

        Ok(Reply { reply, destination_socket })
    }

    pub fn granted(destination_socket: SocketAddrV4) -> Reply {
        Reply { reply: ReplyField::Granted, destination_socket }
    }

    pub fn rejected(destination_socket: SocketAddrV4) -> Reply {
        Reply { reply: ReplyField::Rejected, destination_socket }
    }

    pub fn unreachable(destination_socket: SocketAddrV4) -> Reply {
        Reply { reply: ReplyField::Unreachable, destination_socket }
    }

    #[allow(dead_code)]
    pub fn invalid_id(destination_socket: SocketAddrV4) -> Reply {
        Reply { reply: ReplyField::InvalidId, destination_socket }
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.to_bytes()
    }

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
