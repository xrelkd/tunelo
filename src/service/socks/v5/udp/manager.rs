use std::collections::HashSet;
use std::net::{IpAddr, SocketAddr};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::sync::mpsc;
use tokio::time;

use futures::{stream::FuturesUnordered, FutureExt, StreamExt};

use crate::common::HostAddress;
use crate::protocol::socks::{v5::Reply, Address, Error};
use crate::service::socks::v5::udp::{UdpAssociateCache, UdpServer};
use crate::shutdown;
use crate::transport::Resolver;

pub struct Manager<TransportStream> {
    resolver: Arc<dyn Resolver>,
    cache: UdpAssociateCache,
    cache_expiry_duration: Duration,

    server_addr: IpAddr,
    ports: HashSet<u16>,

    current_server_addr_index: AtomicUsize,
    server_addrs: Vec<SocketAddr>,

    _phantom: std::marker::PhantomData<TransportStream>,
}

impl<TransportStream> Manager<TransportStream>
where
    TransportStream: 'static + Send + Sync + Unpin + AsyncRead + AsyncWrite,
{
    pub fn new(
        server_addr: IpAddr,
        ports: HashSet<u16>,
        resolver: Arc<dyn Resolver>,
        cache_expiry_duration: Duration,
    ) -> Manager<TransportStream> {
        let cache = UdpAssociateCache::new(cache_expiry_duration);
        let server_addrs = Vec::new();
        let current_server_addr_index = AtomicUsize::new(0);

        Manager {
            resolver,
            cache,
            cache_expiry_duration,
            server_addr,
            current_server_addr_index,
            server_addrs,
            ports,
            _phantom: Default::default(),
        }
    }

    pub fn serve(self) -> (mpsc::Sender<(TransportStream, HostAddress)>, shutdown::JoinHandle<()>) {
        let (stream_sender, stream_acceptor) = mpsc::channel(128);
        let (shutdown_signal, shutdown_slot) = shutdown::shutdown_handle();
        let join_handle = tokio::spawn(async move {
            let _ = self.serve_internal(stream_acceptor, shutdown_slot).await;
        });

        (stream_sender, shutdown::JoinHandle::new(shutdown_signal, join_handle))
    }

    async fn serve_internal(
        mut self,
        mut stream_acceptor: mpsc::Receiver<(TransportStream, HostAddress)>,
        mut shutdown_slot: shutdown::ShutdownSlot,
    ) -> Result<(), Error> {
        info!("Start UDP associate manager");

        let server_handles = FuturesUnordered::new();
        let mut server_shutdown_signals = vec![];
        for port in &self.ports {
            let socket_addr = SocketAddr::new(self.server_addr, *port);
            let (server, shutdown_signal) =
                UdpServer::new(socket_addr.clone(), self.cache.clone(), self.resolver.clone());
            self.server_addrs.push(socket_addr);
            server_shutdown_signals.push(shutdown_signal);
            server_handles.push(tokio::spawn({
                async move {
                    let _ = server.serve().await;
                }
            }));
        }

        // remove expired UDP associate
        let mut interval = time::interval(self.cache_expiry_duration);

        loop {
            let (mut stream, cache_key) = futures::select! {
                _ = shutdown_slot.wait().fuse() => break,
                _ = interval.tick().fuse() => {
                    debug!("Remove expired UDP associate");
                    self.cache.remove_stalled().await;
                    continue;
                }
                rx = stream_acceptor.recv().fuse() => {
                    match rx {
                        Some((stream, target_addr)) => (stream,  target_addr),
                        None => break,
                    }
                },
            };

            tokio::spawn({
                let proxy_addr = match self.pick_server() {
                    Some(proxy_addr) => proxy_addr,
                    None => return Ok(()),
                };

                let cache = self.cache.clone();
                let mut shutdown_slot = cache.insert(&cache_key).await;

                let reply = Reply::success(Address::from(proxy_addr));
                let _ = stream.write(&reply.into_bytes()).await?;

                async move {
                    let mut buf = [0u8; 1];
                    loop {
                        let result = futures::select! {
                            _ = shutdown_slot.wait().fuse() => break,
                            res = stream.read(&mut buf).fuse() => res,
                        };

                        match result {
                            Ok(0) => break,
                            Ok(_n) => continue,
                            Err(_err) => break,
                        }
                    }

                    cache.remove(&cache_key).await;
                    let _ = stream.shutdown().await;
                }
            });
        }

        info!("Stop receiving UDP associate request");

        server_shutdown_signals.into_iter().for_each(shutdown::ShutdownSignal::shutdown);
        let _ = server_handles.into_future().await;

        info!("All UDP servers are stopped");

        self.cache.clear().await;

        info!("UDP associate manager is stopped");
        Ok(())
    }

    #[inline]
    fn pick_server(&self) -> Option<SocketAddr> {
        match self.server_addrs.len() {
            0 => None,
            server_count => {
                let next = self.current_server_addr_index.fetch_add(1, Ordering::SeqCst);
                let index = next % server_count;
                Some(self.server_addrs[index])
            }
        }
    }
}
