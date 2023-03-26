use futures::FutureExt;
use snafu::ResultExt;
use trust_dns_resolver::{
    config::{ResolverConfig, ResolverOpts},
    AsyncResolver, TokioAsyncResolver,
};

use crate::transport::{
    error,
    resolver::{Resolve, Resolver},
    Error,
};

#[derive(Clone)]
pub struct TrustDnsResolver {
    resolver: TokioAsyncResolver,
}

impl TrustDnsResolver {
    pub async fn new(
        resolver_config: ResolverConfig,
        resolver_opts: ResolverOpts,
    ) -> Result<TrustDnsResolver, Error> {
        AsyncResolver::tokio(resolver_config, resolver_opts)
            .map(|resolver| TrustDnsResolver { resolver })
            .context(error::InitializeTrustDnsResolverSnafu)
    }

    pub async fn new_default() -> Result<TrustDnsResolver, Error> {
        Self::new(ResolverConfig::default(), ResolverOpts::default()).await
    }

    pub async fn from_system_conf() -> Result<TrustDnsResolver, Error> {
        AsyncResolver::tokio_from_system_conf()
            .map(|resolver| TrustDnsResolver { resolver })
            .context(error::InitializeTrustDnsResolverSnafu)
    }
}

impl Resolver for TrustDnsResolver {
    fn resolve(&self, host: &str) -> Resolve {
        let host = host.to_owned();
        let resolver = self.resolver.clone();

        async move {
            let response =
                resolver.lookup_ip(host).await.context(error::LookupTrustDnsResolverSnafu)?;

            Ok(response.iter().collect())
        }
        .boxed()
    }
}
