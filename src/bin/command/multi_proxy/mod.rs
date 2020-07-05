use std::{future::Future, path::Path, pin::Pin, sync::Arc};

use futures::future::join_all;
use tokio::sync::Mutex;

use tunelo::{
    authentication::AuthenticationManager,
    filter::DefaultFilter,
    server::{http, socks},
    transport::{Resolver, Transport},
};

use crate::{shutdown, signal_handler};

mod config;
mod error;

pub use self::{config::Config, error::Error};

pub async fn run<P: AsRef<Path>>(
    resolver: Arc<dyn Resolver>,
    config_file: Option<P>,
) -> Result<(), crate::error::Error> {
    let config = match config_file {
        Some(path) => Config::load(&path)?,
        None => Config::default(),
    };

    let socks_server_config =
        if config.enable_socks() { config.socks_server.clone() } else { None };
    let http_server_config = if config.enable_http() { config.http_server.clone() } else { None };

    let authentication_manager = Arc::new(Mutex::new(AuthenticationManager::new()));
    let filter = {
        let mut f = DefaultFilter::deny_list();
        socks_server_config.as_ref().map(|config| f.add_socket(config.listen_socket()));
        http_server_config.as_ref().map(|config| f.add_socket(config.listen_socket()));
        Arc::new(f)
    };

    let transport = Arc::new(Transport::direct(resolver, filter));

    let (shutdown_sender, mut shutdown_receiver) = shutdown::new();

    let mut futs: Vec<Pin<Box<dyn Future<Output = Result<(), Error>>>>> = Vec::new();

    if let Some(config) = socks_server_config {
        let socks_serve = {
            let mut shutdown_receiver = shutdown_sender.subscribe();
            let server = socks::Server::new(
                config.into(),
                transport.clone(),
                authentication_manager.clone(),
            );

            let signal = async move {
                let _ = shutdown_receiver.wait().await;
            };
            Box::pin(async {
                Ok(server
                    .serve_with_shutdown(signal)
                    .await
                    .map_err(|source| Error::RunSocksServer { source })?)
            })
        };
        futs.push(socks_serve);
    }

    if let Some(config) = http_server_config {
        let http_serve = {
            let server = http::Server::new(config.into(), transport, authentication_manager);

            let signal = async move {
                let _ = shutdown_receiver.wait().await;
            };
            Box::pin(async {
                Ok(server
                    .serve_with_shutdown(signal)
                    .await
                    .map_err(|source| Error::RunHttpServer { source })?)
            })
        };

        futs.push(http_serve);
    }

    signal_handler::start(Box::new(move || {
        let _ = shutdown_sender.shutdown();
    }));

    let handle = join_all(futs).await;
    let errors: Vec<_> = handle.into_iter().filter_map(Result::err).collect();
    if !errors.is_empty() {
        return Err(Error::ErrorCollection { errors })?;
    }

    Ok(())
}
