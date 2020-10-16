use std::{future::Future, path::PathBuf, pin::Pin, sync::Arc};

use snafu::ResultExt;
use structopt::{clap::Shell as ClapShell, StructOpt};
use tokio::runtime;

use tunelo::transport::{Resolver, TrustDnsResolver};

use crate::{
    consts,
    error::{self, Error},
};

#[macro_use]
pub mod macros;

pub mod http_server;
pub mod multi_proxy;
pub mod proxy_chain;
pub mod proxy_checker;
pub mod socks_server;

#[derive(Debug, StructOpt)]
pub struct Command {
    #[structopt(long = "config", short = "c")]
    config_file: Option<PathBuf>,

    #[structopt(subcommand)]
    subcommand: Option<SubCommand>,
}

#[derive(Debug, StructOpt)]
pub enum SubCommand {
    #[structopt(about = "Shows current version")]
    Version,

    #[structopt(about = "Shows shell completions")]
    Completions { shell: ClapShell },

    #[structopt(about = "Starts multiple proxy server")]
    MultiProxy {
        #[structopt(long = "config", short = "c")]
        config_file: Option<PathBuf>,
    },

    #[structopt(about = "Runs as proxy chain server")]
    ProxyChain {
        #[structopt(long = "config", short = "c")]
        config_file: Option<PathBuf>,

        #[structopt(flatten)]
        options: proxy_chain::Options,
    },

    #[structopt(about = "Runs as proxy checker")]
    ProxyChecker {
        #[structopt(long = "config", short = "c")]
        config_file: Option<PathBuf>,

        #[structopt(flatten)]
        options: proxy_checker::Options,
    },

    #[structopt(about = "Runs as SOCKS proxy server")]
    SocksServer {
        #[structopt(long = "config", short = "c")]
        config_file: Option<PathBuf>,

        #[structopt(flatten)]
        options: socks_server::Options,
    },

    #[structopt(about = "Runs as HTTP proxy server")]
    HttpServer {
        #[structopt(long = "config", short = "c")]
        config_file: Option<PathBuf>,

        #[structopt(flatten)]
        options: http_server::Options,
    },
}

impl Command {
    #[inline]
    pub fn new() -> Command { Command::from_args() }

    #[inline]
    pub fn app_name() -> String { Command::clap().get_name().to_owned() }

    pub fn run(self) -> Result<(), Error> {
        {
            use tracing_subscriber::prelude::*;

            let timer = tracing_subscriber::fmt::time::ChronoUtc::rfc3339();
            let fmt_layer = tracing_subscriber::fmt::layer().with_timer(timer).with_target(true);
            let filter_layer = tracing_subscriber::EnvFilter::try_from_default_env()
                .or_else(|_| tracing_subscriber::EnvFilter::try_new("info"))
                .unwrap();

            tracing_subscriber::registry().with(filter_layer).with(fmt_layer).init();
        }

        match self.subcommand {
            Some(SubCommand::Version) => {
                Command::clap()
                    .write_version(&mut std::io::stdout())
                    .expect("failed to write to stdout");
                Ok(())
            }
            Some(SubCommand::Completions { shell }) => {
                let app_name = Command::app_name();
                Command::clap().gen_completions_to(app_name, shell, &mut std::io::stdout());
                Ok(())
            }
            Some(SubCommand::ProxyChain { options, config_file }) => {
                execute(move |resolver| Box::pin(proxy_chain::run(resolver, options, config_file)))
            }
            Some(SubCommand::SocksServer { options, config_file }) => {
                execute(move |resolver| Box::pin(socks_server::run(resolver, options, config_file)))
            }
            Some(SubCommand::HttpServer { options, config_file }) => {
                execute(move |resolver| Box::pin(http_server::run(resolver, options, config_file)))
            }
            Some(SubCommand::ProxyChecker { options, config_file }) => {
                execute(move |_resolver| Box::pin(proxy_checker::run(options, config_file)))
            }
            Some(SubCommand::MultiProxy { config_file }) => {
                execute(move |resolver| Box::pin(multi_proxy::run(resolver, config_file)))
            }
            None => execute(move |resolver| Box::pin(multi_proxy::run(resolver, self.config_file))),
        }
    }
}

#[inline]
fn execute<F>(f: F) -> Result<(), Error>
where
    F: FnOnce(Arc<dyn Resolver>) -> Pin<Box<dyn Future<Output = Result<(), Error>>>>,
{
    let long_version = {
        let mut buf = vec![];
        Command::clap().write_long_version(&mut buf).expect("failed to write version to buffer");
        String::from_utf8_lossy(&buf).to_string()
    };
    tracing::info!("Starting {}", long_version);

    tracing::info!("Initializing Tokio runtime");
    let mut runtime = runtime::Builder::new()
        .thread_name(consts::THREAD_NAME)
        .threaded_scheduler()
        .enable_all()
        .build()
        .context(error::InitializeTokioRuntime)?;

    let resolver = {
        let handle = runtime.handle().clone();
        runtime
            .block_on(async move {
                tracing::info!("Initializing domain name resolver");

                match TrustDnsResolver::from_system_conf(handle.clone()).await {
                    Ok(resolver) => Ok(resolver),
                    Err(err) => {
                        warn!(
                            "Failed to initialize domain name resolver from system configuration, \
                             try to initialize with fallback option, error: {}",
                            err
                        );
                        TrustDnsResolver::new_default(handle).await
                    }
                }
            })
            .context(error::InitializeDomainNameResolver)?
    };

    runtime.block_on(f(Arc::new(resolver)))
}
