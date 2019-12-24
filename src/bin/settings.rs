#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocksServer {
    tcp_ip: String,
    tcp_port: u16,

    udp_ip: u16,
    udp_ports: Vec<u16>,

    enable_socks4a: bool,
    enable_socks5: bool,

    socks4_server_config: Option<Socks4Server>,
    socks5_server_config: Option<Socks5Server>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Socks4Server {
    enable_tcp_connect: bool,
    enable_tcp_bind: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Socks5Server {
    enable_tcp_connect: bool,
    enable_tcp_bind: bool,
    enable_udp_associate: bool,

    authentication_method: String,
    authentication_list_file_path: Option<String>,
}
