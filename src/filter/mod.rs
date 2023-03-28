mod composer;
mod simple;

use std::net::{IpAddr, SocketAddr};

use crate::common::{HostAddress, ProxyHost, ProxyStrategy};

pub use self::{composer::ComposerFilter, simple::SimpleFilter};

#[derive(Clone, Copy, Debug, Default)]
pub enum FilterMode {
    AllowList,
    #[default]
    DenyList,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum FilterAction {
    Allow,
    Deny,
}

pub trait HostFilter: Send + Sync {
    fn filter_host_address(&self, addr: &HostAddress) -> FilterAction {
        match addr {
            HostAddress::Socket(socket) => self.filter_socket(socket),
            HostAddress::DomainName(host, port) => self.filter_host(host, *port),
        }
    }

    fn filter_socket(&self, socket: &SocketAddr) -> FilterAction;

    fn filter_host(&self, host: &str, port: u16) -> FilterAction;

    fn filter_hostname(&self, hostname: &str) -> FilterAction;

    fn filter_address(&self, addr: &IpAddr) -> FilterAction;

    fn filter_port(&self, port: u16) -> FilterAction;

    fn check_proxy_strategy(&self, strategy: &ProxyStrategy) -> (bool, Vec<HostAddress>) {
        match strategy {
            ProxyStrategy::Single(proxy) => {
                if self.filter_host_address(&proxy.host_address()) == FilterAction::Deny {
                    return (false, vec![proxy.host_address()]);
                }
            }
            ProxyStrategy::Chained(proxies) => {
                let denied: Vec<_> = proxies
                    .iter()
                    .filter(|proxy| {
                        self.filter_host_address(&proxy.host_address()) == FilterAction::Deny
                    })
                    .map(ProxyHost::host_address)
                    .collect();
                return (denied.is_empty(), denied);
            }
        }

        (true, vec![])
    }
}
