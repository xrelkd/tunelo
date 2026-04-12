use crate::client::Error;

#[derive(Debug)]
pub struct Socks5Listener {}

impl Socks5Listener {
    /// Creates a new `Socks5Listener`.
    ///
    /// # Errors
    ///
    /// This function currently returns always [`Ok`].
    pub const fn new() -> Result<Self, Error> { Ok(Self {}) }
}
