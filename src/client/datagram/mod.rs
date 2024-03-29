mod socks;

pub use self::socks::Socks5Datagram;
use crate::{
    client::Error,
    common::{HostAddress, ProxyHost},
};

pub enum ProxyDatagram {
    Socks5(Socks5Datagram),
}

impl ProxyDatagram {
    pub async fn bind(proxy_host: &ProxyHost) -> Result<ProxyDatagram, Error> {
        match proxy_host {
            ProxyHost::Socks5 { host, port, username, password } => Ok(ProxyDatagram::Socks5(
                Socks5Datagram::bind(
                    &HostAddress::new(host, *port),
                    username.as_deref(),
                    password.as_deref(),
                )
                .await?,
            )),
            _ => Err(Error::NoProxyServiceProvided),
        }
    }
}
