use std::{convert::TryInto, future::Future, path::PathBuf, pin::Pin, sync::Arc};

use structopt::{clap::Shell as ClapShell, StructOpt};

use tunelo::transport::{DefaultResolver, Resolver};

use crate::{config::Config, error::Error};

mod http_server;
mod multi_server;
pub mod options;
mod proxy_chain;
mod proxy_checker;
mod socks_server;

#[derive(Debug, StructOpt)]
pub enum Command {
    #[structopt(about = "Shows current version")]
    Version,

    #[structopt(about = "Shows shell completions")]
    Completions { shell: ClapShell },

    #[structopt(about = "Runs as proxy checker")]
    ProxyChecker(options::ProxyCheckerOptions),

    #[structopt(about = "Starts multiple proxy server")]
    MultiServer {
        #[structopt(long = "config", short = "c")]
        config_file: Option<PathBuf>,
    },

    #[structopt(about = "Runs as SOCKS proxy server")]
    SocksServer(options::SocksOptions),

    #[structopt(about = "Runs as HTTP proxy server")]
    HttpServer(options::HttpOptions),

    #[structopt(about = "Runs as proxy chain server")]
    ProxyChain(options::ProxyChainOptions),
}

impl Command {
    #[inline]
    pub fn new() -> Command {
        Command::from_args()
    }

    #[inline]
    pub fn app_name() -> String {
        Command::clap().get_name().to_owned()
    }

    pub fn run(self) -> Result<(), Error> {
        match self {
            Command::Version => {
                Command::clap()
                    .write_version(&mut std::io::stdout())
                    .expect("failed to write to stdout");
                Ok(())
            }
            Command::Completions { shell } => {
                let app_name = Command::app_name();
                Command::clap().gen_completions_to(app_name, shell, &mut std::io::stdout());
                Ok(())
            }
            Command::MultiServer { config_file } => {
                let config = match config_file {
                    Some(path) => match Config::load(&path) {
                        Ok(config) => config,
                        Err(source) => {
                            let file_path = path.to_owned();
                            return Err(Error::ReadConfigFile { source, file_path });
                        }
                    },
                    None => Config::default(),
                };
                execute(move |resolver| Box::pin(multi_server::run(resolver, config)))
            }
            Command::SocksServer(options) => {
                let options = options.try_into()?;
                execute(move |resolver| Box::pin(socks_server::run(resolver, options)))
            }
            Command::HttpServer(options) => {
                let options = options.into();
                execute(move |resolver| Box::pin(http_server::run(resolver, options)))
            }
            Command::ProxyChain(_options) => Ok(()),
            Command::ProxyChecker(_options) => {
                execute(move |_resolver| Box::pin(proxy_checker::run()))
            }
        }
    }
}

#[inline]
fn execute<F>(f: F) -> Result<(), Error>
where
    F: FnOnce(Arc<dyn Resolver>) -> Pin<Box<dyn Future<Output = Result<(), Error>>>>,
{
    use tokio::runtime;

    use crate::consts;
    let mut runtime = runtime::Builder::new()
        .thread_name(consts::THREAD_NAME)
        .threaded_scheduler()
        .enable_all()
        .build()
        .map_err(|source| Error::InitializeTokioRuntime { source })?;

    let resolver = {
        let handle = runtime.handle().clone();
        runtime.block_on(async move { DefaultResolver::from_system_conf(handle).await })?
    };

    runtime.block_on(f(Arc::new(resolver)))
}
