use std::sync::Arc;

use tokio::sync::Mutex;

use tunelo::{
    authentication::AuthenticationManager,
    filter::DefaultFilter,
    server::socks::{Server, ServerOptions},
    transport::{Resolver, Transport},
};

use crate::{command::Error, shutdown, signal_handler};

pub async fn run(resolver: Arc<dyn Resolver>, server_config: ServerOptions) -> Result<(), Error> {
    let socks_server = {
        let filter = {
            let mut f = DefaultFilter::deny_list();
            f.add_socket(server_config.listen_socket());
            Arc::new(f)
        };

        let transport = Arc::new(Transport::direct(resolver, filter));
        let authentication_manager = Arc::new(Mutex::new(AuthenticationManager::new()));
        let server = Server::new(server_config, transport, authentication_manager);
        server
    };

    let (tx, mut rx) = shutdown::shutdown_handle();
    signal_handler::start(Box::new(move || {
        tx.shutdown();
    }));

    socks_server
        .serve_with_shutdown(async move {
            rx.wait().await;
        })
        .await?;

    Ok(())
}
