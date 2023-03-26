use std::{fmt, sync::Arc};

use snafu::ResultExt;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use url::Url;
use webpki::DNSNameRef;

use crate::{
    checker::error::{self, Error, ReportError},
    client::ProxyStream,
    common::{HostAddress, ProxyHost},
};

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
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

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct HttpProber {
    method: HttpMethod,
    url: Url,
    expected_response_code: u16,
}

impl HttpProber {
    #[inline]
    pub fn get(url: Url, expected_response_code: u16) -> HttpProber {
        HttpProber { url, expected_response_code, method: HttpMethod::Get }
    }

    #[inline]
    pub fn head(url: Url, expected_response_code: u16) -> HttpProber {
        HttpProber { url, expected_response_code, method: HttpMethod::Head }
    }

    #[inline]
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
        let stream = ProxyStream::connect_with_proxy(proxy_server, &destination)
            .await
            .context(error::ConnectProxyServer)?;
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
                let dnsname = DNSNameRef::try_from_ascii_str(&host).map_err(|source| {
                    Error::ConstructsDNSNameRef { source, name: host.to_owned() }
                })?;
                let stream =
                    config.connect(dnsname, stream).await.context(error::InitializeTlsStream)?;

                self.check_http(stream, report).await
            }
            scheme => Err(Error::UnknownScheme { scheme: scheme.to_owned() }),
        }
    }

    async fn check_http<Stream>(
        self,
        mut stream: Stream,
        report: &mut HttpProberReport,
    ) -> Result<(), Error>
    where
        Stream: Unpin + AsyncRead + AsyncWrite,
    {
        let request = self.build_request()?;
        stream.write(&request).await.context(error::WriteHttpRequest)?;

        let mut buf = vec![0u8; 1024];
        stream.read(&mut buf[..]).await.context(error::ReadHttpResponse)?;

        let mut headers = [httparse::EMPTY_HEADER; 32];
        let mut response = httparse::Response::new(&mut headers);

        let res = response.parse(&buf).context(error::ParseHttpResponse)?;
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

    #[inline]
    pub fn destination_address(&self) -> Result<HostAddress, Error> {
        Ok(HostAddress::new(&self.host()?, self.port()?))
    }

    #[inline]
    pub fn host(&self) -> Result<String, Error> {
        Ok(self.url.host_str().ok_or(Error::NoHostProvided)?.to_owned())
    }

    #[inline]
    pub fn port(&self) -> Result<u16, Error> {
        self.url.port_or_known_default().ok_or(Error::NoPortProvided)
    }

    #[inline]
    pub fn path(&self) -> Result<String, Error> { Ok(self.url.path().to_owned()) }

    #[inline]
    pub fn method(&self) -> HttpMethod { self.method }

    #[inline]
    pub fn url(&self) -> &Url { &self.url }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct HttpProberReport {
    pub destination_reachable: bool,
    pub method: Option<HttpMethod>,
    pub url: Option<Url>,
    pub response_code: Option<u16>,
    pub error: Option<ReportError>,
}

impl HttpProberReport {
    #[inline]
    pub fn timeout(method: HttpMethod, url: Url) -> Self {
        Self {
            destination_reachable: false,
            method: Some(method),
            url: Some(url),
            response_code: None,
            error: Some(ReportError::Timeout),
        }
    }

    #[inline]
    pub fn has_error(&self) -> bool { self.error.is_some() }
}
