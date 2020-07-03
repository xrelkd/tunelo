use std::sync::Arc;

use tokio::{runtime, sync::Mutex};

use tunelo::{
    authentication::AuthenticationManager,
    filter::DefaultFilter,
    server::socks::{Server, ServerConfig},
    transport::{DefaultResolver, Transport},
};

use crate::{consts, exit_code, shutdown, signal_handler};

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
        let server = Server::new(server_config, transport, authentication_manager);
        server
    };

    runtime.block_on(async move {
        let (tx, mut rx) = shutdown::shutdown_handle();
        signal_handler::start(Box::new(move || {
            tx.shutdown();
        }));

        match socks_server
            .serve_with_shutdown(async move {
                rx.wait().await;
            })
            .await
        {
            Ok(_) => exit_code::EXIT_SUCCESS,
            Err(err) => {
                error!("SOCKS server error: {:?}", err);
                exit_code::EXIT_FAILURE
            }
        }
    })
}
