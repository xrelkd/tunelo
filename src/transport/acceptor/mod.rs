use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use futures::Future;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::Mutex;

use crate::transport::stream_ext::{StatMonitor, StreamExt};

pub trait Acceptor {
    type Stream: Unpin + AsyncRead + AsyncWrite;
    type Address;
    type Error;

    fn accept(&mut self) -> Accept<Self::Stream, Self::Address, Self::Error>;
}

pub type Accept<Stream, Address, Error> =
    Pin<Box<dyn Future<Output = Result<(Stream, Address), Error>> + Send>>;

use tokio::net::{TcpListener, TcpStream};

pub struct TcpAcceptor<Monitor> {
    listener: Arc<Mutex<TcpListener>>,
    monitor: Monitor,
    timeout: Option<Duration>,
}

impl<Monitor> Acceptor for TcpAcceptor<Monitor>
where
    Monitor: 'static + Clone + Unpin + Send + Sync + StatMonitor,
{
    type Stream = StreamExt<TcpStream, Monitor>;
    type Address = SocketAddr;
    type Error = std::io::Error;

    fn accept(&mut self) -> Accept<Self::Stream, Self::Address, Self::Error> {
        let listener = self.listener.clone();
        let timeout = self.timeout.clone();
        let monitor = self.monitor.clone();
        Box::pin(async move {
            let mut listener = listener.lock().await;
            let (stream, addr) = listener.accept().await?;
            Ok((StreamExt::new(stream, timeout, monitor), addr))
        })
    }
}
