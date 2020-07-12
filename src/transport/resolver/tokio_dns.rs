use std::net::ToSocketAddrs;

use futures::FutureExt;

use crate::transport::{
    resolver::{Resolve, Resolver},
    Error,
};

#[derive(Clone)]
pub struct TokioResolver;

impl TokioResolver {
    pub fn new() -> Result<TokioResolver, Error> { Ok(TokioResolver) }
}

impl Resolver for TokioResolver {
    fn resolve(&self, host: &str) -> Resolve {
        use tokio::task::spawn_blocking;

        let host = host.to_owned();
        async move {
            let res = spawn_blocking({
                let host = host.clone();
                move || {
                    (host.as_str(), 0)
                        .to_socket_addrs()
                        .unwrap_or(vec![].into_iter())
                        .into_iter()
                        .map(|addr| addr.ip().clone())
                        .collect()
                }
            })
            .await;

            res.map_err(|_err| Error::ResolveDomainName { domain_name: host.to_owned() })
        }
        .boxed()
    }
}
