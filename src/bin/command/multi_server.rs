use std::sync::Arc;

use tokio::sync::Mutex;

use tunelo::{
    authentication::AuthenticationManager,
    filter::DefaultFilter,
    server::{http, socks},
    transport::{Resolver, Transport},
};

use crate::{config::Config, error::Error, signal_handler};

pub async fn run(resolver: Arc<dyn Resolver>, config: Config) -> Result<(), Error> {
    use futures::future::join_all;
    use std::{future::Future, pin::Pin};
    use tokio::sync::broadcast;

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

    let (shutdown_sender, mut shutdown_receiver) = broadcast::channel(1);

    let mut futs: Vec<Pin<Box<dyn Future<Output = Result<(), Error>>>>> = Vec::new();

    if let Some(config) = socks_server_config {
        let socks_serve = {
            let mut rx = shutdown_sender.subscribe();
            let server = socks::Server::new(
                config.into(),
                transport.clone(),
                authentication_manager.clone(),
            );

            let signal = async move {
                let _ = rx.recv().await;
            };
            Box::pin(async { Ok(server.serve_with_shutdown(signal).await?) })
        };
        futs.push(socks_serve);
    }

    if let Some(config) = http_server_config {
        let http_serve = {
            let server = http::Server::new(config.into(), transport, authentication_manager);

            let signal = async move {
                let _ = shutdown_receiver.recv().await;
            };
            Box::pin(async { Ok(server.serve_with_shutdown(signal).await?) })
        };

        futs.push(http_serve);
    }

    signal_handler::start(Box::new(move || {
        let _ = shutdown_sender.send(());
    }));

    let handle = join_all(futs).await;
    let errors: Vec<_> = handle.into_iter().filter_map(Result::err).collect();
    if !errors.is_empty() {
        return Err(Error::ErrorCollection { errors });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let v = vec![Ok(1), Err(3), Ok(2), Err(4)];
        assert_eq!(v.into_iter().filter_map(Result::err).collect::<Vec<_>>(), vec![3, 4]);
    }
}
