use std::net::{Ipv4Addr, SocketAddrV4};

use tokio::runtime;

use tunelo::{client::ProxyChecker, common::ProxyHost};

use crate::{consts, exit_code};

pub fn run() -> i32 {
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

    let proxy_servers = vec![
        SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 9050).into(),
        SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 9051).into(),
        SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 9052).into(),
        SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 9053).into(),
        SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 9054).into(),
        SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 9055).into(),
        SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 9056).into(),
        SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 9057).into(),
        SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 9058).into(),
        SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 9059).into(),
        SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 9060).into(),
        SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 3128).into(),
    ]
    .into_iter()
    .map(|server| ProxyHost::Socks5 { server, user_name: None, password: None })
    .collect();

    let target_hosts = vec![];
    let checker = ProxyChecker::with_parallel(6, proxy_servers, target_hosts);

    runtime.block_on(async move {
        let report = checker.run().await;
        println!("{:?}", report);
    });

    exit_code::EXIT_SUCCESS
}

#[cfg(test)]
mod tests {}
