mod socks;

pub use self::socks::Socks5Listener;

pub struct ProxyListener {}

impl ProxyListener {
    pub fn new() -> ProxyListener { ProxyListener {} }
}
