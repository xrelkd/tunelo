use std::{fmt, sync::Arc};

use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use url::Url;
use webpki::DNSNameRef;

use crate::{
    checker::error::{Error, ReportError},
    client::ProxyStream,
    common::{HostAddress, ProxyHost},
};

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub enum HttpMethod {
    Head,
    Get,
    Delete,
}

impl fmt::Display for HttpMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HttpMethod::Get => write!(f, "GET"),
            HttpMethod::Head => write!(f, "HEAD"),
            HttpMethod::Delete => write!(f, "DELETE"),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct HttpProber {
    method: HttpMethod,
    url: Url,
    expected_response_code: u16,
}

impl HttpProber {
    pub fn get(url: Url, expected_response_code: u16) -> HttpProber {
        HttpProber { url, expected_response_code, method: HttpMethod::Get }
    }

    pub fn head(url: Url, expected_response_code: u16) -> HttpProber {
        HttpProber { url, expected_response_code, method: HttpMethod::Head }
    }

    pub fn delete(url: Url, expected_response_code: u16) -> HttpProber {
        HttpProber { url, expected_response_code, method: HttpMethod::Delete }
    }

    pub async fn probe(
        self,
        proxy_server: &ProxyHost,
        report: &mut HttpProberReport,
    ) -> Result<(), Error> {
        report.url = Some(self.url.clone());
        report.method = Some(self.method);

        let destination = self.destination_address()?;
        let stream = ProxyStream::connect_with_proxy(&proxy_server, &destination)
            .await
            .map_err(|source| Error::ConnectProxyServer { source })?;
        report.destination_reachable = true;

        let stream = stream.into_inner();
        match self.url.scheme() {
            "http" => self.check_http(stream, report).await,
            "https" => {
                use tokio_rustls::{rustls::ClientConfig, TlsConnector};

                let host = self.host()?;
                let mut config = ClientConfig::new();
                config.root_store.add_server_trust_anchors(&webpki_roots::TLS_SERVER_ROOTS);
                let config = TlsConnector::from(Arc::new(config));
                let dnsname = DNSNameRef::try_from_ascii_str(&host).unwrap();
                let stream = config
                    .connect(dnsname, stream)
                    .await
                    .map_err(|source| Error::InitializeTlsStream { source })?;

                self.check_http(stream, report).await
            }
            scheme => Err(Error::UnknownScheme { scheme: scheme.to_owned() }),
        }
    }

    async fn check_http<Stream: Unpin + AsyncRead + AsyncWrite>(
        self,
        mut stream: Stream,
        report: &mut HttpProberReport,
    ) -> Result<(), Error> {
        let request = self.build_request()?;
        stream.write(&request).await.map_err(|source| Error::WriteHttpRequest { source })?;

        let mut buf = vec![0u8; 1024];
        stream.read(&mut buf[..]).await.map_err(|source| Error::ReadHttpResponse { source })?;

        let mut headers = [httparse::EMPTY_HEADER; 32];
        let mut response = httparse::Response::new(&mut headers);

        let res = response.parse(&buf).map_err(|source| Error::ParseHttpResponse { source })?;
        if res.is_complete() {
            stream.shutdown();
            report.response_code = response.code;
            return Ok(());
        }

        Err(Error::IncompleteHttpResponse)
    }

    fn build_request(&self) -> Result<Vec<u8>, Error> {
        let host = self.host()?;
        let path = self.path()?;

        let req = match self.method {
            HttpMethod::Get => {
                format!("GET {} HTTP/1.1\r\nHost: {}\r\n\r\n", path, host).into_bytes()
            }
            HttpMethod::Head => {
                format!("HEAD {} HTTP/1.1\r\nHost: {}\r\n\r\n", path, host).into_bytes()
            }
            HttpMethod::Delete => {
                format!("DELETE {} HTTP/1.1\r\nHost: {}\r\n\r\n", path, host).into_bytes()
            }
        };

        Ok(req)
    }

    pub fn destination_address(&self) -> Result<HostAddress, Error> {
        Ok(HostAddress::new(&self.host()?, self.port()?))
    }

    pub fn host(&self) -> Result<String, Error> {
        Ok(self.url.host_str().ok_or(Error::NoHostProvided)?.to_owned())
    }

    pub fn port(&self) -> Result<u16, Error> {
        Ok(self.url.port_or_known_default().ok_or(Error::NoPortProvided)?)
    }

    pub fn path(&self) -> Result<String, Error> { Ok(self.url.path().to_owned()) }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct HttpProberReport {
    pub destination_reachable: bool,
    pub method: Option<HttpMethod>,
    pub url: Option<Url>,
    pub response_code: Option<u16>,
    pub error: Option<ReportError>,
}

impl HttpProberReport {
    #[inline]
    pub fn has_error(&self) -> bool { self.error.is_some() }
}

impl Default for HttpProberReport {
    fn default() -> HttpProberReport {
        HttpProberReport {
            destination_reachable: false,
            method: None,
            url: None,
            response_code: None,
            error: None,
        }
    }
}
