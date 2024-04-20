use std::net::ToSocketAddrs;

use futures::FutureExt;
use tokio::task;

use crate::transport::{
    resolver::{Resolve, Resolver},
    Error,
};

#[derive(Clone)]
pub struct TokioResolver;

impl TokioResolver {
    pub const fn new() -> Self { Self }
}

impl Resolver for TokioResolver {
    fn resolve(&self, host: &str) -> Resolve {
        let host = host.to_owned();
        async move {
            let res = task::spawn_blocking({
                let host = host.clone();
                move || {
                    (host.as_str(), 0)
                        .to_socket_addrs()
                        .unwrap_or_else(|_| vec![].into_iter())
                        .map(|addr| addr.ip())
                        .collect()
                }
            })
            .await;

            res.map_err(|_err| Error::ResolveDomainName { domain_name: host.clone() })
        }
        .boxed()
    }
}
