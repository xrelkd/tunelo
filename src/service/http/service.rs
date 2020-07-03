use std::{net::SocketAddr, sync::Arc};

use tokio::{
    io::{AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader},
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
            Err(ProtocolError::UnsupportedMethod(method)) => {
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
            Err(_err) => return Err(Error::Protocol(ProtocolError::HostUnreachable)),
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
        let mut buf_reader = BufReader::new(stream);
        let mut remote_host = {
            let mut line = String::new();
            match buf_reader.read_line(&mut line).await {
                Ok(0) | Err(_) => return Err(ProtocolError::BadRequest),
                Ok(_) => {}
            };

            let mut parts = line.trim().split_whitespace();
            match parts.next() {
                Some(method) if method.to_uppercase() != "CONNECT" => {
                    return Err(ProtocolError::UnsupportedMethod(method.to_owned()));
                }
                Some(_) => {}
                None => return Err(ProtocolError::BadRequest),
            }

            // get remote host
            let remote_host = match parts.next() {
                Some(r) => r,
                None => return Err(ProtocolError::BadRequest),
            };

            // HTTP version
            if parts.next().is_none() {
                return Err(ProtocolError::BadRequest);
            }

            remote_host.to_owned()
        };

        let mut line = String::new();
        while let Ok(len) = buf_reader.read_line(&mut line).await {
            if len == 0 || line == "\r\n" || line.trim_end().is_empty() {
                break;
            }

            let mut parts = line.split(": ");
            match parts.next() {
                Some(field) if field.trim().to_lowercase() == "host" => {
                    if let Some(host) = parts.next() {
                        remote_host = host.trim().to_owned();
                    }
                }
                Some(_) | None => {}
            }

            line.clear();
        }

        let remote_host = {
            let parts: Vec<_> = remote_host.split(':').collect();
            let host = parts[0];
            let port = match parts[1].parse() {
                Ok(p) => p,
                Err(_err) => return Err(ProtocolError::BadRequest),
            };

            HostAddress::DomainName(host.to_owned(), port)
        };

        Ok(remote_host)
    }
}
