use std::net::IpAddr;
use std::pin::Pin;

use futures::Future;

use crate::transport::Error;

mod tokio_dns;
mod trust_dns;

pub use self::tokio_dns::TokioResolver;
pub use self::trust_dns::DefaultResolver;

pub type Resolve = Pin<Box<dyn Future<Output = Result<Vec<IpAddr>, Error>> + Send>>;

pub trait Resolver: Send + Sync {
    fn resolve(&self, host: &str) -> Resolve;
}

// pub type SharedResolver = Arc<Box<dyn Resolver>>;

#[derive(Clone)]
pub struct DummyResolver {}

impl DummyResolver {
    pub fn new() -> DummyResolver {
        DummyResolver {}
    }
}

impl Resolver for DummyResolver {
    fn resolve(&self, _host: &str) -> Resolve {
        Box::pin(futures::future::lazy(|_| Ok(vec![IpAddr::from([0, 0, 0, 0])])))
    }
}
