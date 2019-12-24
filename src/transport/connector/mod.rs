use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;

use futures::Future;
use tokio::io::{AsyncRead, AsyncWrite};

use crate::common::HostAddress;

mod proxy;

pub use self::proxy::ProxyConnector;

pub type Connect<Stream, Error> = Pin<Box<dyn Future<Output = Result<Stream, Error>> + Send>>;

pub trait Connector: Send + Sync {
    type Stream: Unpin + AsyncRead + AsyncWrite;
    type Error: Send + Sync;

    fn connect(&self, host: &HostAddress) -> Connect<Self::Stream, Self::Error>;

    fn connect_addr(&self, addr: &SocketAddr) -> Connect<Self::Stream, Self::Error> {
        let host = HostAddress::from(addr.clone());
        self.connect(&host)
    }
}

type ConnectFn<Stream, Error> = dyn Fn(&HostAddress) -> Connect<Stream, Error> + Send + Sync;
type ConnectAddrFn<Stream, Error> = dyn Fn(&SocketAddr) -> Connect<Stream, Error> + Send + Sync;

pub struct FnConnector<Stream, Error> {
    connect_fn: Box<ConnectFn<Stream, Error>>,
    connect_addr_fn: Box<ConnectAddrFn<Stream, Error>>,
}

impl<Stream, Error> Connector for FnConnector<Stream, Error>
where
    Stream: Unpin + AsyncRead + AsyncWrite,
    Error: Send + Sync,
{
    type Stream = Stream;
    type Error = Error;

    fn connect(&self, host: &HostAddress) -> Connect<Self::Stream, Self::Error> {
        (self.connect_fn)(host)
    }

    fn connect_addr(&self, addr: &SocketAddr) -> Connect<Self::Stream, Self::Error> {
        (self.connect_addr_fn)(addr)
    }
}

pub fn connect_fn<Stream, Error>(
    connect_fn: Box<ConnectFn<Stream, Error>>,
    connect_addr_fn: Box<ConnectAddrFn<Stream, Error>>,
) -> Arc<FnConnector<Stream, Error>> {
    Arc::new(FnConnector { connect_fn, connect_addr_fn })
}
