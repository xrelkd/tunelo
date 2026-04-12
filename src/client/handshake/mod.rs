pub mod error;
mod http;
mod socks_v4;
mod socks_v5;

use snafu::ResultExt;
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt};

pub use self::error::Error;

pub struct ClientHandshake<Stream> {
    stream: Stream,
}

impl<Stream> ClientHandshake<Stream>
where
    Stream: Unpin + Send + Sync + AsyncRead + AsyncWrite,
{
    #[inline]
    pub const fn new(stream: Stream) -> Self { Self { stream } }

    #[inline]
    pub fn into_inner(self) -> Stream { self.stream }

    #[expect(dead_code, reason = "Reserved for future stream inspection capabilities")]
    #[inline]
    const fn as_ref(&self) -> &Stream { &self.stream }

    /// Shuts down the underlying stream.
    ///
    /// # Errors
    ///
    /// Returns an error if the stream shutdown fails.
    #[inline]
    pub async fn shutdown(mut self) -> Result<(), Error> {
        self.stream.shutdown().await.context(error::ShutdownStreamSnafu)?;
        Ok(())
    }
}
