use tunelo::{client::ProxyChecker, common::ProxyHost};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    simple_logger::init_with_level(log::Level::Info).unwrap();

    let proxy_servers = {
        let localhost = "127.0.0.1";
        vec![
            (localhost.to_owned(), 3128),
            (localhost.to_owned(), 9050),
            (localhost.to_owned(), 9051),
            (localhost.to_owned(), 9052),
            (localhost.to_owned(), 9053),
            (localhost.to_owned(), 9054),
            (localhost.to_owned(), 9055),
            (localhost.to_owned(), 9056),
            (localhost.to_owned(), 9057),
            (localhost.to_owned(), 9058),
            (localhost.to_owned(), 9059),
            (localhost.to_owned(), 9060),
            (localhost.to_owned(), 9061),
            (localhost.to_owned(), 9062),
        ]
        .into_iter()
        .map(|(host, port)| ProxyHost::Socks5 { host, port, username: None, password: None })
        .collect()
    };

    let target_hosts = vec![];
    let checker = ProxyChecker::with_parallel(6, proxy_servers, target_hosts);

    let report = checker.run().await;
    println!("{:?}", report);

    Ok(())
}
