use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};

use tokio::io::{AsyncRead, AsyncWrite, ReadHalf, WriteHalf};

pub trait StatMonitor: Send + Sync {
    fn increase_rx(&mut self, n: usize);
    fn increase_tx(&mut self, n: usize);
}

pub struct MonitoredStream<Stream, Monitor> {
    stream: Stream,
    monitor: Monitor,
}

impl<Stream, Monitor> MonitoredStream<Stream, Monitor>
where
    Stream: Unpin + AsyncRead + AsyncWrite,
    Monitor: Unpin + StatMonitor,
{
    #[inline]
    pub fn new(stream: Stream, monitor: Monitor) -> MonitoredStream<Stream, Monitor> {
        MonitoredStream { stream, monitor }
    }

    #[inline]
    pub fn split(
        self,
    ) -> (ReadHalf<MonitoredStream<Stream, Monitor>>, WriteHalf<MonitoredStream<Stream, Monitor>>)
    {
        tokio::io::split(self)
    }

    #[inline]
    pub fn into_inner(self) -> Stream {
        self.stream
    }
}

impl<Stream, Monitor> AsRef<Stream> for MonitoredStream<Stream, Monitor>
where
    Stream: Unpin + AsyncRead + AsyncWrite,
{
    fn as_ref(&self) -> &Stream {
        &self.stream
    }
}

impl<Stream, Monitor> AsMut<Stream> for MonitoredStream<Stream, Monitor>
where
    Stream: Unpin + AsyncRead + AsyncWrite,
{
    fn as_mut(&mut self) -> &mut Stream {
        &mut self.stream
    }
}

impl<Stream, Monitor> AsyncRead for MonitoredStream<Stream, Monitor>
where
    Stream: Unpin + AsyncRead,
    Monitor: Unpin + StatMonitor,
{
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        let n = match Pin::new(&mut self.stream).poll_read(cx, buf)? {
            Poll::Ready(n) => n,
            Poll::Pending => return Poll::Pending,
        };
        self.monitor.increase_rx(n);
        Poll::Ready(Ok(n))
    }
}

impl<Stream, Monitor> AsyncWrite for MonitoredStream<Stream, Monitor>
where
    Stream: Unpin + AsyncWrite,
    Monitor: Unpin + StatMonitor,
{
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context,
        buf: &[u8],
    ) -> Poll<Result<usize, io::Error>> {
        let n = match Pin::new(&mut self.stream).poll_write(cx, buf)? {
            Poll::Ready(n) => n,
            Poll::Pending => return Poll::Pending,
        };
        self.monitor.increase_tx(n);
        Poll::Ready(Ok(n))
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), io::Error>> {
        Pin::new(&mut self.stream).poll_flush(cx)
    }

    fn poll_shutdown(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<(), io::Error>> {
        Pin::new(&mut self.stream).poll_shutdown(cx)
    }
}
