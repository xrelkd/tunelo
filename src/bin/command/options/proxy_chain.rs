use std::{net::IpAddr, path::PathBuf};

use structopt::StructOpt;

use tunelo::common::ProxyHost;

#[derive(Debug, StructOpt)]
pub struct ProxyChainOptions {
    #[structopt(long = "enable-socks4a")]
    pub enable_socks4a: bool,

    #[structopt(long = "enable-socks5")]
    pub enable_socks5: bool,

    #[structopt(long = "enable-http")]
    pub enable_http: bool,

    #[structopt(long = "socks-ip")]
    pub socks_ip: Option<IpAddr>,

    #[structopt(long = "socks-port")]
    pub socks_port: Option<u16>,

    #[structopt(long = "http-ip")]
    pub http_ip: Option<IpAddr>,

    #[structopt(long = "http-port")]
    pub http_port: Option<u16>,

    #[structopt(long = "proxy-chain-file")]
    pub proxy_chain_file: Option<PathBuf>,

    #[structopt(long = "proxy-chain")]
    pub proxy_chain: Option<Vec<ProxyHost>>,
}
