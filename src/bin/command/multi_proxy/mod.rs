use std::{future::Future, path::Path, pin::Pin, sync::Arc};

use futures::future::join_all;
use snafu::ResultExt;
use tokio::sync::Mutex;

use tunelo::{
    authentication::AuthenticationManager,
    filter::SimpleFilter,
    server::{http, socks},
    transport::{Resolver, Transport},
};

use crate::{error, error::Error, shutdown, signal_handler};

mod config;

pub use self::config::Config;

pub async fn run<P: AsRef<Path>>(
    resolver: Arc<dyn Resolver>,
    config_file: Option<P>,
) -> Result<(), Error> {
    let config = match config_file {
        Some(path) => Config::load(&path)?,
        None => Config::default(),
    };

    let socks_server_config =
        if config.enable_socks() { config.socks_server.clone() } else { None };
    let http_server_config = if config.enable_http() { config.http_server.clone() } else { None };

    let authentication_manager = Arc::new(Mutex::new(AuthenticationManager::new()));
    let filter = {
        let mut f = SimpleFilter::deny_list();
        if let Some(config) = socks_server_config.as_ref() {
            f.add_socket(config.listen_socket())
        }
        if let Some(config) = http_server_config.as_ref() {
            f.add_socket(config.listen_socket())
        }
        Arc::new(f)
    };

    let transport = Arc::new(Transport::direct(resolver, filter));

    let (shutdown_sender, mut shutdown_receiver) = shutdown::new();

    type ServeFuture = Pin<Box<dyn Future<Output = Result<(), Error>>>>;
    let mut futs: Vec<ServeFuture> = Vec::new();

    if let Some(config) = socks_server_config {
        let socks_serve = {
            let mut shutdown_receiver = shutdown_sender.subscribe();
            let server = socks::Server::new(
                config.into(),
                transport.clone(),
                authentication_manager.clone(),
            );

            let signal = async move {
                shutdown_receiver.wait().await;
            };
            Box::pin(async {
                server.serve_with_shutdown(signal).await.context(error::RunSocksServer)
            })
        };

        futs.push(socks_serve);
    }

    if let Some(config) = http_server_config {
        let http_serve = {
            let server = http::Server::new(config.into(), transport, authentication_manager);

            let signal = async move {
                shutdown_receiver.wait().await;
            };
            Box::pin(async {
                server.serve_with_shutdown(signal).await.context(error::RunHttpServer)
            })
        };

        futs.push(http_serve);
    }

    if futs.is_empty() {
        return Err(Error::NoProxyServer);
    }

    signal_handler::start(Box::new(move || {
        shutdown_sender.shutdown();
    }));

    let handle = join_all(futs).await;
    let errors: Vec<_> = handle.into_iter().filter_map(Result::err).collect();
    if !errors.is_empty() {
        return Err(Error::Collection { errors });
    }

    Ok(())
}
