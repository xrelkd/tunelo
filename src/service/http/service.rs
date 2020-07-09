use std::{net::SocketAddr, str::FromStr, sync::Arc};

use bytes::BytesMut;
use http::{header::HeaderName, HeaderMap, HeaderValue, Method};

use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
    sync::Mutex,
};

use crate::{
    authentication::AuthenticationManager,
    common::HostAddress,
    protocol::http::{Error as ProtocolError, StatusCode},
    service::http::Error,
    transport::Transport,
};

const INITIAL_BUF_SIZE: usize = 256;
const BUF_ADDITIONAL_SIZE: usize = 128;
const MAX_HEADER_BUF_SIZE: usize = 512;

struct ParsedMessage {
    req_method: Method,
    headers: HeaderMap,
    path: String,
}

impl ParsedMessage {
    fn host_address(&self) -> Option<HostAddress> {
        match self.headers.get(http::header::HOST) {
            Some(host) => HostAddress::from_str(host.to_str().unwrap_or_default()).ok(),
            None => HostAddress::from_str(&self.path).ok(),
        }
    }
}

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

    fn parse_header(buf: &mut BytesMut) -> Result<Option<ParsedMessage>, ProtocolError> {
        if buf.is_empty() {
            return Ok(None);
        }

        let mut empty_headers = [httparse::EMPTY_HEADER; 32];
        let mut request = httparse::Request::new(&mut empty_headers);
        let status = request
            .parse(&buf.as_ref())
            .map_err(|source| ProtocolError::ParseRequest { source })?;

        match status {
            httparse::Status::Partial => return Ok(None),
            httparse::Status::Complete(parsed_len) => {
                let method = request.method.ok_or(ProtocolError::NoMethodProvided)?;
                let method = Method::from_bytes(method.as_bytes())
                    .map_err(|_| ProtocolError::InvalidMethod { method: method.to_owned() })?;

                let path =
                    request.path.map(|p| p.to_string()).ok_or(ProtocolError::NoPathProvided)?;

                let mut headers = HeaderMap::with_capacity(request.headers.len());
                for header in request.headers {
                    let name = HeaderName::from_str(header.name).map_err(|_| {
                        ProtocolError::InvalidHeaderName { name: header.name.to_string() }
                    })?;
                    let value = HeaderValue::from_bytes(header.value).map_err(|_| {
                        ProtocolError::InvalidHeaderValue {
                            value: String::from_utf8_lossy(header.value).to_string(),
                        }
                    })?;
                    headers.append(name, value);
                }

                let _header_buf = buf.split_to(parsed_len);
                Ok(Some(ParsedMessage { req_method: method, headers, path }))
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
            let _n = client_stream
                .read_buf(&mut buf)
                .await
                .map_err(|source| Error::ReadBuf { source })?;
            match Self::parse_header(&mut buf) {
                Ok(Some(msg)) => break msg,
                Ok(None) => {
                    if buf.capacity() < MAX_HEADER_BUF_SIZE {
                        buf.reserve(std::cmp::min(
                            BUF_ADDITIONAL_SIZE,
                            MAX_HEADER_BUF_SIZE - buf.capacity(),
                        ));
                        continue;
                    }
                    return Err(Error::RequestTooLarge);
                }
                Err(err) => match err {
                    ProtocolError::ParseRequest { .. } => {
                        Self::shutdown_with_status(&mut client_stream, StatusCode::BadRequest)
                            .await?;
                        return Ok(());
                    }
                    ProtocolError::HostUnreachable => {
                        Self::shutdown_with_status(&mut client_stream, StatusCode::NotFound)
                            .await?;
                        return Ok(());
                    }
                    source => return Err(Error::OtherProtocolError { source }),
                },
            }
        };

        if msg.req_method != Method::CONNECT {
            client_stream
                .write(StatusCode::NotImplemented.status_line().as_bytes())
                .await
                .map_err(|source| Error::WriteStream { source })?;
            client_stream
                .write(
                    format!(
                        "<html lang=\"en\"><body>HTTP method {} is NOT SUPPORTED</body></html>\r\n",
                        msg.req_method
                    )
                    .as_bytes(),
                )
                .await
                .map_err(|source| Error::WriteStream { source })?;
            client_stream.shutdown().await.map_err(|source| Error::Shutdown { source })?;
            return Ok(());
        }

        let remote_host = match msg.host_address() {
            Some(r) => r,
            None => {
                client_stream
                    .write(StatusCode::NotFound.status_line().as_bytes())
                    .await
                    .map_err(|source| Error::WriteStream { source })?;
                client_stream.shutdown().await.map_err(|source| Error::Shutdown { source })?;
                return Ok(());
            }
        };

        let (remote_socket, _remote_addr) = match self.transport.connect(&remote_host).await {
            Ok((socket, addr)) => {
                let response = "HTTP/1.1 200 Connection Established\r\n\r\n";
                let _n = client_stream
                    .write(response.as_ref())
                    .await
                    .map_err(|source| Error::WriteStream { source })?;
                (socket, addr)
            }
            Err(source) => return Err(Error::ConnectRemoteHost { host: remote_host, source }),
        };

        let on_finished = Box::new(move || {
            info!("Remote host {} is disconnected", remote_host.to_string());
        });
        self.transport
            .relay(client_stream, remote_socket, Some(on_finished))
            .await
            .map_err(|source| Error::RelayStream { source })?;

        Ok(())
    }

    async fn shutdown_with_status(
        stream: &mut TransportStream,
        status_code: StatusCode,
    ) -> Result<(), Error> {
        stream
            .write(status_code.status_line().as_bytes())
            .await
            .map_err(|source| Error::WriteStream { source })?;
        stream.shutdown().await.map_err(|source| Error::Shutdown { source })?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {}
