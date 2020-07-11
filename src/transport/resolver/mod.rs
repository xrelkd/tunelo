use std::{net::IpAddr, pin::Pin};

use futures::Future;

use crate::transport::Error;

mod tokio_dns;
mod trust_dns;

pub use self::{tokio_dns::TokioResolver, trust_dns::DefaultResolver};

pub type Resolve = Pin<Box<dyn Future<Output = Result<Vec<IpAddr>, Error>> + Send>>;

pub trait Resolver: Send + Sync {
    fn resolve(&self, host: &str) -> Resolve;
}

#[derive(Clone)]
pub struct DummyResolver;

impl DummyResolver {
    pub fn new() -> DummyResolver {
        DummyResolver
    }
}

impl Resolver for DummyResolver {
    fn resolve(&self, _host: &str) -> Resolve {
        Box::pin(futures::future::lazy(|_| Ok(vec![IpAddr::from([0, 0, 0, 0])])))
    }
}

#[cfg(test)]
mod tests {
    use tokio::runtime::Runtime;

    use super::*;

    #[test]
    fn dummy_resolver() -> Result<(), Box<dyn std::error::Error>> {
        let resolver = DummyResolver::new();

        let result =
            Runtime::new()?.block_on(async move { resolver.resolve("www.google.com").await })?;
        assert_eq!(result, vec![IpAddr::from([0, 0, 0, 0])]);

        Ok(())
    }
}
