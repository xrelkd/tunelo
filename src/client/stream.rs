use std::sync::Arc;

use tokio::net::TcpStream;

use crate::{
    client::{Error, ProxyConnector},
    common::{HostAddress, ProxyHost, ProxyStrategy},
};

#[derive(Debug)]
pub struct ProxyStream {
    socket: TcpStream,
    strategy: Arc<ProxyStrategy>,
}

impl ProxyStream {
    #[inline]
    pub fn from_raw(socket: TcpStream, strategy: Arc<ProxyStrategy>) -> ProxyStream {
        ProxyStream { socket, strategy }
    }

    #[inline]
    pub async fn connect_with_proxy(
        proxy_host: &ProxyHost,
        host: &HostAddress,
    ) -> Result<ProxyStream, Error> {
        let strategy = Arc::new(ProxyStrategy::Single(proxy_host.clone()));
        ProxyConnector::new(strategy)?.connect(host).await
    }

    #[inline]
    pub async fn connect_with_proxy_chain(
        proxies: Vec<ProxyHost>,
        host: &HostAddress,
    ) -> Result<ProxyStream, Error> {
        let strategy = Arc::new(ProxyStrategy::Chained(proxies));
        ProxyConnector::new(strategy)?.connect(host).await
    }

    #[inline]
    pub fn into_inner(self) -> TcpStream { self.socket }

    #[inline]
    pub fn proxy_strategy(&self) -> &ProxyStrategy { &self.strategy }
}

impl AsMut<TcpStream> for ProxyStream {
    fn as_mut(&mut self) -> &mut TcpStream { &mut self.socket }
}

impl AsRef<TcpStream> for ProxyStream {
    fn as_ref(&self) -> &TcpStream { &self.socket }
}
