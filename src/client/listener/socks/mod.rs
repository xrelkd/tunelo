use crate::client::Error;

#[derive(Debug)]
pub struct Socks5Listener {}

impl Socks5Listener {
    pub fn new() -> Result<Self, Error> { Ok(Self {}) }
}
