#[macro_use]
extern crate log;

use std::{sync::Arc, time::Duration};

use tokio::{runtime::Runtime, sync::Mutex};

use tunelo::{
    authentication::AuthenticationManager,
    filter::DefaultFilter,
    lifecycle::LifecycleManager,
    protocol::socks::{SocksCommand, SocksVersion},
    server::socks::{Server, ServerConfig},
    transport::{DefaultResolver, Transport},
};

fn main() {
    simple_logger::init_with_level(log::Level::Info).unwrap();

    let listen_address = "127.0.0.1".parse().unwrap();
    let listen_port = 9050;
    let server_config = ServerConfig {
        listen_address,
        listen_port,
        udp_cache_expiry_duration: Duration::from_secs(5),
        udp_ports: [29348, 35922, 44023, 57296, 63523].iter().cloned().collect(),
        supported_commands: [
            SocksCommand::TcpConnect,
            SocksCommand::TcpBind,
            SocksCommand::UdpAssociate,
        ]
        .iter()
        .cloned()
        .collect(),
        supported_versions: [SocksVersion::V4, SocksVersion::V5].iter().cloned().collect(),
        connection_timeout: Duration::from_secs(20),
        tcp_keepalive: Duration::from_secs(32),
    };

    let mut runtime = Runtime::new().unwrap();

    let (mut lifecycle_manager, _shutdown_signal) = LifecycleManager::new();
    let socks_server = {
        use std::net::{Ipv4Addr, SocketAddrV4};

        use tunelo::common::{ProxyHost, ProxyStrategy};

        let proxy_chain: Vec<ProxyHost> = vec![
            // SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 9050).into(),
            SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 9051).into(),
            SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 9052).into(),
            SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 9053).into(),
            SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 3128).into(),
            /* SocketAddrV4::new(Ipv4Addr::new(91, 121, 67, 146), 9050).into(),
             * SocketAddrV4::new(Ipv4Addr::new(89, 223, 92, 30), 9049).into(),
             * SocketAddrV4::new(Ipv4Addr::new(169, 62, 192, 70), 9050).into(),
             * SocketAddrV4::new(Ipv4Addr::new(124, 120, 195, 233), 8118).into(),
             * SocketAddrV4::new(Ipv4Addr::new(127,0,0,1), 8080).into(), */
        ]
        .into_iter()
        .map(|server| ProxyHost::Socks5 { server, user_name: None, password: None })
        .collect();

        let resolver = match DefaultResolver::from_system_conf(&mut runtime) {
            Ok(r) => Arc::new(r),
            Err(err) => {
                eprintln!("{:?}", err);
                return;
            }
        };

        let filter = {
            let mut f = DefaultFilter::default();
            f.add_socket(server_config.listen_socket());
            Arc::new(f)
        };

        let proxy_strategy = Arc::new(ProxyStrategy::Chained(proxy_chain));
        // Arc::new(Transport::direct(resolver, filter))
        let transport = match Transport::proxy(resolver.clone(), filter, proxy_strategy) {
            Ok(t) => Arc::new(t),
            Err(err) => {
                eprintln!("{:?}", err);
                return;
            }
        };

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
            Ok(_) => return 0,
            Err(err) => {
                error!("SOCKS server error: {:?}", err);
                return 1;
            }
        }
    }));
}
