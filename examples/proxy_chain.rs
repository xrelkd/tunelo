use std::net::{Ipv4Addr, SocketAddrV4};

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    runtime::Runtime,
};

use tunelo::{
    client::ProxyStream,
    common::{HostAddress, ProxyHost},
};

fn main() {
    let proxy_chain = vec![
        SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 9050).into(),
        SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 9051).into(),
        SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 9052).into(),
        SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 9053).into(),
        SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 3128).into(),
        // SocketAddrV4::new(Ipv4Addr::new(91, 121, 67, 146), 9050).into(),
        // SocketAddrV4::new(Ipv4Addr::new(89, 223, 92, 30), 9049).into(),
        // SocketAddrV4::new(Ipv4Addr::new(169, 62, 192, 70), 9050).into(),
        // SocketAddrV4::new(Ipv4Addr::new(124, 120, 195, 233), 8118).into(),
        // SocketAddrV4::new(Ipv4Addr::new(127,0,0,1), 8080).into(),
    ]
    .into_iter()
    // .map(|server| ProxyHost::Socks4a { server, id: None })
    .map(|server| ProxyHost::Socks5 { host, port, user_name: None, password: None })
    // .map(|server| ProxyHost::HttpTunnel {
    //     server,
    //     user_agent: Some(String::from(
    //         "Mozilla/5.0 (Windows NT 10; Win64; x64; rv:70.0) Gecko/20100101 Firefox/70.0",
    //     )),
    //     user_name: None,
    //     password: None,
    // })
    .collect();

    let target_host = HostAddress::DomainName("ifconfig.me".to_owned(), 80);
    let mut runtime = Runtime::new().unwrap();

    runtime.block_on(async move {
        let mut stream =
            match ProxyStream::connect_with_proxy_chain(proxy_chain, &target_host).await {
                Ok(stream) => stream.into_inner(),
                Err(err) => {
                    eprint!("{:?}", err);
                    return;
                }
            };

        let request = "GET /ip HTTP/1.0\r\nHost: ifconfig.me\r\n\r\n";
        let _ = stream.write(request.as_bytes()).await;
        println!("==========================");
        while let Ok(ch) = stream.read_u8().await {
            print!("{}", ch as char);
        }
        println!("\n==========================");
        let _ = stream.shutdown(std::net::Shutdown::Write);
    });
}
