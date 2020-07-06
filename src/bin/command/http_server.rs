use std::{
    net::{IpAddr, Ipv4Addr},
    path::Path,
    sync::Arc,
};

use structopt::StructOpt;
use tokio::sync::Mutex;

use tunelo::{
    authentication::AuthenticationManager,
    filter::DefaultFilter,
    server::http::{self, Server, ServerOptions},
    transport::{Resolver, Transport},
};

use crate::{error::Error, shutdown, signal_handler};

pub async fn run<P: AsRef<Path>>(
    resolver: Arc<dyn Resolver>,
    opts: Options,
    config_file: Option<P>,
) -> Result<(), Error> {
    let config = match config_file {
        Some(path) => Config::load(path)?.merge(opts),
        None => Config::default().merge(opts),
    };

    let server_config: ServerOptions = config.into();

    let http_server = {
        let filter = {
            let mut f = DefaultFilter::deny_list();
            f.add_socket(server_config.listen_socket());
            Arc::new(f)
        };
        let transport = Arc::new(Transport::direct(resolver, filter));
        let authentication_manager = Arc::new(Mutex::new(AuthenticationManager::new()));
        Server::new(server_config, transport, authentication_manager)
    };

    let (tx, mut rx) = shutdown::new();
    signal_handler::start(Box::new(|| tx.shutdown()));

    http_server
        .serve_with_shutdown(async move {
            rx.wait().await;
        })
        .await
        .map_err(|source| Error::RunHttpServer { source })?;

    Ok(())
}

#[derive(Debug, StructOpt, Serialize, Deserialize)]
pub struct Options {
    #[structopt(long = "ip", help = "IP address to listen")]
    ip: Option<IpAddr>,

    #[structopt(long = "port", help = "Port number to listen")]
    port: Option<u16>,
}

#[derive(Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Config {
    ip: IpAddr,
    port: u16,
}

impl Default for Config {
    fn default() -> Config { Config { ip: IpAddr::V4(Ipv4Addr::LOCALHOST), port: 8118 } }
}

impl Config {
    impl_config_load!(Config);

    pub fn merge(mut self, opts: Options) -> Config {
        let Options { mut ip, mut port } = opts;

        merge_option_field!(self, ip);
        merge_option_field!(self, port);

        self
    }
}

impl Into<http::ServerOptions> for Config {
    fn into(self) -> http::ServerOptions {
        let listen_address = self.ip;
        let listen_port = self.port;

        http::ServerOptions { listen_address, listen_port }
    }
}
