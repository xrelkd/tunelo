use std::{
    io,
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};

use futures::Future;
use tokio::{
    io::{AsyncRead, AsyncWrite, ReadBuf, ReadHalf, WriteHalf},
    time::{self, Sleep},
};

pub struct TimedStream<Stream> {
    stream: Stream,
    timer: Option<Sleep>,
    timeout: Option<Duration>,
}

impl<Stream> TimedStream<Stream>
where
    Stream: Unpin + AsyncRead + AsyncWrite,
{
    #[inline]
    pub fn new(stream: Stream, timeout: Option<Duration>) -> TimedStream<Stream> {
        TimedStream { stream, timeout, timer: None }
    }

    #[inline]
    pub fn split(self) -> (ReadHalf<TimedStream<Stream>>, WriteHalf<TimedStream<Stream>>) {
        tokio::io::split(self)
    }

    #[inline]
    pub fn into_inner(self) -> Stream { self.stream }
}

impl<Stream> AsRef<Stream> for TimedStream<Stream>
where
    Stream: Unpin + AsyncRead + AsyncWrite,
{
    fn as_ref(&self) -> &Stream { &self.stream }
}

impl<Stream> AsMut<Stream> for TimedStream<Stream>
where
    Stream: Unpin + AsyncRead + AsyncWrite,
{
    fn as_mut(&mut self) -> &mut Stream { &mut self.stream }
}

impl<Stream> TimedStream<Stream> {
    #[inline]
    fn make_timeout_error() -> io::Error { io::ErrorKind::TimedOut.into() }

    fn poll_timeout(&mut self, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        loop {
            if let Some(ref mut timer) = self.timer {
                futures::ready!(Pin::new(timer).poll(cx));
                // FIXME: Clear self.timer or not?
                return Poll::Ready(Err(Self::make_timeout_error()));
            } else {
                match self.timeout {
                    Some(timeout) => self.timer = Some(time::sleep(timeout)),
                    None => break,
                }
            }
        }
        Poll::Ready(Ok(()))
    }

    fn cancel_timeout(&mut self) { let _ = self.timer.take(); }
}

impl<Stream> AsyncRead for TimedStream<Stream>
where
    Stream: Unpin + AsyncRead,
{
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        match Pin::new(&mut self.stream).poll_read(cx, buf) {
            Poll::Ready(r) => {
                self.cancel_timeout();
                Poll::Ready(r)
            }
            Poll::Pending => {
                futures::ready!(self.poll_timeout(cx))?;
                Poll::Pending
            }
        }
    }
}

impl<Stream> AsyncWrite for TimedStream<Stream>
where
    Stream: Unpin + AsyncWrite,
{
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context,
        buf: &[u8],
    ) -> Poll<Result<usize, io::Error>> {
        match Pin::new(&mut self.stream).poll_write(cx, buf) {
            Poll::Ready(r) => {
                self.cancel_timeout();
                Poll::Ready(r)
            }
            Poll::Pending => {
                futures::ready!(self.poll_timeout(cx))?;
                Poll::Pending
            }
        }
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), io::Error>> {
        match Pin::new(&mut self.stream).poll_flush(cx) {
            Poll::Ready(r) => {
                self.cancel_timeout();
                Poll::Ready(r)
            }
            Poll::Pending => {
                futures::ready!(self.poll_timeout(cx))?;
                Poll::Pending
            }
        }
    }

    fn poll_shutdown(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<(), io::Error>> {
        Pin::new(&mut self.stream).poll_shutdown(cx)
    }
}
