use std::{sync::Arc, time::Duration};

use tokio::sync::Mutex;

use lru_time_cache::LruCache;

use crate::{common::HostAddress, service::socks::v5::udp::shutdown};

#[derive(Debug, Clone, Hash, Eq, PartialEq, Ord, PartialOrd)]
struct CacheKey(String);

impl From<HostAddress> for CacheKey {
    fn from(host_addr: HostAddress) -> CacheKey { Self::from(&host_addr) }
}

impl From<&HostAddress> for CacheKey {
    fn from(host_addr: &HostAddress) -> CacheKey {
        match host_addr {
            HostAddress::Socket(socket)
                if socket.ip().is_unspecified() || socket.ip().is_loopback() =>
            {
                CacheKey(socket.port().to_string())
            }
            _ => CacheKey(host_addr.port().to_string()),
        }
    }
}

#[derive(Clone)]
pub struct UdpAssociateCache {
    cache: Arc<Mutex<LruCache<CacheKey, shutdown::ShutdownSignal>>>,
}

impl UdpAssociateCache {
    pub fn new(cache_expiry_duration: Duration) -> UdpAssociateCache {
        let cache = Arc::new(Mutex::new(LruCache::with_expiry_duration(cache_expiry_duration)));
        UdpAssociateCache { cache }
    }

    pub async fn insert(&self, addr: &HostAddress) -> shutdown::ShutdownSlot {
        let (shutdown_signal, shutdown_slot) = shutdown::shutdown_handle();

        {
            let key = addr.into();
            let mut cache = self.cache.lock().await;
            cache.insert(key, shutdown_signal);
        }

        info!("Client {} is inserted into UDP associate", addr.to_string());
        shutdown_slot
    }

    pub async fn contains(&self, addr: &HostAddress) -> bool {
        self.cache.lock().await.contains_key(&addr.into())
    }

    pub async fn remove(&self, addr: &HostAddress) {
        let key = CacheKey::from(addr);
        let mut cache = self.cache.lock().await;
        if cache.remove(&key).is_some() {
            info!("Drop UDP association {}", addr.to_string());
        }
    }

    pub async fn clear(&mut self) { self.cache.lock().await.clear(); }

    pub async fn remove_stalled(&self) { self.cache.lock().await.iter(); }
}
