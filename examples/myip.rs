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
    let proxy_host = ProxyHost::Socks5 {
        server: HostAddress::from(SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 3128)),
        user_name: None,
        password: None,
    };
    let remote_addr = HostAddress::DomainName("ifconfig.me".to_owned(), 80);

    let mut runtime = Runtime::new().unwrap();

    runtime.block_on(async move {
        let mut stream = match ProxyStream::connect_with_proxy(&proxy_host, &remote_addr).await {
            Ok(s) => s.into_inner(),
            Err(err) => {
                eprint!("{:?}", err);
                return;
            }
        };

        let request = "GET /ip HTTP/1.0\r\nHost: ifconfig.me\r\n\r\n";
        let _ = stream.write(request.as_bytes()).await;
        let mut response = String::new();
        let _ = stream.read_to_string(&mut response).await;
        println!("{}", response);
        let _ = stream.shutdown(std::net::Shutdown::Write);
    });
}
