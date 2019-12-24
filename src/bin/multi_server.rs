use std::sync::Arc;

use tokio::runtime;
use tokio::sync::Mutex;

use tunelo::{
    authentication::AuthenticationManager,
    filter::DefaultFilter,
    lifecycle::LifecycleManager,
    server::http::{Server as HttpServer, ServerConfig as HttpServerConfig},
    server::socks::{Server as SocksServer, ServerConfig as SocksServerConfig},
    transport::{DefaultResolver, Transport},
};

use crate::consts;
use crate::exit_code;

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

    let (mut lifecycle_manager, _shutdown_signal) = LifecycleManager::new();

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

    let socks_serve = {
        let (server, shutdown_signal) = SocksServer::new(
            socks_server_config,
            transport.clone(),
            authentication_manager.clone(),
        );
        let shutdown_hook = Box::new(move || {
            shutdown_signal.shutdown();
        });
        lifecycle_manager.register("SOCKS Server", shutdown_hook);
        server.serve()
    };

    let http_serve = {
        let (server, shutdown_signal) =
            HttpServer::new(http_server_config, transport, authentication_manager);
        let shutdown_hook = Box::new(move || {
            shutdown_signal.shutdown();
        });
        lifecycle_manager.register("HTTP Server", shutdown_hook);
        server.serve()
    };

    runtime.block_on(lifecycle_manager.block_on(async {
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
    }))
}
