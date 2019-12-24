use std::sync::Arc;

use tokio::runtime;
use tokio::sync::Mutex;

use tunelo::{
    authentication::AuthenticationManager,
    filter::DefaultFilter,
    lifecycle::LifecycleManager,
    server::socks::{Server, ServerConfig},
    transport::{DefaultResolver, Transport},
};

use crate::consts;
use crate::exit_code;

pub fn run(server_config: ServerConfig) -> i32 {
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

    let socks_server = {
        let filter = {
            let mut f = DefaultFilter::blacklist();
            f.add_socket(server_config.listen_socket());
            Arc::new(f)
        };

        let resolver = match DefaultResolver::from_system_conf(&mut runtime) {
            Ok(r) => Arc::new(r),
            Err(err) => {
                error!("Error: {:?}", err);
                return exit_code::EXIT_FAILURE;
            }
        };

        let transport = Arc::new(Transport::direct(resolver, filter));
        let authentication_manager = Arc::new(Mutex::new(AuthenticationManager::new()));
        let (server, shutdown_signal) =
            Server::new(server_config, transport, authentication_manager);
        let shutdown_hook = Box::new(move || {
            shutdown_signal.shutdown();
        });
        lifecycle_manager.register("SOCKS Server", shutdown_hook);
        server
    };

    runtime.block_on(lifecycle_manager.block_on(async {
        match socks_server.serve().await {
            Ok(_) => exit_code::EXIT_SUCCESS,
            Err(err) => {
                error!("SOCKS server error: {:?}", err);
                exit_code::EXIT_FAILURE
            }
        }
    }))
}
