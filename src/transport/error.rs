use std::path::PathBuf;

use snafu::Snafu;

use crate::{client, common::HostAddress};

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("Could not open file {}, error: {}", file_path.display(), source))]
    OpenFile { file_path: PathBuf, source: std::io::Error },

    #[snafu(display("Could not connect remote server {}, error: {}", host, source))]
    ConnectRemoteServer { host: HostAddress, source: std::io::Error },

    #[snafu(display("Could not create proxy connector, error: {}", source))]
    CreateProxyConnector { source: client::Error },

    #[snafu(display("Could not connect proxy server, error: {}", source))]
    ConnectProxyServer { source: client::Error },

    #[snafu(display("Could not resolve domain name: {}", domain_name))]
    ResolveDomainName { domain_name: String },

    #[snafu(display("Connect to forbidden hosts: {:?}", hosts))]
    ConnectForbiddenHosts { hosts: Vec<HostAddress> },

    #[snafu(display("Could not initialize trust_dns_resolver, error: {}", error))]
    InitializeTrustDnsResolver { error: String },

    #[snafu(display("Could not resolve domain name via trust_dns_resolver, error: {}", error))]
    LookupTrustDnsResolver { error: String },
}
