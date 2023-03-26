mod host_address;
mod proxy;
pub mod utils;

pub use self::{
    host_address::{HostAddress, HostAddressError},
    proxy::{ProxyHost, ProxyHostError, ProxyStrategy},
};
