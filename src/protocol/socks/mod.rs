use std::{
    convert::TryFrom,
    net::{SocketAddr, SocketAddrV4, SocketAddrV6},
};

use tokio::io::AsyncRead;

use crate::common::HostAddress;

mod error;

pub mod v4;
pub mod v5;

pub use self::error::Error;

#[rustfmt::skip]
pub mod consts {
    pub const SOCKS4_VERSION:                          u8 = 0x04;
    pub const SOCKS5_VERSION:                          u8 = 0x05;

    pub const SOCKS4_CMD_TCP_CONNECT:                  u8 = 0x01;
    pub const SOCKS4_CMD_TCP_BIND:                     u8 = 0x02;

    pub const SOCKS4_REPLY_GRANTED:                    u8 = 0x5A;
    pub const SOCKS4_REPLY_REJECTED:                   u8 = 0x5B;
    pub const SOCKS4_REPLY_UNREACHABLE:                u8 = 0x5C;
    pub const SOCKS4_REPLY_INVALID_ID:                 u8 = 0x5D;

    pub const SOCKS5_AUTH_METHOD_NONE:                 u8 = 0x00;
    pub const SOCKS5_AUTH_METHOD_GSSAPI:               u8 = 0x01;
    pub const SOCKS5_AUTH_METHOD_PASSWORD:             u8 = 0x02;
    pub const SOCKS5_AUTH_METHOD_NOT_ACCEPTABLE:       u8 = 0xFF;

    pub const SOCKS5_CMD_TCP_CONNECT:                  u8 = 0x01;
    pub const SOCKS5_CMD_TCP_BIND:                     u8 = 0x02;
    pub const SOCKS5_CMD_UDP_ASSOCIATE:                u8 = 0x03;

    pub const SOCKS5_ADDR_TYPE_IPV4:                   u8 = 0x01;
    pub const SOCKS5_ADDR_TYPE_DOMAIN_NAME:            u8 = 0x03;
    pub const SOCKS5_ADDR_TYPE_IPV6:                   u8 = 0x04;

    pub const SOCKS5_REPLY_SUCCEEDED:                  u8 = 0x00;
    pub const SOCKS5_REPLY_GENERAL_FAILURE:            u8 = 0x01;
    pub const SOCKS5_REPLY_CONNECTION_NOT_ALLOWED:     u8 = 0x02;
    pub const SOCKS5_REPLY_NETWORK_UNREACHABLE:        u8 = 0x03;
    pub const SOCKS5_REPLY_HOST_UNREACHABLE:           u8 = 0x04;
    pub const SOCKS5_REPLY_CONNECTION_REFUSED:         u8 = 0x05;
    pub const SOCKS5_REPLY_TTL_EXPIRED:                u8 = 0x06;
    pub const SOCKS5_REPLY_COMMAND_NOT_SUPPORTED:      u8 = 0x07;
    pub const SOCKS5_REPLY_ADDRESS_TYPE_NOT_SUPPORTED: u8 = 0x08;
    pub const SOCKS5_REPLY_UNKNOWN:                    u8 = 0xFF;
}

#[derive(Hash, Clone, Copy, Debug, Eq, PartialEq)]
pub enum SocksVersion {
    V4,
    V5,
}

impl TryFrom<u8> for SocksVersion {
    type Error = Error;

    fn try_from(version: u8) -> Result<Self, Self::Error> {
        match version {
            consts::SOCKS4_VERSION => Ok(SocksVersion::V4),
            consts::SOCKS5_VERSION => Ok(SocksVersion::V5),
            version => Err(Error::InvalidSocksVersion(version)),
        }
    }
}

impl Into<u8> for SocksVersion {
    fn into(self) -> u8 {
        match self {
            SocksVersion::V4 => consts::SOCKS4_VERSION,
            SocksVersion::V5 => consts::SOCKS5_VERSION,
        }
    }
}

impl SocksVersion {
    #[inline]
    pub fn serialized_len() -> usize { std::mem::size_of::<u8>() }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum SocksCommand {
    TcpConnect,
    TcpBind,
    UdpAssociate,
}

impl SocksCommand {
    #[inline]
    pub fn serialized_len(&self) -> usize { std::mem::size_of::<u8>() }
}

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub enum AddressType {
    Ipv4,
    Domain,
    Ipv6,
}

impl AddressType {
    #[inline]
    pub fn serialized_len() -> usize { std::mem::size_of::<u8>() }
}

impl TryFrom<u8> for AddressType {
    type Error = Error;

    fn try_from(version: u8) -> Result<Self, Self::Error> {
        match version {
            consts::SOCKS5_ADDR_TYPE_IPV4 => Ok(AddressType::Ipv4),
            consts::SOCKS5_ADDR_TYPE_IPV6 => Ok(AddressType::Ipv6),
            consts::SOCKS5_ADDR_TYPE_DOMAIN_NAME => Ok(AddressType::Domain),
            v => Err(Error::InvalidAddressType(v)),
        }
    }
}

impl Into<u8> for AddressType {
    fn into(self) -> u8 {
        match self {
            AddressType::Ipv4 => consts::SOCKS5_ADDR_TYPE_IPV4,
            AddressType::Ipv6 => consts::SOCKS5_ADDR_TYPE_IPV6,
            AddressType::Domain => consts::SOCKS5_ADDR_TYPE_DOMAIN_NAME,
        }
    }
}

#[derive(Hash, Debug, Clone, Eq, PartialEq)]
pub struct Address(HostAddress);

impl Address {
    pub fn from_bytes(buf: &mut [u8]) -> Result<(Address, usize), Error> {
        use byteorder::{BigEndian, ReadBytesExt};
        use std::io::Read;

        let mut rdr = std::io::Cursor::new(buf);
        let address_type = AddressType::try_from(rdr.read_u8()?)?;
        match address_type {
            AddressType::Ipv4 => {
                let mut buf = [0u8; 4];
                rdr.read(&mut buf)?;

                let port = rdr.read_u16::<BigEndian>()?;
                Ok((SocketAddr::new(buf.into(), port).into(), rdr.position() as usize))
            }
            AddressType::Ipv6 => {
                let mut buf = [0u8; 16];
                rdr.read_exact(&mut buf)?;

                let port = rdr.read_u16::<BigEndian>()?;
                Ok((SocketAddr::new(buf.into(), port).into(), rdr.position() as usize))
            }
            AddressType::Domain => {
                let len = rdr.read_u8()? as usize;

                let mut host = vec![0u8; len];
                rdr.read_exact(&mut host)?;

                let port = rdr.read_u16::<BigEndian>()?;
                Ok((Address::new_domain(&host, port), rdr.position() as usize))
            }
        }
    }

    pub async fn from_reader<R>(rdr: &mut R) -> Result<Address, Error>
    where
        R: AsyncRead + Unpin,
    {
        use tokio::io::AsyncReadExt;

        let address_type = AddressType::try_from(rdr.read_u8().await?)?;
        match address_type {
            AddressType::Ipv4 => {
                let mut buf = [0u8; 4];
                rdr.read_exact(&mut buf).await?;

                let port = rdr.read_u16().await?;
                Ok(SocketAddr::new(buf.into(), port).into())
            }
            AddressType::Ipv6 => {
                let mut buf = [0u8; 16];
                rdr.read_exact(&mut buf).await?;

                let port = rdr.read_u16().await?;
                Ok(SocketAddr::new(buf.into(), port).into())
            }
            AddressType::Domain => {
                let len = rdr.read_u8().await? as usize;

                let mut host = vec![0u8; len];
                rdr.read_exact(&mut host).await?;

                let port = rdr.read_u16().await?;
                Ok(Address::new_domain(&host, port))
            }
        }
    }

    #[inline]
    pub fn max_len() -> usize {
        AddressType::serialized_len() + std::mem::size_of::<u8>() + 256 + std::mem::size_of::<u16>()
    }

    #[inline]
    pub fn serialized_len(&self, socks_version: SocksVersion) -> usize {
        AddressRef(&self.0).serialized_len(socks_version)
    }

    pub fn to_bytes(&self, socks_version: SocksVersion) -> Vec<u8> {
        AddressRef(&self.0).to_bytes(socks_version)
    }

    pub fn into_bytes(self, socks_version: SocksVersion) -> Vec<u8> { self.to_bytes(socks_version) }

    pub fn new_domain(host: &[u8], port: u16) -> Address {
        Address(HostAddress::DomainName(String::from_utf8_lossy(&host).into_owned(), port))
    }

    #[allow(dead_code)]
    pub fn empty_domain() -> Self { Address::from(HostAddress::empty_domain()) }

    #[inline]
    pub fn empty_ipv4() -> Self { Address::from(HostAddress::empty_ipv4()) }

    #[inline]
    pub fn empty_ipv6() -> Self { Address::from(HostAddress::empty_ipv6()) }

    #[inline]
    pub fn port(&self) -> u16 { self.0.port() }

    #[inline]
    pub fn set_port(&mut self, port: u16) { self.0.set_port(port); }

    pub fn address_type(&self) -> AddressType { AddressRef(&self.0).address_type() }

    #[inline]
    pub fn into_inner(self) -> HostAddress { self.0 }
}

#[derive(Hash, Debug, Clone, Eq, PartialEq)]
pub struct AddressRef<'a>(&'a HostAddress);

impl<'a> From<&'a HostAddress> for AddressRef<'a> {
    fn from(addr: &'a HostAddress) -> AddressRef<'a> { AddressRef(addr) }
}

impl<'a> Into<&'a HostAddress> for AddressRef<'a> {
    fn into(self) -> &'a HostAddress { self.0 }
}

impl<'a> AddressRef<'a> {
    #[inline]
    pub fn address_type(&self) -> AddressType {
        match &self.0 {
            HostAddress::Socket(socket) => match socket {
                SocketAddr::V4(_) => AddressType::Ipv4,
                SocketAddr::V6(_) => AddressType::Ipv6,
            },
            HostAddress::DomainName(..) => AddressType::Domain,
        }
    }

    #[inline]
    pub fn serialized_len(&self, socks_version: SocksVersion) -> usize {
        match socks_version {
            SocksVersion::V4 => match self.0 {
                HostAddress::Socket(socket) => match socket {
                    SocketAddr::V4(_) => std::mem::size_of::<u16>() + 4,
                    _ => 0,
                },
                _ => 0,
            },
            SocksVersion::V5 => match &self.0 {
                HostAddress::Socket(socket) => match socket {
                    SocketAddr::V4(_) => {
                        AddressType::serialized_len() + std::mem::size_of::<u16>() + 4
                    }
                    SocketAddr::V6(_) => {
                        AddressType::serialized_len() + std::mem::size_of::<u16>() + 16
                    }
                },
                HostAddress::DomainName(host, _) => {
                    AddressType::serialized_len()
                        + std::mem::size_of::<u8>()
                        + host.len()
                        + std::mem::size_of::<u16>()
                }
            },
        }
    }

    fn to_bytes(&self, socks_version: SocksVersion) -> Vec<u8> {
        use byteorder::{BigEndian, WriteBytesExt};

        let mut buf = Vec::with_capacity(self.serialized_len(socks_version));
        match socks_version {
            SocksVersion::V4 => match self.0 {
                HostAddress::Socket(socket) => match socket {
                    SocketAddr::V4(socket) => {
                        buf.write_u16::<BigEndian>(socket.port()).unwrap();
                        buf.extend(&socket.ip().octets());
                        buf
                    }

                    _ => vec![],
                },
                _ => vec![],
            },
            SocksVersion::V5 => match &self.0 {
                HostAddress::Socket(socket) => match socket {
                    SocketAddr::V4(socket) => {
                        buf.push(AddressType::Ipv4.into());
                        buf.extend(&socket.ip().octets());
                        buf.write_u16::<BigEndian>(socket.port()).unwrap();
                        buf
                    }
                    SocketAddr::V6(socket) => {
                        buf.push(AddressType::Ipv6.into());
                        buf.extend(&socket.ip().octets());
                        buf.write_u16::<BigEndian>(socket.port()).unwrap();
                        buf
                    }
                },
                HostAddress::DomainName(host, port) => {
                    let host_len = host.len() as u8;
                    buf.push(AddressType::Domain.into());
                    buf.write_u8(host_len).unwrap();
                    buf.extend(host.as_bytes());
                    buf.write_u16::<BigEndian>(*port).unwrap();
                    buf
                }
            },
        }
    }
}

impl AsRef<HostAddress> for Address {
    fn as_ref(&self) -> &HostAddress { &self.0 }
}

impl ToString for Address {
    fn to_string(&self) -> String { self.0.to_string() }
}

impl From<HostAddress> for Address {
    fn from(addr: HostAddress) -> Address { Address(addr) }
}

impl Into<HostAddress> for Address {
    fn into(self) -> HostAddress { self.0 }
}

impl From<SocketAddr> for Address {
    fn from(socket_addr: SocketAddr) -> Address { Address(HostAddress::from(socket_addr)) }
}

impl From<SocketAddrV4> for Address {
    fn from(socket_addr: SocketAddrV4) -> Address { Address(HostAddress::from(socket_addr)) }
}

impl From<SocketAddrV6> for Address {
    fn from(socket_addr: SocketAddrV6) -> Address { Address(HostAddress::from(socket_addr)) }
}

#[cfg(test)]
mod tests {}
