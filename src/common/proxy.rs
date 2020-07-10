use std::fmt;

use snafu::Snafu;
use url::Url;

use crate::common::HostAddress;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
            ProxyHost::HttpTunnel { host, .. } => host,
            ProxyHost::Socks4a { host, .. } => host,
            ProxyHost::Socks5 { host, .. } => host,
        }
    }

    pub fn port(&self) -> u16 {
        match *self {
            ProxyHost::HttpTunnel { port, .. } => port,
            ProxyHost::Socks4a { port, .. } => port,
            ProxyHost::Socks5 { port, .. } => port,
        }
    }

    pub fn host_address(&self) -> HostAddress {
        let port = self.port();
        let host = self.host();
        HostAddress::new(host, port)
    }

    pub fn proxy_type_str(&self) -> &str {
        match self {
            ProxyHost::Socks4a { .. } => "socks4a",
            ProxyHost::Socks5 { .. } => "socks5",
            ProxyHost::HttpTunnel { .. } => "http",
        }
    }
}

impl std::str::FromStr for ProxyHost {
    type Err = ProxyHostError;

    fn from_str(url: &str) -> Result<ProxyHost, Self::Err> {
        let url = Url::parse(url)?;

        let host = url.host_str().ok_or(ProxyHostError::NoHostName)?.to_owned();
        let port = url.port_or_known_default().ok_or(ProxyHostError::NoPortNumber)?;
        let username =
            if !url.username().is_empty() { Some(url.username().to_owned()) } else { None };
        let password = url.password().map(|p| p.to_owned());
        let host = match url.scheme() {
            "socks4a" | "socks4" => ProxyHost::Socks4a { host, port, id: None },
            "socks5" => ProxyHost::Socks5 { host, port, username, password },
            "http" => ProxyHost::HttpTunnel { host, port, username, password, user_agent: None },
            scheme => return Err(ProxyHostError::InvalidScheme { scheme: scheme.to_owned() }),
        };

        Ok(host)
    }
}

impl fmt::Display for ProxyHost {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProxyHost::Socks4a { host, port, .. } => write!(f, "socks4a://{}:{}", host, port),
            ProxyHost::Socks5 { host, port, .. } => write!(f, "socks5://{}:{}", host, port),
            ProxyHost::HttpTunnel { host, port, .. } => write!(f, "http://{}:{}", host, port),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
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

    #[snafu(display("Could not parse URL, error: {}", source))]
    ParseUrlError { source: url::ParseError },

    #[snafu(display("Invalid scheme: {}", scheme))]
    InvalidScheme { scheme: String },
}

impl fmt::Display for ProxyStrategy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProxyStrategy::Single(proxy) => write!(f, "{}", proxy),
            ProxyStrategy::Chained(chain) => {
                let text =
                    chain.iter().map(|proxy| format!("{}", proxy)).collect::<Vec<_>>().join(" ➔ ");
                write!(f, "[{}]", text)
            }
        }
    }
}

impl From<url::ParseError> for ProxyHostError {
    fn from(source: url::ParseError) -> ProxyHostError { ProxyHostError::ParseUrlError { source } }
}
