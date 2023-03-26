use std::{net::SocketAddr, str::FromStr, sync::Arc};

use bytes::{Bytes, BytesMut};
use http::{header::HeaderName, HeaderMap, HeaderValue, Method, StatusCode};
use url::Url;

use snafu::ResultExt;
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
    sync::Mutex,
};

use crate::{
    authentication::AuthenticationManager,
    common::HostAddress,
    service::http::{error, Error},
    transport::Transport,
};

const INITIAL_BUF_SIZE: usize = 256;
const BUF_ADDITIONAL_SIZE: usize = 128;
const MAX_HEADER_BUF_SIZE: usize = 10240;

pub struct Service<TransportStream> {
    transport: Arc<Transport<TransportStream>>,
    _authentication_manager: Arc<Mutex<AuthenticationManager>>,
}

impl<TransportStream> Service<TransportStream>
where
    TransportStream: Unpin + AsyncRead + AsyncWrite,
{
    pub fn new(
        transport: Arc<Transport<TransportStream>>,
        authentication_manager: Arc<Mutex<AuthenticationManager>>,
    ) -> Service<TransportStream> {
        Service { transport, _authentication_manager: authentication_manager }
    }

    fn parse_header(buf: &mut BytesMut) -> Result<Option<ParsedMessage>, Error> {
        if buf.is_empty() {
            return Ok(None);
        }

        let mut empty_headers = [httparse::EMPTY_HEADER; 32];
        let mut request = httparse::Request::new(&mut empty_headers);
        let status = request.parse(buf.as_ref()).context(error::ParseRequestSnafu)?;

        match status {
            httparse::Status::Partial => Ok(None),
            httparse::Status::Complete(parsed_len) => {
                let method = {
                    let method = request.method.ok_or(Error::NoMethodProvided)?;
                    Method::from_bytes(method.as_bytes())
                        .map_err(|_| Error::InvalidMethod { method: method.to_owned() })?
                };

                let url = match request.path {
                    Some(p) => Url::from_str(p).context(error::ParseUrlSnafu)?,
                    None => return Err(Error::NoPathProvided),
                };

                let mut headers = HeaderMap::with_capacity(request.headers.len());
                for header in request.headers {
                    let name = HeaderName::from_str(header.name)
                        .map_err(|_| Error::InvalidHeaderName { name: header.name.to_string() })?;
                    let value = HeaderValue::from_bytes(header.value).map_err(|_| {
                        Error::InvalidHeaderValue {
                            value: String::from_utf8_lossy(header.value).to_string(),
                        }
                    })?;
                    headers.append(name, value);
                }

                let header_buf = buf.split_to(parsed_len).freeze();
                Ok(Some(ParsedMessage { req_method: method, headers, url, header_buf }))
            }
        }
    }

    pub async fn handle(
        &self,
        mut client_stream: TransportStream,
        _client_addr: SocketAddr,
    ) -> Result<(), Error> {
        let mut buf = BytesMut::with_capacity(INITIAL_BUF_SIZE);
        let msg = loop {
            let _n = client_stream.read_buf(&mut buf).await.context(error::ReadBufSnafu)?;
            match Self::parse_header(&mut buf) {
                Ok(Some(msg)) => break msg,
                Ok(None) => {
                    if !buf.is_empty() && buf.capacity() < MAX_HEADER_BUF_SIZE {
                        let additional_size = std::cmp::min(
                            BUF_ADDITIONAL_SIZE,
                            MAX_HEADER_BUF_SIZE - buf.capacity(),
                        );
                        buf.reserve(additional_size);
                        continue;
                    }
                    Self::shutdown_with_status(client_stream, StatusCode::BAD_REQUEST).await?;
                    return Err(Error::RequestTooLarge);
                }
                Err(err) => {
                    Self::shutdown_with_status(client_stream, StatusCode::BAD_REQUEST).await?;
                    return Err(err);
                }
            }
        };

        let remote_host = match msg.host_address() {
            Some(r) => r,
            None => {
                Self::shutdown_with_status(client_stream, StatusCode::NOT_FOUND).await?;
                return Err(Error::NoHostProvided);
            }
        };

        let (remote_socket, _remote_addr) = match self.transport.connect(&remote_host).await {
            Ok((mut remote_socket, addr)) => {
                match msg.req_method {
                    Method::CONNECT => {
                        const ESTABLISHED_RESPONSE: &[u8] =
                            b"HTTP/1.1 200 Connection Established\r\n\r\n";
                        let _n = client_stream
                            .write(ESTABLISHED_RESPONSE)
                            .await
                            .context(error::WriteStreamSnafu)?;
                    }
                    _ => {
                        let _n = remote_socket.write(msg.header_buf.as_ref()).await;
                    }
                }
                (remote_socket, addr)
            }
            Err(source) => {
                return Err(Error::ConnectRemoteHost {
                    host: remote_host,
                    source: Box::new(source),
                })
            }
        };

        let on_finished = Box::new(move || {
            tracing::info!("Remote host {} is disconnected", remote_host.to_string());
        });
        self.transport
            .relay(client_stream, remote_socket, Some(on_finished))
            .await
            .context(error::RelayStreamSnafu)?;

        Ok(())
    }

    #[inline]
    async fn shutdown_with_status(
        mut stream: TransportStream,
        status_code: StatusCode,
    ) -> Result<(), Error> {
        stream
            .write(status_code.status_line().as_bytes())
            .await
            .context(error::WriteStreamSnafu)?;
        stream.shutdown().await.context(error::ShutdownSnafu)?;
        Ok(())
    }
}

trait StatusCodeExt {
    fn status_line(&self) -> String;
}

impl StatusCodeExt for StatusCode {
    fn status_line(&self) -> String {
        match self.canonical_reason() {
            Some(reason) => format!("HTTP/1.1 {} {}\r\n\r\n", self.as_u16(), reason),
            None => format!("HTTP/1.1 {}\r\n\r\n", self.as_u16()),
        }
    }
}

#[derive(Debug)]
struct ParsedMessage {
    req_method: Method,
    headers: HeaderMap,
    url: Url,
    header_buf: Bytes,
}

impl ParsedMessage {
    fn host_address(&self) -> Option<HostAddress> {
        match (&self.req_method, self.headers.get(http::header::HOST)) {
            (&Method::CONNECT, Some(host)) => {
                HostAddress::from_str(host.to_str().unwrap_or_default()).ok()
            }
            _ => {
                let domain = &self.url.host_str()?;
                let port = self.url.port_or_known_default()?;
                Some(HostAddress::new(domain, port))
            }
        }
    }
}

#[cfg(test)]
mod tests {}
