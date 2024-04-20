use std::{
    net::{IpAddr, SocketAddr},
    sync::Arc,
};

use crate::filter::{FilterAction, HostFilter};

#[derive(Default)]
pub struct ComposerFilter {
    filters: Vec<Arc<dyn HostFilter>>,
}

impl ComposerFilter {
    #[inline]
    #[must_use]
    pub fn new() -> Self { Self::default() }

    #[inline]
    pub fn add_filter(&mut self, filter: Arc<dyn HostFilter>) { self.filters.push(filter); }

    #[inline]
    fn filter<F: FnMut(&Arc<dyn HostFilter>) -> bool>(&self, predictor: F) -> FilterAction {
        if self.filters.iter().any(predictor) {
            return FilterAction::Deny;
        }
        FilterAction::Allow
    }

    #[inline]
    #[must_use]
    pub fn destruct(self) -> Vec<Arc<dyn HostFilter>> { self.filters }
}

impl HostFilter for ComposerFilter {
    #[inline]
    fn filter_port(&self, port: u16) -> FilterAction {
        self.filter(|filter| filter.filter_port(port) == FilterAction::Deny)
    }

    #[inline]
    fn filter_hostname(&self, hostname: &str) -> FilterAction {
        self.filter(|filter| filter.filter_hostname(hostname) == FilterAction::Deny)
    }

    #[inline]
    fn filter_address(&self, addr: &IpAddr) -> FilterAction {
        self.filter(|filter| filter.filter_address(addr) == FilterAction::Deny)
    }

    #[inline]
    fn filter_socket(&self, socket: &SocketAddr) -> FilterAction {
        self.filter(|filter| filter.filter_socket(socket) == FilterAction::Deny)
    }

    #[inline]
    fn filter_host(&self, host: &str, port: u16) -> FilterAction {
        self.filter(|filter| filter.filter_host(host, port) == FilterAction::Deny)
    }
}

#[cfg(test)]
mod tests {
    use std::net::{IpAddr, SocketAddr};

    use super::*;
    use crate::filter::{FilterMode, SimpleFilter};

    #[test]
    fn construct() {
        let port = 10001;
        let ip: IpAddr = "220.181.38.148".parse().unwrap();
        let hostname = "baidu.com";
        let socket = SocketAddr::new("127.0.3.1".parse().unwrap(), 9332);

        let simple_filter = SimpleFilter::new(
            vec![hostname.to_owned()].into_iter().collect(),
            vec![ip].into_iter().collect(),
            vec![(hostname.to_owned(), port)].into_iter().collect(),
            vec![socket].into_iter().collect(),
            vec![port].into_iter().collect(),
            FilterMode::DenyList,
        );

        let mut composer = ComposerFilter::new();
        composer.add_filter(Arc::new(simple_filter));
        composer.add_filter(Arc::new(ComposerFilter::new()));
    }

    #[test]
    fn filters() {
        let port = 18986;
        let ip: IpAddr = "220.181.38.148".parse().unwrap();
        let hostname = "baidu.com";
        let socket = SocketAddr::new("127.0.3.1".parse().unwrap(), 9332);
        let mut simple = SimpleFilter::default();

        simple.add_port(port);
        simple.add_address(ip);
        simple.add_socket(socket);
        simple.add_hostname(hostname);
        simple.add_host(hostname, port);

        let mut composer = ComposerFilter::new();
        composer.add_filter(Arc::new(simple));

        assert_eq!(composer.filter_port(port), FilterAction::Deny);
        assert_eq!(composer.filter_hostname(hostname), FilterAction::Deny);
        assert_eq!(composer.filter_address(&ip), FilterAction::Deny);
        assert_eq!(composer.filter_socket(&socket), FilterAction::Deny);
        assert_eq!(composer.filter_host(hostname, port), FilterAction::Deny);

        assert_eq!(composer.filter_port(port + 1), FilterAction::Allow);
    }
}
