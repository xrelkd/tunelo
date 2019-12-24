use std::net::{Ipv4Addr, SocketAddrV4};

use tokio::runtime;

use tunelo::{client::ProxyChecker, common::ProxyHost};

fn main() {
    simple_logger::init_with_level(log::Level::Info).unwrap();
    let mut runtime = runtime::Builder::new()
        .thread_name("proxy-checker")
        .threaded_scheduler()
        .enable_all()
        .build()
        .unwrap();

    let proxy_servers = {
        let local_host = Ipv4Addr::new(127, 0, 0, 1);
        vec![
            SocketAddrV4::new(local_host.clone(), 9050).into(),
            SocketAddrV4::new(local_host.clone(), 9051).into(),
            SocketAddrV4::new(local_host.clone(), 9052).into(),
            SocketAddrV4::new(local_host.clone(), 9053).into(),
            SocketAddrV4::new(local_host.clone(), 9054).into(),
            SocketAddrV4::new(local_host.clone(), 9055).into(),
            SocketAddrV4::new(local_host.clone(), 9056).into(),
            SocketAddrV4::new(local_host.clone(), 9057).into(),
            SocketAddrV4::new(local_host.clone(), 9058).into(),
            SocketAddrV4::new(local_host.clone(), 9059).into(),
            SocketAddrV4::new(local_host.clone(), 9060).into(),
            SocketAddrV4::new(local_host.clone(), 3128).into(),
        ]
        .into_iter()
        .map(|server| ProxyHost::Socks5 { server, user_name: None, password: None })
        .collect()
    };

    let target_hosts = vec![];
    let checker = ProxyChecker::with_parallel(6, proxy_servers, target_hosts);

    runtime.block_on(async move {
        let report = checker.run().await;
        println!("{:?}", report);
    });
}
