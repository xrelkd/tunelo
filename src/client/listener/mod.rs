mod socks;

pub use self::socks::Socks5Listener;

#[derive(Default)]
pub struct ProxyListener {}

impl ProxyListener {
    #[must_use]
    pub const fn new() -> Self { Self {} }
}
