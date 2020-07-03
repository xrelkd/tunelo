use std::{
    net::{IpAddr, SocketAddr},
    sync::Arc,
};

use futures::FutureExt;
use tokio::{
    fs::File,
    io::{AsyncRead, AsyncWrite, AsyncWriteExt},
    net::TcpStream,
};

use crate::{
    common::{HostAddress, ProxyStrategy},
    filter::{FilterAction, HostFilter},
};

mod acceptor;
mod connector;
mod error;
mod metrics;
mod resolver;
mod stream_ext;

pub use self::error::Error;

pub use self::stream_ext::StatMonitor;
use self::{
    connector::{Connector, ProxyConnector},
    metrics::TransportMetrics,
    resolver::DummyResolver,
};

pub use self::{
    resolver::{DefaultResolver, Resolver, TokioResolver},
    stream_ext::{MonitoredStream, StreamExt, TimedStream},
};

pub struct Transport<Stream> {
    metrics: TransportMetrics,
    resolver: Arc<dyn Resolver>,
    connector: Arc<dyn Connector<Stream = Stream, Error = Error>>,
    filter: Arc<dyn HostFilter>,
}

impl Transport<File> {
    pub fn open_device(path: String, filter: Arc<dyn HostFilter>) -> Transport<File> {
        let metrics = TransportMetrics::new();

        let connector = connector::connect_fn(
            {
                let path = path.clone();
                Box::new(move |_host: &HostAddress| {
                    let path = path.clone();
                    async move {
                        let null_file = File::open(path).await?;
                        Ok(null_file)
                    }
                    .boxed()
                })
            },
            Box::new(move |_addr: &SocketAddr| {
                let path = path.clone();
                async move {
                    let null_file = File::open(path).await?;
                    Ok(null_file)
                }
                .boxed()
            }),
        );

        let resolver = Arc::new(DummyResolver::new());
        Transport { metrics, connector, resolver, filter }
    }

    #[inline]
    pub fn dev_random(filter: Arc<dyn HostFilter>) -> Transport<File> {
        Self::open_device("/dev/random".to_owned(), filter)
    }

    #[inline]
    pub fn dev_null(filter: Arc<dyn HostFilter>) -> Transport<File> {
        Self::open_device("/dev/null".to_owned(), filter)
    }
}

impl Transport<TcpStream> {
    pub fn direct(
        resolver: Arc<dyn Resolver>,
        filter: Arc<dyn HostFilter>,
    ) -> Transport<TcpStream> {
        let metrics = TransportMetrics::new();

        let connector = connector::connect_fn(
            Box::new(|host: &HostAddress| {
                let host = host.to_string();
                async move { Ok(TcpStream::connect(&host).await?) }.boxed()
            }),
            Box::new(|addr: &SocketAddr| {
                let addr = addr.clone();
                async move { Ok(TcpStream::connect(&addr).await?) }.boxed()
            }),
        );

        Transport { metrics, connector, resolver, filter }
    }

    pub fn proxy(
        resolver: Arc<dyn Resolver>,
        filter: Arc<dyn HostFilter>,
        strategy: Arc<ProxyStrategy>,
    ) -> Result<Transport<TcpStream>, Error> {
        let metrics = TransportMetrics::new();
        if !filter.check_proxy_strategy(strategy.as_ref()) {
            return Err(Error::ConnectForbiddenHost);
        }
        let connector = Arc::new(ProxyConnector::new(strategy)?);
        Ok(Transport { metrics, connector, resolver, filter })
    }
}

impl<Stream> Transport<Stream>
where
    Stream: Unpin + AsyncRead + AsyncWrite,
{
    #[inline]
    pub fn resolver(&self) -> Arc<dyn Resolver> { self.resolver.clone() }

    #[inline]
    pub fn connector(&self) -> Arc<dyn Connector<Stream = Stream, Error = Error>> {
        self.connector.clone()
    }

    #[inline]
    pub fn filter(&self) -> Arc<dyn HostFilter> { self.filter.clone() }

    pub async fn resolve_host(&self, host: &str) -> Result<IpAddr, Error> {
        let addrs = self.resolver.resolve(host).await?;
        if addrs.is_empty() {
            warn!("Failed to resolve domain name {}", host);
            return Err(Error::FailedToResolveDomainName);
        }
        let addr = addrs[0];
        debug!("Resolved {} => {}", host, addr);
        Ok(addr)
    }

    pub async fn resolve(&self, host: &HostAddress) -> Result<SocketAddr, Error> {
        match host {
            HostAddress::Socket(addr) => Ok(*addr),
            HostAddress::DomainName(host, port) => {
                let addr = self.resolve_host(host).await?;
                Ok(SocketAddr::new(addr, *port))
            }
        }
    }

    #[inline]
    pub async fn connect(&self, host: &HostAddress) -> Result<(Stream, HostAddress), Error> {
        if self.filter.filter_host_address(host) == FilterAction::Deny {
            return Err(Error::ConnectForbiddenHost);
        }

        debug!("Try to connect remote host {}", host.to_string());
        let host_addr = self.resolve(host).await?;
        let stream = match self.connector.connect_addr(&host_addr).await {
            Ok(stream) => stream,
            Err(err) => {
                error!("Failed to connect host: {}, error: {:?}", host, err);
                return Err(err);
            }
        };
        Ok((stream, host.clone()))
    }

    #[inline]
    pub async fn connect_addr(&self, addr: &SocketAddr) -> Result<(Stream, SocketAddr), Error> {
        if self.filter.filter_socket(addr) == FilterAction::Deny {
            return Err(Error::ConnectForbiddenHost);
        }

        debug!("Try to connect remote host {}", addr);
        let stream = match self.connector.connect_addr(&addr).await {
            Ok(stream) => stream,
            Err(err) => {
                error!("Failed to connect host: {}, error: {:?}", addr, err);
                return Err(err);
            }
        };
        Ok((stream, addr.clone()))
    }

    pub async fn relay<Client>(
        &self,
        client: Client,
        remote: Stream,
        on_finished: Option<Box<dyn FnOnce() -> () + Send>>,
    ) -> Result<(), Error>
    where
        Client: Unpin + AsyncRead + AsyncWrite,
    {
        let (client_counter, _prev_count) = self.metrics.count_client();
        let (remote_counter, _prev_count) = self.metrics.count_remote();
        let (relay_counter, _prev_count) = self.metrics.count_relay();

        let (mut client_reader, mut client_writer) = tokio::io::split(client);
        let (mut remote_reader, mut remote_writer) = tokio::io::split(remote);

        let half1 = tokio::io::copy(&mut client_reader, &mut remote_writer);
        let half2 = tokio::io::copy(&mut remote_reader, &mut client_writer);
        let fut = async {
            futures::select! {
                _ = half1.fuse() => {},
                _ = half2.fuse() => {},
            }
        };

        let _ = fut.await;

        on_finished.map(|f| f());

        let mut client = client_reader.unsplit(client_writer);
        let mut remote = remote_reader.unsplit(remote_writer);

        remote.shutdown();
        drop(remote_counter);

        client.shutdown();
        drop(client_counter);

        drop(relay_counter);

        Ok(())
    }
}
