use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    runtime::Runtime,
};

use tunelo::{
    client::ProxyStream,
    common::{HostAddress, ProxyHost},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let proxy_host = ProxyHost::Socks5 {
        host: "127.96.0.3".to_owned(),
        port: 3128,
        username: None,
        password: None,
    };
    let remote_addr = HostAddress::DomainName("ifconfig.me".to_owned(), 80);

    let mut stream = {
        let s = ProxyStream::connect_with_proxy(&proxy_host, &remote_addr).await?;
        s.into_inner()
    };

    let request = "GET /ip HTTP/1.0\r\nHost: ifconfig.me\r\n\r\n";
    stream.write(request.as_bytes()).await?;
    let mut response = String::new();
    stream.read_to_string(&mut response).await?;
    println!("{}", response);
    let _ = stream.shutdown(std::net::Shutdown::Write)?;

    Ok(())
}
