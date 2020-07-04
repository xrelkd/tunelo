use std::net::IpAddr;

use structopt::StructOpt;

use tunelo::server::http;

#[derive(Debug, StructOpt)]
pub struct HttpOptions {
    #[structopt(long = "ip", default_value = "127.0.0.1", help = "IP address to listen")]
    ip: IpAddr,

    #[structopt(long = "port", default_value = "8118", help = "Port number to listen")]
    port: u16,
}

impl Into<http::ServerOptions> for HttpOptions {
    fn into(self) -> http::ServerOptions {
        let listen_address = self.ip;
        let listen_port = self.port;

        http::ServerOptions { listen_address, listen_port }
    }
}
