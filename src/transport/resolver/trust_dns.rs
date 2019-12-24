use futures::FutureExt;
use tokio::runtime::Runtime;
use trust_dns_resolver::{
    config::{ResolverConfig, ResolverOpts},
    AsyncResolver, TokioAsyncResolver,
};

use crate::transport::{
    resolver::{Resolve, Resolver},
    Error,
};

#[derive(Clone)]
pub struct DefaultResolver {
    resolver: TokioAsyncResolver,
}

impl DefaultResolver {
    pub fn new(
        runtime: &mut Runtime,
        resolver_config: ResolverConfig,
        resolver_opts: ResolverOpts,
    ) -> Result<DefaultResolver, Error> {
        let resolver = {
            let runtime_handle = runtime.handle().clone();

            runtime.block_on(async move {
                match AsyncResolver::new(resolver_config, resolver_opts, runtime_handle.clone())
                    .await
                {
                    Ok(r) => Ok(r),
                    Err(err) => Err(Error::FailedToInitializeNameResolver(err.to_string())),
                }
            })?
        };

        Ok(DefaultResolver { resolver })
    }

    pub fn from_system_conf(runtime: &mut Runtime) -> Result<DefaultResolver, Error> {
        let resolver = {
            let runtime_handle = runtime.handle().clone();

            runtime.block_on(async move {
                match AsyncResolver::from_system_conf(runtime_handle.clone()).await {
                    Ok(r) => Ok(r),
                    Err(err) => Err(Error::FailedToInitializeNameResolver(err.to_string())),
                }
            })?
        };

        Ok(DefaultResolver { resolver })
    }
}

impl Resolver for DefaultResolver {
    fn resolve(&self, host: &str) -> Resolve {
        let host = host.to_owned();
        let resolver = self.resolver.clone();

        async move {
            let response = match resolver.lookup_ip(host).await {
                Ok(res) => res,
                Err(_err) => return Err(Error::NameResolver),
            };

            Ok(response.iter().collect())
        }
        .boxed()
    }
}
