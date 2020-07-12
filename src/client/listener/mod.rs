mod socks;

pub use self::socks::Socks5Listener;

pub struct ProxyListener {}

impl Default for ProxyListener {
    fn default() -> ProxyListener { ProxyListener {} }
}

impl ProxyListener {
    pub fn new() -> ProxyListener { ProxyListener {} }
}
