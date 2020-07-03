use std::net::ToSocketAddrs;

use futures::FutureExt;

use crate::transport::{
    resolver::{Resolve, Resolver},
    Error,
};

#[derive(Clone)]
pub struct TokioResolver {}

impl TokioResolver {
    pub fn new() -> Result<TokioResolver, Error> { Ok(TokioResolver {}) }
}

impl Resolver for TokioResolver {
    fn resolve(&self, host: &str) -> Resolve {
        use tokio::task::spawn_blocking;

        let host = host.to_owned();
        async move {
            let res = spawn_blocking(move || {
                (host.as_str(), 0)
                    .to_socket_addrs()
                    .unwrap_or(vec![].into_iter())
                    .into_iter()
                    .map(|addr| addr.ip().clone())
                    .collect()
            })
            .await;

            match res {
                Ok(addrs) => Ok(addrs),
                Err(_err) => Err(Error::FailedToResolveDomainName),
            }
        }
        .boxed()
    }
}

// impl SharedResolver for TokioResolver {
//     fn clone(&self) -> Self {
//         Clone::clone(self)
//     }
// }
