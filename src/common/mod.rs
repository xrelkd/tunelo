mod host_address;
mod proxy;

pub use self::{
    host_address::{HostAddress, HostAddressError},
    proxy::{ProxyHost, ProxyHostError, ProxyStrategy},
};

pub mod utils {
    use std::time::Duration;

    #[inline]
    pub fn safe_duration(d: Duration) -> Option<Duration> {
        if d == Duration::from_secs(0) {
            None
        } else {
            Some(d)
        }
    }
}

#[cfg(test)]
mod tests {}
