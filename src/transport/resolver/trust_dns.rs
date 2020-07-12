use futures::FutureExt;
use tokio::runtime::Handle;
use trust_dns_resolver::{
    config::{ResolverConfig, ResolverOpts},
    AsyncResolver, TokioAsyncResolver,
};

use crate::transport::{
    resolver::{Resolve, Resolver},
    Error,
};

#[derive(Clone)]
pub struct TrustDnsResolver {
    resolver: TokioAsyncResolver,
}

impl TrustDnsResolver {
    pub async fn new(
        runtime_handle: Handle,
        resolver_config: ResolverConfig,
        resolver_opts: ResolverOpts,
    ) -> Result<TrustDnsResolver, Error> {
        AsyncResolver::new(resolver_config, resolver_opts, runtime_handle)
            .await
            .map(|resolver| TrustDnsResolver { resolver })
            .map_err(|err| Error::InitializeTrustDnsResolver { error: err.to_string() })
    }

    pub async fn from_system_conf(runtime_handle: Handle) -> Result<TrustDnsResolver, Error> {
        AsyncResolver::from_system_conf(runtime_handle)
            .await
            .map(|resolver| TrustDnsResolver { resolver })
            .map_err(|err| Error::InitializeTrustDnsResolver { error: err.to_string() })
    }
}

impl Resolver for TrustDnsResolver {
    fn resolve(&self, host: &str) -> Resolve {
        let host = host.to_owned();
        let resolver = self.resolver.clone();

        async move {
            let response = match resolver.lookup_ip(host).await {
                Ok(res) => res,
                Err(err) => return Err(Error::LookupTrustDnsResolver { error: err.to_string() }),
            };

            Ok(response.iter().collect())
        }
        .boxed()
    }
}
