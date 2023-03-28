#[macro_use]
pub mod macros;
pub mod http_server;
pub mod multi_proxy;
pub mod proxy_chain;
pub mod proxy_checker;
pub mod socks_server;

use std::{future::Future, io::Write, path::PathBuf, pin::Pin, sync::Arc};

use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::Shell;
use snafu::ResultExt;
use tokio::runtime;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use tunelo::transport::{Resolver, TrustDnsResolver};

use crate::{
    consts,
    error::{self, Error},
};

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[arg(long = "config", short = 'c')]
    config_file: Option<PathBuf>,

    #[command(subcommand)]
    commands: Option<Commands>,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    #[command(about = "Show current version")]
    Version,

    #[command(about = "Show shell completions")]
    Completions { shell: Shell },

    #[command(about = "Starts multiple proxy server")]
    MultiProxy {
        #[arg(long = "config", short = 'c')]
        config_file: Option<PathBuf>,
    },

    #[command(about = "Run as proxy chain server")]
    ProxyChain {
        #[arg(long = "config", short = 'c')]
        config_file: Option<PathBuf>,

        #[clap(flatten)]
        options: proxy_chain::Options,
    },

    #[command(about = "Run as proxy checker")]
    ProxyChecker {
        #[arg(long = "config", short = 'c')]
        config_file: Option<PathBuf>,

        #[clap(flatten)]
        options: proxy_checker::Options,
    },

    #[command(about = "Run as SOCKS proxy server")]
    SocksServer {
        #[arg(long = "config", short = 'c')]
        config_file: Option<PathBuf>,

        #[clap(flatten)]
        options: socks_server::Options,
    },

    #[command(about = "Run as HTTP proxy server")]
    HttpServer {
        #[arg(long = "config", short = 'c')]
        config_file: Option<PathBuf>,

        #[clap(flatten)]
        options: http_server::Options,
    },
}

impl Default for Cli {
    #[inline]
    fn default() -> Self { Self::parse() }
}

impl Cli {
    pub fn run(self) -> Result<(), Error> {
        match self.commands {
            Some(Commands::Version) => {
                let mut stdout = std::io::stdout();
                stdout
                    .write_all(Self::command().render_long_version().as_bytes())
                    .expect("failed to write to stdout");
                Ok(())
            }
            Some(Commands::Completions { shell }) => {
                let mut app = Self::command();
                let bin_name = app.get_name().to_string();
                clap_complete::generate(shell, &mut app, bin_name, &mut std::io::stdout());
                Ok(())
            }
            Some(Commands::ProxyChain { options, config_file }) => {
                execute(move |resolver| Box::pin(proxy_chain::run(resolver, options, config_file)))
            }
            Some(Commands::SocksServer { options, config_file }) => {
                execute(move |resolver| Box::pin(socks_server::run(resolver, options, config_file)))
            }
            Some(Commands::HttpServer { options, config_file }) => {
                execute(move |resolver| Box::pin(http_server::run(resolver, options, config_file)))
            }
            Some(Commands::ProxyChecker { options, config_file }) => {
                execute(move |_resolver| Box::pin(proxy_checker::run(options, config_file)))
            }
            Some(Commands::MultiProxy { config_file }) => {
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
    init_tracing();

    tracing::info!("Starting {}", Cli::command().get_long_version().unwrap_or_default());

    tracing::info!("Initializing Tokio runtime");
    let runtime = runtime::Builder::new_multi_thread()
        .thread_name(consts::THREAD_NAME)
        .enable_all()
        .build()
        .context(error::InitializeTokioRuntimeSnafu)?;

    let resolver = {
        runtime
            .block_on(async move {
                tracing::info!("Initializing domain name resolver");

                match TrustDnsResolver::from_system_conf().await {
                    Ok(resolver) => Ok(resolver),
                    Err(err) => {
                        tracing::warn!(
                            "Failed to initialize domain name resolver from system configuration, \
                             try to initialize with fallback option, error: {err}"
                        );
                        TrustDnsResolver::new_default().await
                    }
                }
            })
            .context(error::InitializeDomainNameResolverSnafu)?
    };

    runtime.block_on(f(Arc::new(resolver)))
}

fn init_tracing() {
    // filter
    let filter_layer = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));

    // format
    let fmt_layer =
        tracing_subscriber::fmt::layer().pretty().with_thread_ids(true).with_thread_names(true);
    // subscriber
    tracing_subscriber::registry().with(filter_layer).with(fmt_layer).init();
}
