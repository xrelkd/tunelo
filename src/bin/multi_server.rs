use std::sync::Arc;

use tokio::runtime;
use tokio::sync::Mutex;

use tunelo::{
    authentication::AuthenticationManager,
    filter::DefaultFilter,
    server::http::{Server as HttpServer, ServerConfig as HttpServerConfig},
    server::socks::{Server as SocksServer, ServerConfig as SocksServerConfig},
    transport::{DefaultResolver, Transport},
};

use crate::consts;
use crate::exit_code;
use crate::shutdown;
use crate::signal_handler;

pub fn run(socks_server_config: SocksServerConfig, http_server_config: HttpServerConfig) -> i32 {
    let mut runtime = match runtime::Builder::new()
        .thread_name(consts::THREAD_NAME)
        .threaded_scheduler()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(err) => {
            error!("Error: {:?}", err);
            return exit_code::EXIT_FAILURE;
        }
    };

    let authentication_manager = Arc::new(Mutex::new(AuthenticationManager::new()));
    let filter = {
        let mut f = DefaultFilter::blacklist();
        f.add_socket(socks_server_config.listen_socket());
        f.add_socket(http_server_config.listen_socket());
        Arc::new(f)
    };

    let resolver = match DefaultResolver::from_system_conf(&mut runtime) {
        Ok(r) => (Arc::new(r)),
        Err(err) => {
            error!("Error: {:?}", err);
            return exit_code::EXIT_FAILURE;
        }
    };

    let transport = Arc::new(Transport::direct(resolver.clone(), filter));
    use tokio::sync::broadcast;
    let (shutdown_sender, mut shutdown_receiver) = broadcast::channel(1);

    let socks_serve = {
        let mut rx = shutdown_sender.subscribe();
        let server = SocksServer::new(
            socks_server_config,
            transport.clone(),
            authentication_manager.clone(),
        );

        server.serve_with_shutdown(async move {
            let _ = rx.recv().await;
        })
    };

    let http_serve = {
        let server = HttpServer::new(http_server_config, transport, authentication_manager);

        server.serve_with_shutdown(async move {
            let _ = shutdown_receiver.recv().await;
        })
    };

    runtime.block_on(async {
        signal_handler::start(Box::new(move || {
            let _ = shutdown_sender.send(());
        }));

        let handle = futures::join!(socks_serve, http_serve);
        match handle {
            (Ok(_), Ok(_)) => exit_code::EXIT_SUCCESS,
            (Ok(_), Err(http_err)) => {
                error!("HTTP server error: {:?}", http_err);
                exit_code::EXIT_FAILURE
            }
            (Err(socks_err), Ok(_)) => {
                error!("SOCKS server error: {:?}", socks_err);
                exit_code::EXIT_FAILURE
            }
            (Err(socks_err), Err(http_err)) => {
                error!("SOCKS server error: {:?}", socks_err);
                error!("HTTP server error: {:?}", http_err);
                exit_code::EXIT_FAILURE
            }
        }
    })
}
