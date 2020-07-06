use std::path::Path;

use structopt::StructOpt;

use tunelo::{client::ProxyChecker, common::ProxyHost};

use crate::error::Error;

pub async fn run<P: AsRef<Path>>(_config: Config, _config_file: Option<P>) -> Result<(), Error> {
    let proxy_servers = vec![
        ("127.0.0.1", 9050),
        ("127.0.0.1", 9051),
        ("127.0.0.1", 9052),
        ("127.0.0.1", 9053),
        ("127.0.0.1", 9054),
        ("127.0.0.1", 9055),
        ("127.0.0.1", 9056),
        ("127.0.0.1", 9057),
        ("127.0.0.1", 9058),
        ("127.0.0.1", 9059),
        ("127.0.0.1", 9060),
        ("127.0.0.1", 3128),
    ]
    .into_iter()
    .map(|(host, port)| ProxyHost::Socks5 {
        host: host.to_owned(),
        port,
        username: None,
        password: None,
    })
    .collect();

    let target_hosts = vec![];
    let checker = ProxyChecker::with_parallel(6, proxy_servers, target_hosts);

    let report = checker.run().await;
    println!("{:?}", report);

    Ok(())
}

#[derive(Debug, StructOpt)]
pub struct Config {}

#[cfg(test)]
mod tests {}
