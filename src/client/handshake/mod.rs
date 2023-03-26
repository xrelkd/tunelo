use tokio::io::{AsyncRead, AsyncWrite};

pub mod error;
mod http;
mod socks_v4;
mod socks_v5;

pub use self::error::Error;

pub struct ClientHandshake<Stream> {
    stream: Stream,
}

impl<Stream> ClientHandshake<Stream>
where
    Stream: Unpin + Send + Sync + AsyncRead + AsyncWrite,
{
    #[inline]
    pub fn new(stream: Stream) -> ClientHandshake<Stream> { ClientHandshake { stream } }

    #[allow(dead_code)]
    #[inline]
    pub fn into_inner(self) -> Stream { self.stream }

    #[allow(dead_code)]
    #[inline]
    fn as_ref(&self) -> &Stream { &self.stream }

    #[allow(dead_code)]
    #[inline]
    pub async fn shutdown(mut self) -> Result<(), Error> {
        use snafu::ResultExt;
        use tokio::io::AsyncWriteExt;

        self.stream.shutdown().await.context(error::ShutdownStreamSnafu)?;
        Ok(())
    }
}
