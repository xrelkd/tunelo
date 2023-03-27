use std::{net::SocketAddr, sync::Arc};

use futures::FutureExt;
use snafu::ResultExt;
use tokio::net::TcpStream;

use crate::{
    client,
    common::{HostAddress, ProxyStrategy},
    transport::{
        connector::{Connect, Connector},
        error, Error,
    },
};

#[derive(Clone)]
pub struct ProxyConnector {
    connector: client::ProxyConnector,
}

impl ProxyConnector {
    #[inline]
    pub fn new(proxy_strategy: Arc<ProxyStrategy>) -> Result<Self, Error> {
        let connector = client::ProxyConnector::new(proxy_strategy)
            .context(error::CreateProxyConnectorSnafu)?;
        Ok(Self { connector })
    }
}

impl Connector for ProxyConnector {
    type Error = Error;
    type Stream = TcpStream;

    fn connect(&self, host: &HostAddress) -> Connect<Self::Stream, Self::Error> {
        let host = host.clone();
        let connector = self.connector.clone();

        async move {
            let stream = connector.connect(&host).await.context(error::ConnectProxyServerSnafu)?;
            Ok(stream.into_inner())
        }
        .boxed()
    }

    fn connect_addr(&self, addr: &SocketAddr) -> Connect<Self::Stream, Self::Error> {
        let host = HostAddress::from(*addr);
        self.connect(&host)
    }
}
