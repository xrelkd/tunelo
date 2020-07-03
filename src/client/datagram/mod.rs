mod socks;

use crate::{client::Error, common::ProxyHost};

pub use self::socks::Socks5Datagram;

pub enum ProxyDatagram {
    Socks5(Socks5Datagram),
}

impl ProxyDatagram {
    pub async fn bind(proxy_host: &ProxyHost) -> Result<ProxyDatagram, Error> {
        match proxy_host {
            ProxyHost::Socks5 { server, user_name, password } => Ok(ProxyDatagram::Socks5(
                Socks5Datagram::bind(server, user_name.as_deref(), password.as_deref()).await?,
            )),
            _ => return Err(Error::NoProxyProvided),
        }
    }
}
