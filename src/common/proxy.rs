use std::{fmt, str::FromStr};

use serde::{Deserialize, Serialize};
use snafu::Snafu;
use url::Url;

use crate::common::HostAddress;

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ProxyHost {
    Socks4a {
        host: String,
        port: u16,
        id: Option<String>,
    },
    Socks5 {
        host: String,
        port: u16,
        username: Option<String>,
        password: Option<String>,
    },
    HttpTunnel {
        host: String,
        port: u16,
        user_agent: Option<String>,
        username: Option<String>,
        password: Option<String>,
    },
}

impl ProxyHost {
    pub fn host(&self) -> &str {
        match self {
            Self::HttpTunnel { host, .. } => host,
            Self::Socks4a { host, .. } => host,
            Self::Socks5 { host, .. } => host,
        }
    }

    pub fn port(&self) -> u16 {
        match *self {
            Self::HttpTunnel { port, .. } => port,
            Self::Socks4a { port, .. } => port,
            Self::Socks5 { port, .. } => port,
        }
    }

    pub fn host_address(&self) -> HostAddress {
        let port = self.port();
        let host = self.host();
        HostAddress::new(host, port)
    }

    pub fn proxy_type_str(&self) -> &str {
        match self {
            Self::Socks4a { .. } => "socks4a",
            Self::Socks5 { .. } => "socks5",
            Self::HttpTunnel { .. } => "http",
        }
    }
}

impl FromStr for ProxyHost {
    type Err = ProxyHostError;

    fn from_str(url: &str) -> Result<Self, Self::Err> {
        let url = Url::parse(url)?;

        let host = url.host_str().ok_or(ProxyHostError::NoHostName)?.to_string();
        let port = url.port_or_known_default().ok_or(ProxyHostError::NoPortNumber)?;
        let username = (!url.username().is_empty()).then_some(url.username().to_string());
        let password = url.password().map(ToString::to_string);
        let host = match url.scheme() {
            "socks4a" | "socks4" => Self::Socks4a { host, port, id: None },
            "socks5" => Self::Socks5 { host, port, username, password },
            "http" => Self::HttpTunnel { host, port, username, password, user_agent: None },
            scheme => return Err(ProxyHostError::InvalidScheme { scheme: scheme.to_string() }),
        };

        Ok(host)
    }
}

impl fmt::Display for ProxyHost {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProxyHost::Socks4a { host, port, .. } => write!(f, "socks4a://{host}:{port}"),
            ProxyHost::Socks5 { host, port, .. } => write!(f, "socks5://{host}:{port}"),
            ProxyHost::HttpTunnel { host, port, .. } => write!(f, "http://{host}:{port}"),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum ProxyStrategy {
    Single(ProxyHost),
    Chained(Vec<ProxyHost>),
}

#[derive(Debug, Snafu)]
pub enum ProxyHostError {
    #[snafu(display("No host name"))]
    NoHostName,

    #[snafu(display("No port number"))]
    NoPortNumber,

    #[snafu(display("Could not parse URL, error: {source}"))]
    ParseUrlError { source: url::ParseError },

    #[snafu(display("Invalid scheme: {scheme}"))]
    InvalidScheme { scheme: String },
}

impl fmt::Display for ProxyStrategy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProxyStrategy::Single(proxy) => write!(f, "{proxy}"),
            ProxyStrategy::Chained(chain) => {
                let text = chain.iter().map(ToString::to_string).collect::<Vec<_>>().join(" ➔ ");
                write!(f, "[{text}]")
            }
        }
    }
}

impl From<url::ParseError> for ProxyHostError {
    fn from(source: url::ParseError) -> Self { Self::ParseUrlError { source } }
}
