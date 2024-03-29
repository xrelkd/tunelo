use bytes::BytesMut;
use snafu::ResultExt;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use crate::{
    client::handshake::{error, ClientHandshake, Error},
    common::HostAddress,
};

const INITIAL_BUF_SIZE: usize = 128;
const BUF_ADDITIONAL_SIZE: usize = 128;
const MAX_BUF_SIZE: usize = 512;

impl<Stream> ClientHandshake<Stream>
where
    Stream: Unpin + Send + Sync + AsyncRead + AsyncWrite,
{
    pub async fn handshake_http_tunnel(
        &mut self,
        target_host: &HostAddress,
        user_agent: Option<&str>,
    ) -> Result<(), Error>
    where
        Stream: AsyncRead + AsyncWrite + Unpin,
    {
        let request = {
            use std::fmt::Write;
            let host = target_host.to_string();
            let mut req = BytesMut::with_capacity(128);
            write!(req, "CONNECT {host} HTTP/1.1\r\n").context(error::BuildHttpRequestSnafu)?;
            write!(req, "Host: {host}\r\n").context(error::BuildHttpRequestSnafu)?;

            if let Some(ua) = user_agent {
                write!(req, "User-Agent: {ua}\r\n").context(error::BuildHttpRequestSnafu)?;
            }

            write!(req, "\r\n").context(error::BuildHttpRequestSnafu)?;
            req
        };
        self.stream.write(request.as_ref()).await.context(error::WriteStreamSnafu)?;

        let mut buf = BytesMut::with_capacity(INITIAL_BUF_SIZE);
        let msg = loop {
            let _n = self.stream.read_buf(&mut buf).await.context(error::ReadStreamSnafu)?;
            match parse_header(&mut buf)? {
                None => {
                    if buf.capacity() < MAX_BUF_SIZE {
                        buf.reserve(std::cmp::min(
                            BUF_ADDITIONAL_SIZE,
                            MAX_BUF_SIZE - buf.capacity(),
                        ));
                        continue;
                    }
                    return Err(Error::HttpResponseTooLarge);
                }
                Some(msg) => break msg,
            }
        };

        match msg.status_code {
            200 => Ok(()),
            401..=404 => Err(Error::HostUnreachable),
            _ => Err(Error::HostUnreachable),
        }
    }
}

struct ParsedMessage {
    status_code: u16,
}

fn parse_header(buf: &mut BytesMut) -> Result<Option<ParsedMessage>, Error> {
    let mut headers = [httparse::EMPTY_HEADER; 32];
    let mut response = httparse::Response::new(&mut headers);
    match response.parse(&buf[..]) {
        Err(source) => Err(Error::ParseHttpResponse { source }),
        Ok(httparse::Status::Partial) => Ok(None),
        Ok(httparse::Status::Complete(parsed_len)) => {
            let status_code = response.code.ok_or(Error::NoHttpResponseCode)?;
            let _header_buf = buf.split_to(parsed_len).freeze();
            Ok(Some(ParsedMessage { status_code }))
        }
    }
}
