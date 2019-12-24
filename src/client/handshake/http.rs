use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use crate::client::handshake::{ClientHandshake, Error};
use crate::common::HostAddress;

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
        Stream: AsyncRead + AsyncReadExt + AsyncWrite + AsyncWriteExt + Unpin,
    {
        let request = {
            let host = target_host.to_string();

            let mut req = format!("CONNECT {} HTTP/1.1\r\n", host);
            req.push_str(&format!("Host: {}\r\n", host));

            if let Some(ua) = user_agent {
                req.push_str(&format!("User-Agent: {}\r\n", ua));
            }

            req.push_str("\r\n");
            req
        };
        self.stream.write(request.as_bytes()).await?;

        let mut lines_reader = tokio::io::BufReader::new(&mut self.stream).lines();
        match lines_reader.next_line().await? {
            None => return Err(Error::BadHttpResponse),
            Some(line) => {
                let mut parts = line.split_whitespace();

                // HTTP version
                if let None = parts.next() {
                    return Err(Error::BadHttpResponse);
                }

                // status code
                match parts.next() {
                    Some(status_code) => match status_code {
                        "200" => {}
                        "401" | "402" | "403" | "404" => return Err(Error::HostUnreachable),
                        _ => return Err(Error::HostUnreachable),
                    },
                    None => return Err(Error::BadHttpResponse),
                }
            }
        }

        while let Some(line) = lines_reader.next_line().await? {
            if line.is_empty() {
                break;
            }
        }

        Ok(())
    }
}
