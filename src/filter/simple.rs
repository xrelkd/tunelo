use std::{
    collections::HashSet,
    net::{IpAddr, SocketAddr},
};

use crate::{
    common::HostAddress,
    filter::{FilterAction, FilterMode, HostFilter},
};

#[derive(Clone, Debug, Default)]
pub struct SimpleFilter {
    hostnames: HashSet<String>,
    addresses: HashSet<IpAddr>,
    hosts: HashSet<(String, u16)>,
    sockets: HashSet<SocketAddr>,
    ports: HashSet<u16>,
    mode: FilterMode,
}

impl SimpleFilter {
    #[inline]
    pub fn new(
        hostnames: HashSet<String>,
        addresses: HashSet<IpAddr>,
        hosts: HashSet<(String, u16)>,
        sockets: HashSet<SocketAddr>,
        ports: HashSet<u16>,
        mode: FilterMode,
    ) -> Self {
        Self { hostnames, addresses, hosts, sockets, ports, mode }
    }

    #[inline]
    pub fn allow_list() -> Self { Self { mode: FilterMode::AllowList, ..Default::default() } }

    #[inline]
    pub fn deny_list() -> Self { Self { mode: FilterMode::DenyList, ..Default::default() } }

    pub fn set_mode(&mut self, mode: FilterMode) { self.mode = mode; }

    #[inline]
    pub fn add_socket(&mut self, socket: SocketAddr) { self.sockets.insert(socket); }

    #[inline]
    pub fn add_host(&mut self, host: &str, port: u16) {
        self.hosts.insert((host.to_owned(), port));
    }

    #[inline]
    pub fn add_hostname(&mut self, host: &str) { self.hostnames.insert(host.to_owned()); }

    #[inline]
    pub fn add_port(&mut self, port: u16) { self.ports.insert(port); }

    #[inline]
    pub fn add_address(&mut self, addr: IpAddr) { self.addresses.insert(addr); }

    #[inline]
    pub fn add_host_address(&mut self, addr: HostAddress) {
        match addr {
            HostAddress::Socket(socket) => self.add_socket(socket),
            HostAddress::DomainName(host, port) => self.add_host(&host, port),
        }
    }

    #[inline]
    fn filter(&self, b: bool) -> FilterAction {
        match self.mode {
            FilterMode::DenyList => Self::deny(b),
            FilterMode::AllowList => Self::allow(b),
        }
    }

    #[inline]
    fn allow(b: bool) -> FilterAction {
        if b {
            FilterAction::Allow
        } else {
            FilterAction::Deny
        }
    }

    #[inline]
    fn deny(b: bool) -> FilterAction {
        if b {
            FilterAction::Deny
        } else {
            FilterAction::Allow
        }
    }
}

impl HostFilter for SimpleFilter {
    #[inline]
    fn filter_port(&self, port: u16) -> FilterAction { self.filter(self.ports.contains(&port)) }

    #[inline]
    fn filter_hostname(&self, hostname: &str) -> FilterAction {
        self.filter(self.hostnames.contains(hostname))
    }

    #[inline]
    fn filter_address(&self, addr: &IpAddr) -> FilterAction {
        self.filter(self.addresses.contains(addr))
    }

    #[inline]
    fn filter_socket(&self, socket: &SocketAddr) -> FilterAction {
        self.filter(self.addresses.contains(&socket.ip()) || self.sockets.contains(socket))
    }

    #[inline]
    fn filter_host(&self, host: &str, port: u16) -> FilterAction {
        self.filter(self.hostnames.contains(host) || self.hosts.contains(&(host.to_owned(), port)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constructors() {
        let _filter = SimpleFilter::default();
        let _filter = SimpleFilter::allow_list();
        let _filter = SimpleFilter::deny_list();

        let port = 10001;
        let ip: IpAddr = "220.181.38.148".parse().unwrap();
        let hostname = "baidu.com";
        let socket = SocketAddr::new("127.0.3.1".parse().unwrap(), 9332);

        let filter = SimpleFilter::new(
            vec![hostname.to_owned()].into_iter().collect(),
            vec![ip].into_iter().collect(),
            vec![(hostname.to_owned(), port)].into_iter().collect(),
            vec![socket].into_iter().collect(),
            vec![port].into_iter().collect(),
            FilterMode::DenyList,
        );

        assert_eq!(filter.filter_port(port), FilterAction::Deny);
        assert_eq!(filter.filter_hostname(hostname), FilterAction::Deny);
        assert_eq!(filter.filter_address(&ip), FilterAction::Deny);
        assert_eq!(filter.filter_socket(&socket), FilterAction::Deny);
        assert_eq!(filter.filter_host(hostname, port), FilterAction::Deny);
    }

    #[test]
    fn filters() {
        let port = 18986;
        let ip: IpAddr = "220.181.38.148".parse().unwrap();
        let hostname = "baidu.com";
        let socket = SocketAddr::new("127.0.3.1".parse().unwrap(), 9332);
        let mut filter = SimpleFilter::default();

        filter.add_port(port);
        filter.add_address(ip);
        filter.add_socket(socket);
        filter.add_hostname(hostname);
        filter.add_host(hostname, port);

        assert_eq!(filter.filter_port(port), FilterAction::Deny);
        assert_eq!(filter.filter_hostname(hostname), FilterAction::Deny);
        assert_eq!(filter.filter_address(&ip), FilterAction::Deny);
        assert_eq!(filter.filter_socket(&socket), FilterAction::Deny);
        assert_eq!(filter.filter_host(hostname, port), FilterAction::Deny);

        filter.set_mode(FilterMode::AllowList);

        assert_eq!(filter.filter_port(port), FilterAction::Allow);
        assert_eq!(filter.filter_hostname(hostname), FilterAction::Allow);
        assert_eq!(filter.filter_address(&ip), FilterAction::Allow);
        assert_eq!(filter.filter_socket(&socket), FilterAction::Allow);
        assert_eq!(filter.filter_host(hostname, port), FilterAction::Allow);
    }
}
