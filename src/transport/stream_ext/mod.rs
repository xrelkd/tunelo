mod monitored;
mod timed;

use std::{
    io,
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};

use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

pub use self::{
    monitored::{MonitoredStream, StatMonitor},
    timed::TimedStream,
};

pub struct StreamExt<Stream, Monitor> {
    stream: MonitoredStream<TimedStream<Stream>, Monitor>,
}

type ReadHalf<Stream, Monitor> = tokio::io::ReadHalf<StreamExt<Stream, Monitor>>;
type WriteHalf<Stream, Monitor> = tokio::io::WriteHalf<StreamExt<Stream, Monitor>>;

impl<Stream, Monitor> StreamExt<Stream, Monitor>
where
    Stream: Unpin + AsyncRead + AsyncWrite,
    Monitor: Unpin + StatMonitor,
{
    #[inline]
    pub fn new(
        stream: Stream,
        timeout: Option<Duration>,
        monitor: Monitor,
    ) -> StreamExt<Stream, Monitor> {
        let timed_stream = TimedStream::new(stream, timeout);
        let monitored_stream = MonitoredStream::new(timed_stream, monitor);
        StreamExt { stream: monitored_stream }
    }

    #[inline]
    pub fn split(self) -> (ReadHalf<Stream, Monitor>, WriteHalf<Stream, Monitor>) {
        tokio::io::split(self)
    }
}

impl<Stream, Monitor> AsyncRead for StreamExt<Stream, Monitor>
where
    Stream: Unpin + AsyncRead,
    Monitor: Unpin + StatMonitor,
{
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        Pin::new(&mut self.stream).poll_read(cx, buf)
    }
}

impl<Stream, Monitor> AsyncWrite for StreamExt<Stream, Monitor>
where
    Stream: Unpin + AsyncWrite,
    Monitor: Unpin + StatMonitor,
{
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context,
        buf: &[u8],
    ) -> Poll<Result<usize, io::Error>> {
        Pin::new(&mut self.stream).poll_write(cx, buf)
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
