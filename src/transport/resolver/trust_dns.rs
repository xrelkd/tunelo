use futures::FutureExt;
use snafu::ResultExt;
use trust_dns_resolver::{
    AsyncResolver, TokioAsyncResolver,
    config::{ResolverConfig, ResolverOpts},
};

use crate::transport::{
    Error, error,
    resolver::{Resolve, Resolver},
};

#[derive(Clone)]
pub struct TrustDnsResolver {
    resolver: TokioAsyncResolver,
}

impl TrustDnsResolver {
    /// Creates a new `TrustDnsResolver` with custom configuration.
    ///
    /// # Errors
    ///
    /// Returns an error if the resolver cannot be initialized.
    pub fn new(
        resolver_config: ResolverConfig,
        resolver_opts: ResolverOpts,
    ) -> Result<Self, Error> {
        let resolver = AsyncResolver::tokio(resolver_config, resolver_opts);
        Ok(Self { resolver })
    }

    /// Creates a new `TrustDnsResolver` with default configuration.
    ///
    /// # Errors
    ///
    /// Returns an error if the resolver cannot be initialized.
    pub fn new_default() -> Result<Self, Error> {
        Self::new(ResolverConfig::default(), ResolverOpts::default())
    }

    /// Creates a new `TrustDnsResolver` from system configuration.
    ///
    /// # Errors
    ///
    /// Returns an error if the resolver cannot be initialized from system
    /// config.
    pub fn from_system_conf() -> Result<Self, Error> {
        AsyncResolver::tokio_from_system_conf()
            .map(|resolver| Self { resolver })
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
