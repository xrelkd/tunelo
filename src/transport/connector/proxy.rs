use std::net::SocketAddr;
use std::sync::Arc;

use futures::FutureExt;
use tokio::net::TcpStream;

use crate::client;
use crate::common::{HostAddress, ProxyStrategy};
use crate::transport::{
    connector::{Connect, Connector},
    Error,
};

#[derive(Clone)]
pub struct ProxyConnector {
    connector: client::ProxyConnector,
}

impl ProxyConnector {
    #[inline]
    pub fn new(proxy_strategy: Arc<ProxyStrategy>) -> Result<ProxyConnector, Error> {
        let connector = client::ProxyConnector::new(proxy_strategy)?;
        Ok(ProxyConnector { connector })
    }
}

impl Connector for ProxyConnector {
    type Stream = TcpStream;
    type Error = Error;

    fn connect(&self, host: &HostAddress) -> Connect<Self::Stream, Self::Error> {
        let host = host.clone();
        let connector = self.connector.clone();

        async move {
            let stream = connector.connect(&host).await?.into_inner();
            Ok(stream)
        }
        .boxed()
    }

    fn connect_addr(&self, addr: &SocketAddr) -> Connect<Self::Stream, Self::Error> {
        let host = HostAddress::from(addr.clone());
        self.connect(&host)
    }
}
