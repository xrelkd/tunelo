use std::{net::SocketAddr, pin::Pin, sync::Arc, time::Duration};

use futures::Future;
use tokio::{
    io::{AsyncRead, AsyncWrite},
    net::{TcpListener, TcpStream},
    sync::Mutex,
};

// FIXME: Use `Acceptor`.
#[allow(dead_code)]
pub trait Acceptor {
    type Stream: Unpin + AsyncRead + AsyncWrite;
    type Address;
    type Error;

    fn accept(&mut self) -> Accept<Self::Stream, Self::Address, Self::Error>;
}

pub type Accept<Stream, Address, Error> =
    Pin<Box<dyn Future<Output = Result<(Stream, Address), Error>> + Send>>;

// FIXME: Use `TcpAcceptor`.
#[allow(dead_code)]
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
        #[expect(
            clippy::no_effect_underscore_binding,
            reason = "Timeout stored for future use; may be needed for accept timeout \
                      configuration"
        )]
        let _timeout = self.timeout;

        #[expect(
            clippy::significant_drop_tightening,
            reason = "Arc<Mutex<TcpListener>> is cloned for async operation; the clone is \
                      intentionally held across await boundary"
        )]
        Box::pin(async move {
            let listener = listener.lock().await;
            let (stream, addr) = listener.accept().await?;
            Ok((stream, addr))
        })
    }
}
