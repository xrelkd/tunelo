use std::str::FromStr;
use std::{net::SocketAddr, sync::Arc};

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

    pub async fn handle(
        &self,
        mut client_stream: TransportStream,
        _client_addr: SocketAddr,
    ) -> Result<(), Error> {
        let remote_host = match Self::parse_request(&mut client_stream).await {
            Ok(r) => r,
            Err(ProtocolError::BadRequest) => {
                Self::shutdown_with_status(&mut client_stream, StatusCode::BadRequest).await?;
                return Ok(());
            }
            Err(ProtocolError::HostUnreachable) => {
                Self::shutdown_with_status(&mut client_stream, StatusCode::NotFound).await?;
                return Ok(());
            }
            Err(ProtocolError::UnsupportedMethod { method }) => {
                client_stream.write(StatusCode::NotImplemented.status_line().as_bytes()).await?;
                client_stream
                    .write(
                        format!(
                            "<html lang=\"en\"><body>HTTP method {} is NOT \
                             SUPPORTED</body></html>\r\n",
                            method
                        )
                        .as_bytes(),
                    )
                    .await?;
                client_stream.shutdown().await?;
                return Ok(());
            }
            Err(err) => return Err(err.into()),
        };

        let (remote_socket, _remote_addr) = match self.transport.connect(&remote_host).await {
            Ok((socket, addr)) => {
                let response = "HTTP/1.1 200 Connection Established\r\n\r\n";
                let _n = client_stream.write(response.as_ref()).await?;
                (socket, addr)
            }
            Err(_err) => return Err(Error::Protocol { source: ProtocolError::HostUnreachable }),
        };

        let on_finished = Box::new(move || {
            info!("Remote host {} is disconnected", remote_host.to_string());
        });
        self.transport.relay(client_stream, remote_socket, Some(on_finished)).await?;

        Ok(())
    }

    async fn shutdown_with_status(
        stream: &mut TransportStream,
        status_code: StatusCode,
    ) -> Result<(), Error> {
        stream.write(status_code.status_line().as_bytes()).await?;
        stream.shutdown().await?;
        Ok(())
    }

    async fn parse_request(stream: &mut TransportStream) -> Result<HostAddress, ProtocolError> {
        use bytes::BytesMut;
        let mut buf = BytesMut::with_capacity(384);
        let _n = stream.read_buf(&mut buf).await?;

        let mut headers = [httparse::EMPTY_HEADER; 16];
        let mut request = httparse::Request::new(&mut headers);
        let status = request.parse(&buf.as_ref()).map_err(|_source| ProtocolError::BadRequest)?;

        let remote_host = {
            match status {
                httparse::Status::Partial => return Err(ProtocolError::BadRequest),
                httparse::Status::Complete(_parsed_len) => {
                    let method = request.method.ok_or(ProtocolError::BadRequest)?;
                    if method.to_uppercase() != "CONNECT" {
                        return Err(ProtocolError::UnsupportedMethod { method: method.to_owned() });
                    }

                    let mut remote_host =
                        request.path.map(|p| p.to_string()).ok_or(ProtocolError::BadRequest)?;

                    for header in request.headers {
                        if header.name.to_lowercase() == "host" {
                            remote_host = String::from_utf8_lossy(header.value).to_string();
                            break;
                        }
                    }

                    remote_host
                }
            }
        };

        let remote_host =
            HostAddress::from_str(&remote_host).map_err(|_| ProtocolError::BadRequest)?;

        Ok(remote_host)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let raw_request = r#"CONNECT cdnjs.cloudflare.com:443 HTTP/1.1
User-Agent: Mozilla/5.0 (X11; Linux x86_64; rv:77.0) Gecko/20100101 Firefox/77.0
Proxy-Connection: keep-alive
Connection: keep-alive
Host: cdnjs.cloudflare.com:443"#;

        let raw_request2 = r#"CONNECT www.google.com:443 HTTP/1.1
User-Agent: Mozilla/5.0 (X11; Linux x86_64; rv:77.0) Gecko/20100101 Firefox/77.0
Proxy-Connection: keep-alive
Connection: keep-alive
Host: www.google.com:443"#;

        let raw_request3 = r#"GET http://httpbin.org/ HTTP/1.1
Host: httpbin.org
User-Agent: Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:77.0) Gecko/20100101 Firefox/77.0
Accept: text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,*/*;q=0.8
Accept-Language: zh-TW,zh-CN;q=0.8,en-US;q=0.5,en;q=0.3
Accept-Encoding: gzip, deflate
DNT: 1
Connection: keep-alive
Upgrade-Insecure-Requests: 1
"#;

        let raw_request4 = r#"GET http://myip.com.tw/ HTTP/1.1
Host: myip.com.tw
User-Agent: Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:77.0) Gecko/20100101 Firefox/77.0
Accept: text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,*/*;q=0.8
Accept-Language: zh-TW,zh-CN;q=0.8,en-US;q=0.5,en;q=0.3
Accept-Encoding: gzip, deflate
DNT: 1
Connection: keep-alive
Upgrade-Insecure-Requests: 1
"#;
    }
}
