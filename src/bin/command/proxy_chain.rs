use std::{collections::HashSet, future::Future, pin::Pin, sync::Arc, time::Duration};

use futures::future::join_all;
use tokio::sync::{broadcast, Mutex};

use tunelo::{
    authentication::AuthenticationManager,
    common::{ProxyHost, ProxyStrategy},
    filter::DefaultFilter,
    server::{http, socks},
    transport::{Resolver, Transport},
};

use crate::{
    command::{
        options::{OptionsError, ProxyChainOptions},
        Error,
    },
    shutdown, signal_handler,
};

pub async fn run(resolver: Arc<dyn Resolver>, options: ProxyChainOptions) -> Result<(), Error> {
    let socks_opts = if options.enable_socks4a || options.enable_socks5 {
        use tunelo::protocol::socks::{SocksCommand, SocksVersion};

        let supported_versions = {
            let mut v = HashSet::new();
            if options.enable_socks4a {
                v.insert(SocksVersion::V4);
            }
            if options.enable_socks5 {
                v.insert(SocksVersion::V5);
            }
            v
        };

        let supported_commands = vec![SocksCommand::TcpConnect].into_iter().collect();

        let listen_address = options.socks_ip.ok_or(OptionsError::NoSocksListenAddress)?;
        let listen_port = options.socks_port.ok_or(OptionsError::NoSocksListenPort)?;

        Some(socks::ServerOptions {
            supported_versions,
            supported_commands,
            listen_address,
            listen_port,
            udp_ports: HashSet::new(),
            connection_timeout: Duration::from_secs(10),
            tcp_keepalive: Duration::from_secs(10),
            udp_cache_expiry_duration: Duration::from_secs(10),
        })
    } else {
        None
    };

    let http_opts = if options.enable_http {
        let listen_address = options.http_ip.ok_or(OptionsError::NoHttpListenAddress)?;
        let listen_port = options.http_port.ok_or(OptionsError::NoHttpListenPort)?;
        Some(http::ServerOptions { listen_address, listen_port })
    } else {
        None
    };

    let proxy_strategy = {
        let chain = match (options.proxy_chain, options.proxy_chain_file) {
            (Some(chain), _) => chain,
            (_, Some(file)) => {
                ProxyHost::load(file).map_err(|source| OptionsError::LoadProxyChain { source })?
            }
            (None, None) => return Err(OptionsError::NoProxyChain)?,
        };

        let chain = ProxyStrategy::Chained(chain);
        info!("Proxy chain: {}", chain);
        Arc::new(chain)
    };

    let filter = {
        let mut f = DefaultFilter::deny_list();
        if let Some(ref opts) = socks_opts {
            f.add_socket(opts.listen_socket());
        }
        if let Some(ref opts) = http_opts {
            f.add_socket(opts.listen_socket());
        }
        Arc::new(f)
    };

    let transport = Arc::new(Transport::proxy(resolver, filter, proxy_strategy)?);
    let authentication_manager = Arc::new(Mutex::new(AuthenticationManager::new()));

    let (shutdown_sender, mut shutdown_receiver) = broadcast::channel(1);

    let mut futs: Vec<Pin<Box<dyn Future<Output = Result<(), Error>>>>> = Vec::new();

    if let Some(opts) = socks_opts {
        let socks_serve = {
            let mut shutdown_receiver = shutdown_sender.subscribe();
            let server =
                socks::Server::new(opts, transport.clone(), authentication_manager.clone());

            let signal = async move {
                let _ = shutdown_receiver.recv().await;
            };
            Box::pin(async { Ok(server.serve_with_shutdown(signal).await?) })
        };
        futs.push(socks_serve);
    }

    if let Some(opts) = http_opts {
        let http_serve = {
            let server = http::Server::new(opts, transport, authentication_manager);

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
