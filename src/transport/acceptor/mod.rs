use std::{net::SocketAddr, pin::Pin, sync::Arc, time::Duration};

use futures::Future;
use tokio::{
    io::{AsyncRead, AsyncWrite},
    net::{TcpListener, TcpStream},
    sync::Mutex,
};

pub trait Acceptor {
    type Stream: Unpin + AsyncRead + AsyncWrite;
    type Address;
    type Error;

    fn accept(&mut self) -> Accept<Self::Stream, Self::Address, Self::Error>;
}

pub type Accept<Stream, Address, Error> =
    Pin<Box<dyn Future<Output = Result<(Stream, Address), Error>> + Send>>;

pub struct TcpAcceptor {
    listener: Arc<Mutex<TcpListener>>,
    timeout: Option<Duration>,
}

impl Acceptor for TcpAcceptor {
    type Address = SocketAddr;
    type Error = std::io::Error;
    type Stream = TcpStream;

    fn accept(&mut self) -> Accept<Self::Stream, Self::Address, Self::Error> {
        let listener = self.listener.clone();
        let _timeout = self.timeout;

        Box::pin(async move {
            let listener = listener.lock().await;
            let (stream, addr) = listener.accept().await?;
            Ok((stream, addr))
        })
    }
}
