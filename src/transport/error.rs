use snafu::Snafu;

use crate::{client, common::HostAddress};

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("StdIo error: {}", source))]
    StdIo { source: std::io::Error },

    #[snafu(display("ProxyClient error: {}", source))]
    ProxyClient { source: client::Error },

    #[snafu(display("Could not resolve domain name: {}", domain_name))]
    ResolveDomainName { domain_name: String },

    #[snafu(display("Connect to forbidden hosts: {:?}", hosts))]
    ConnectForbiddenHosts { hosts: Vec<HostAddress> },

    #[snafu(display("Could not initialize trust_dns_resolver, error: {}", error))]
    InitializeTrustDnsResolver { error: String },

    #[snafu(display("Could not resolve domain name via trust_dns_resolver, error: {}", error))]
    LookupTrustDnsResolver { error: String },
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Error { Error::StdIo { source: err } }
}

impl From<client::Error> for Error {
    fn from(err: client::Error) -> Error { Error::ProxyClient { source: err } }
}
