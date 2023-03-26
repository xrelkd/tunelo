pub mod consts;
pub mod error;
pub mod v4;
pub mod v5;

use std::{
    convert::TryFrom,
    fmt,
    net::{SocketAddr, SocketAddrV4, SocketAddrV6},
};

use snafu::ResultExt;
use tokio::io::AsyncRead;

use crate::common::HostAddress;

pub use self::error::Error;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
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
            version => Err(Error::InvalidSocksVersion { version }),
        }
    }
}

impl From<SocksVersion> for u8 {
    fn from(version: SocksVersion) -> Self {
        match version {
            SocksVersion::V4 => consts::SOCKS4_VERSION,
            SocksVersion::V5 => consts::SOCKS5_VERSION,
        }
    }
}

impl fmt::Display for SocksVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SocksVersion::V4 => write!(f, "SOCKS4"),
            SocksVersion::V5 => write!(f, "SOCKS5"),
        }
    }
}

impl SocksVersion {
    #[inline]
    pub const fn serialized_len() -> usize { std::mem::size_of::<u8>() }
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

impl From<v4::Command> for SocksCommand {
    fn from(cmd: v4::Command) -> SocksCommand {
        match cmd {
            v4::Command::TcpConnect => SocksCommand::TcpConnect,
            v4::Command::TcpBind => SocksCommand::TcpBind,
        }
    }
}

impl From<v5::Command> for SocksCommand {
    fn from(cmd: v5::Command) -> SocksCommand {
        match cmd {
            v5::Command::TcpConnect => SocksCommand::TcpConnect,
            v5::Command::TcpBind => SocksCommand::TcpBind,
            v5::Command::UdpAssociate => SocksCommand::UdpAssociate,
        }
    }
}

impl std::fmt::Display for SocksCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SocksCommand::TcpConnect => write!(f, "TCP Connect"),
            SocksCommand::TcpBind => write!(f, "TCP Bind"),
            SocksCommand::UdpAssociate => write!(f, "UDP Associate"),
        }
    }
}

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub enum AddressType {
    Ipv4,
    Domain,
    Ipv6,
}

impl AddressType {
    #[inline]
    pub const fn serialized_len() -> usize { std::mem::size_of::<u8>() }
}

impl TryFrom<u8> for AddressType {
    type Error = Error;

    fn try_from(version: u8) -> Result<Self, Self::Error> {
        match version {
            consts::SOCKS5_ADDR_TYPE_IPV4 => Ok(AddressType::Ipv4),
            consts::SOCKS5_ADDR_TYPE_IPV6 => Ok(AddressType::Ipv6),
            consts::SOCKS5_ADDR_TYPE_DOMAIN_NAME => Ok(AddressType::Domain),
            ty => Err(Error::InvalidAddressType { ty }),
        }
    }
}

impl From<AddressType> for u8 {
    fn from(val: AddressType) -> Self {
        match val {
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
        let address_type = AddressType::try_from(rdr.read_u8().context(error::ReadStreamSnafu)?)?;
        match address_type {
            AddressType::Ipv4 => {
                let mut buf = [0u8; 4];
                rdr.read(&mut buf).context(error::ReadStreamSnafu)?;

                let port = rdr.read_u16::<BigEndian>().context(error::ReadStreamSnafu)?;
                Ok((SocketAddr::new(buf.into(), port).into(), rdr.position() as usize))
            }
            AddressType::Ipv6 => {
                let mut buf = [0u8; 16];
                rdr.read_exact(&mut buf).context(error::ReadStreamSnafu)?;

                let port = rdr.read_u16::<BigEndian>().context(error::ReadStreamSnafu)?;
                Ok((SocketAddr::new(buf.into(), port).into(), rdr.position() as usize))
            }
            AddressType::Domain => {
                let len = rdr.read_u8().context(error::ReadStreamSnafu)? as usize;

                let mut host = vec![0u8; len];
                rdr.read_exact(&mut host).context(error::ReadStreamSnafu)?;

                let port = rdr.read_u16::<BigEndian>().context(error::ReadStreamSnafu)?;
                Ok((Address::new_domain(&host, port), rdr.position() as usize))
            }
        }
    }

    pub async fn from_reader<R>(rdr: &mut R) -> Result<Address, Error>
    where
        R: AsyncRead + Unpin,
    {
        use tokio::io::AsyncReadExt;

        let address_type =
            AddressType::try_from(rdr.read_u8().await.context(error::ReadStreamSnafu)?)?;
        match address_type {
            AddressType::Ipv4 => {
                let mut buf = [0u8; 4];
                rdr.read_exact(&mut buf).await.context(error::ReadStreamSnafu)?;

                let port = rdr.read_u16().await.context(error::ReadStreamSnafu)?;
                Ok(SocketAddr::new(buf.into(), port).into())
            }
            AddressType::Ipv6 => {
                let mut buf = [0u8; 16];
                rdr.read_exact(&mut buf).await.context(error::ReadStreamSnafu)?;

                let port = rdr.read_u16().await.context(error::ReadStreamSnafu)?;
                Ok(SocketAddr::new(buf.into(), port).into())
            }
            AddressType::Domain => {
                let len = rdr.read_u8().await.context(error::ReadStreamSnafu)? as usize;

                let mut host = vec![0u8; len];
                rdr.read_exact(&mut host).await.context(error::ReadStreamSnafu)?;

                let port = rdr.read_u16().await.context(error::ReadStreamSnafu)?;
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
        Address(HostAddress::DomainName(String::from_utf8_lossy(host).into_owned(), port))
    }

    #[inline]
    pub fn empty_domain() -> Self { Self::from(HostAddress::empty_domain()) }

    #[inline]
    pub fn empty_ipv4() -> Self { Self::from(HostAddress::empty_ipv4()) }

    #[inline]
    pub fn empty_ipv6() -> Self { Self::from(HostAddress::empty_ipv6()) }

    #[inline]
    pub fn port(&self) -> u16 { self.0.port() }

    #[inline]
    pub fn set_port(&mut self, port: u16) { self.0.set_port(port); }

    pub fn address_type(&self) -> AddressType { AddressRef(&self.0).address_type() }

    #[inline]
    pub fn into_inner(self) -> HostAddress { self.0 }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct AddressRef<'a>(&'a HostAddress);

impl<'a> From<&'a HostAddress> for AddressRef<'a> {
    fn from(addr: &'a HostAddress) -> AddressRef<'a> { AddressRef(addr) }
}

impl<'a> From<AddressRef<'a>> for &'a HostAddress {
    fn from(val: AddressRef<'a>) -> Self { val.0 }
}

impl<'a> AddressRef<'a> {
    #[inline]
    pub const fn address_type(&self) -> AddressType {
        match &self.0 {
            HostAddress::Socket(SocketAddr::V4(_)) => AddressType::Ipv4,
            HostAddress::Socket(SocketAddr::V6(_)) => AddressType::Ipv6,
            HostAddress::DomainName(..) => AddressType::Domain,
        }
    }

    #[inline]
    pub fn serialized_len(&self, socks_version: SocksVersion) -> usize {
        match (socks_version, self.0) {
            (SocksVersion::V4, HostAddress::Socket(SocketAddr::V4(_))) => {
                std::mem::size_of::<u16>() + 4
            }
            (SocksVersion::V4, _) => 0,
            (SocksVersion::V5, HostAddress::Socket(SocketAddr::V4(_))) => {
                AddressType::serialized_len() + std::mem::size_of::<u16>() + 4
            }
            (SocksVersion::V5, HostAddress::Socket(SocketAddr::V6(_))) => {
                AddressType::serialized_len() + std::mem::size_of::<u16>() + 16
            }
            (SocksVersion::V5, HostAddress::DomainName(host, _)) => {
                AddressType::serialized_len()
                    + std::mem::size_of::<u8>()
                    + host.len()
                    + std::mem::size_of::<u16>()
            }
        }
    }

    fn to_bytes(&self, socks_version: SocksVersion) -> Vec<u8> {
        use byteorder::{BigEndian, WriteBytesExt};

        let mut buf = Vec::with_capacity(self.serialized_len(socks_version));
        match (socks_version, self.0) {
            (SocksVersion::V4, HostAddress::Socket(SocketAddr::V4(socket))) => {
                buf.write_u16::<BigEndian>(socket.port()).unwrap();
                buf.extend(&socket.ip().octets());
                buf
            }
            (SocksVersion::V4, _) => Vec::new(),
            (SocksVersion::V5, HostAddress::Socket(SocketAddr::V4(socket))) => {
                buf.push(AddressType::Ipv4.into());
                buf.extend(&socket.ip().octets());
                buf.write_u16::<BigEndian>(socket.port()).unwrap();
                buf
            }
            (SocksVersion::V5, HostAddress::Socket(SocketAddr::V6(socket))) => {
                buf.push(AddressType::Ipv6.into());
                buf.extend(&socket.ip().octets());
                buf.write_u16::<BigEndian>(socket.port()).unwrap();
                buf
            }
            (SocksVersion::V5, HostAddress::DomainName(host, port)) => {
                let host_len = host.len() as u8;
                buf.push(AddressType::Domain.into());
                buf.write_u8(host_len).unwrap();
                buf.extend(host.as_bytes());
                buf.write_u16::<BigEndian>(*port).unwrap();
                buf
            }
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
    fn from(addr: HostAddress) -> Self { Self(addr) }
}

impl From<Address> for HostAddress {
    fn from(val: Address) -> Self { val.0 }
}

impl From<SocketAddr> for Address {
    fn from(socket_addr: SocketAddr) -> Self { Self(HostAddress::from(socket_addr)) }
}

impl From<SocketAddrV4> for Address {
    fn from(socket_addr: SocketAddrV4) -> Self { Self(HostAddress::from(socket_addr)) }
}

impl From<SocketAddrV6> for Address {
    fn from(socket_addr: SocketAddrV6) -> Self { Self(HostAddress::from(socket_addr)) }
}

#[cfg(test)]
mod tests {}
