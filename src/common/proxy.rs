use crate::common::HostAddress;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ProxyHost {
    Socks4a {
        server: HostAddress,
        id: Option<String>,
    },
    Socks5 {
        server: HostAddress,
        user_name: Option<String>,
        password: Option<String>,
    },
    HttpTunnel {
        server: HostAddress,
        user_agent: Option<String>,
        user_name: Option<String>,
        password: Option<String>,
    },
}

impl ProxyHost {
    pub fn host_address(&self) -> &HostAddress {
        match self {
            ProxyHost::HttpTunnel { server, .. } => &server,
            ProxyHost::Socks4a { server, .. } => &server,
            ProxyHost::Socks5 { server, .. } => &server,
        }
    }
}

#[derive(Debug)]
pub enum ProxyStrategy {
    Single(ProxyHost),
    Chained(Vec<ProxyHost>),
}
