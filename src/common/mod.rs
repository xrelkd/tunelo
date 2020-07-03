mod host_address;
mod proxy;

pub use self::{
    host_address::HostAddress,
    proxy::{ProxyHost, ProxyStrategy},
};
