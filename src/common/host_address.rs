use std::fmt;
use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6};

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum HostAddress {
    Socket(SocketAddr),
    DomainName(String, u16),
}

impl PartialEq<(String, u16)> for HostAddress {
    fn eq(&self, other: &(String, u16)) -> bool {
        match self {
            HostAddress::DomainName(host, port) => host == &other.0 && port == &other.1,
            _ => false,
        }
    }
}
impl PartialEq<SocketAddr> for HostAddress {
    fn eq(&self, other: &SocketAddr) -> bool {
        match self {
            HostAddress::Socket(addr) => addr == other,
            _ => false,
        }
    }
}

impl HostAddress {
    #[inline]
    pub fn port(&self) -> u16 {
        match self {
            HostAddress::Socket(socket) => socket.port(),
            HostAddress::DomainName(_, port) => *port,
        }
    }

    #[inline]
    pub fn set_port(&mut self, port: u16) {
        match self {
            HostAddress::Socket(socket) => {
                socket.set_port(port);
            }
            HostAddress::DomainName(_, self_port) => {
                *self_port = port;
            }
        }
    }

    #[inline]
    pub fn empty_ipv4() -> Self {
        HostAddress::Socket(SocketAddr::new(Ipv4Addr::UNSPECIFIED.into(), 0))
    }

    #[inline]
    pub fn empty_ipv6() -> Self {
        HostAddress::Socket(SocketAddr::new(Ipv6Addr::UNSPECIFIED.into(), 0))
    }

    #[inline]
    pub fn empty_domain() -> Self {
        HostAddress::DomainName(String::new(), 0)
    }
}

impl From<SocketAddr> for HostAddress {
    fn from(addr: SocketAddr) -> HostAddress {
        HostAddress::Socket(addr)
    }
}

impl From<SocketAddrV4> for HostAddress {
    fn from(addr: SocketAddrV4) -> HostAddress {
        HostAddress::Socket(SocketAddr::V4(addr))
    }
}

impl From<SocketAddrV6> for HostAddress {
    fn from(addr: SocketAddrV6) -> HostAddress {
        HostAddress::Socket(SocketAddr::V6(addr))
    }
}

impl fmt::Display for HostAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HostAddress::Socket(ip) => ip.fmt(f),
            HostAddress::DomainName(host, port) => write!(f, "{}:{}", host, port),
        }
    }
}
